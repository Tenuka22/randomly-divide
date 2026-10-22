use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use libsql::Connection;
use ratatui::DefaultTerminal;

use crate::db::{self, ChangeEvent, DbConfig};
use crate::pages::add_person::{AddPersonOutcome, AddPersonState};
use crate::pages::groups::{GroupsOutcome, GroupsState};
use crate::pages::settings::{ConnStatus, SettingsOutcome, SettingsState};
use crate::person::Person;
use crate::random::random_group;
use crate::{store, ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Entry,
    Groups,
    Stats,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Success,
    Error,
}

#[derive(Debug)]
pub struct Status {
    pub text: String,
    pub kind: StatusKind,
}

/// Everything the main loop can react to: keyboard input from the terminal
/// thread, and database sync results from the background tokio task.
pub enum AppEvent {
    Key(KeyEvent),
    /// Bracketed-paste text, delivered as one chunk instead of a flood of
    /// synthetic keystrokes (which would otherwise let pasted text trigger
    /// keybinds like `q` and quit the app mid-paste).
    Paste(String),
    Redraw,
    DbStatus(ConnStatus),
    /// A freshly opened connection, handed back so writes can be mirrored.
    DbConnected(Connection),
    /// Full-table refresh (used when CDC isn't available on this database).
    DbSnapshot(Vec<Person>),
    /// Incremental changes decoded from `turso_cdc`.
    DbChanges(Vec<ChangeEvent>),
}

pub struct App {
    pub page: Page,
    pub people: Vec<Person>,
    pub csv_path: PathBuf,
    pub exit: bool,
    pub status: Option<Status>,
    pub add_person: AddPersonState,
    pub groups: GroupsState,
    pub settings: SettingsState,

    db_conn: Option<Connection>,
    rt_handle: tokio::runtime::Handle,
    event_tx: mpsc::Sender<AppEvent>,
    /// When the first `q` of a double-tap-to-quit was pressed, if any.
    quit_armed_at: Option<Instant>,
}

impl App {
    pub fn new(
        csv_path: PathBuf,
        rt_handle: tokio::runtime::Handle,
        event_tx: mpsc::Sender<AppEvent>,
        db_config: DbConfig,
    ) -> Result<Self> {
        let people = store::load(&csv_path)?;
        let settings = SettingsState::from_config(&db_config);
        let app = Self {
            page: Page::Entry,
            people,
            csv_path,
            exit: false,
            status: None,
            add_person: AddPersonState::default(),
            groups: GroupsState::default(),
            settings,
            db_conn: None,
            rt_handle,
            event_tx,
            quit_armed_at: None,
        };
        if db_config.is_usable() {
            app.spawn_connect(db_config);
        }
        Ok(app)
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal, rx: &mpsc::Receiver<AppEvent>) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| ui::draw(frame, self))?;
            match rx.recv() {
                Ok(event) => self.handle_event(event)?,
                Err(_) => self.exit = true, // all senders dropped
            }
        }
        Ok(())
    }

    fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Key(key) if key.kind == KeyEventKind::Press => self.handle_key(key)?,
            AppEvent::Key(_) | AppEvent::Redraw => {}
            AppEvent::Paste(text) => self.handle_paste(&text),
            AppEvent::DbStatus(status) => self.settings.status = status,
            AppEvent::DbConnected(conn) => self.db_conn = Some(conn),
            AppEvent::DbSnapshot(people) => self.apply_snapshot(people)?,
            AppEvent::DbChanges(changes) => self.apply_changes(changes)?,
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        // The Add Person modal captures all input while open, including
        // letters that would otherwise be page shortcuts.
        if self.add_person.open {
            match self.add_person.handle_key(key) {
                AddPersonOutcome::Submit {
                    name,
                    class,
                    school,
                    gender,
                    avatar_seed,
                } => self.add_person(name, class, school, gender, avatar_seed)?,
                AddPersonOutcome::Cancelled | AddPersonOutcome::None => {}
            }
            return Ok(());
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.exit = true;
            return Ok(());
        }

        // Double-tap `q` to quit. Skipped entirely on Settings, where `q`
        // needs to type into the URL/token fields instead — Ctrl+C is the
        // universal hard-quit from there (and everywhere else).
        let quit_eligible = self.page != Page::Settings;
        let bare_q = key.code == KeyCode::Char('q') && key.modifiers.is_empty();
        if quit_eligible && bare_q {
            let armed_recently = self
                .quit_armed_at
                .is_some_and(|at| at.elapsed() < Duration::from_secs(3));
            if armed_recently {
                self.exit = true;
            } else {
                self.quit_armed_at = Some(Instant::now());
                self.status = Some(Status {
                    text: "Press q again to quit".to_string(),
                    kind: StatusKind::Error,
                });
            }
            return Ok(());
        }
        // Any other key cancels a pending quit confirmation, and clears the
        // "press q again" hint so it doesn't linger looking like it's still
        // armed.
        if self.quit_armed_at.take().is_some() {
            self.status = None;
        }

        match key.code {
            KeyCode::F(1) => {
                self.page = Page::Entry;
                self.status = None;
                return Ok(());
            }
            KeyCode::F(2) => {
                self.page = Page::Groups;
                self.status = None;
                return Ok(());
            }
            KeyCode::F(3) => {
                self.page = Page::Stats;
                self.status = None;
                return Ok(());
            }
            KeyCode::F(4) => {
                self.page = Page::Settings;
                self.status = None;
                return Ok(());
            }
            KeyCode::Char('a') if self.page == Page::Entry => {
                self.add_person.open();
                return Ok(());
            }
            _ => {}
        }

        match self.page {
            Page::Entry => {}
            Page::Groups => {
                if let GroupsOutcome::Remove(id) = self.groups.handle_key(key, &self.people) {
                    self.remove_person(id)?;
                }
            }
            Page::Stats => {}
            Page::Settings => {
                if let SettingsOutcome::Save = self.settings.handle_key(key) {
                    let cfg = self.settings.config();
                    db::save_config(&cfg)?;
                    if cfg.is_usable() {
                        self.settings.status = ConnStatus::Connecting;
                        self.spawn_connect(cfg);
                        self.status = Some(Status {
                            text: "Saved .env.production — connecting…".to_string(),
                            kind: StatusKind::Success,
                        });
                    } else {
                        self.db_conn = None;
                        self.settings.status = ConnStatus::Unconfigured;
                        self.status = Some(Status {
                            text: "Saved .env.production (no database configured)".to_string(),
                            kind: StatusKind::Success,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Route a bracketed paste into whatever text field currently has
    /// focus; ignored when there's nowhere sensible to put it (e.g. the
    /// Groups page, or the School/Gender selectors in the modal).
    fn handle_paste(&mut self, text: &str) {
        if self.add_person.open {
            self.add_person.paste(text);
        } else if self.page == Page::Settings {
            self.settings.paste(text);
        }
    }

    fn add_person(
        &mut self,
        name: String,
        class: String,
        school: String,
        gender: crate::person::Gender,
        avatar_seed: u64,
    ) -> Result<()> {
        let id = self.people.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        let group = random_group();
        let person = Person {
            id,
            name: name.clone(),
            class,
            school,
            gender,
            group,
            avatar_seed,
        };
        self.people.push(person.clone());
        store::save(&self.csv_path, &self.people)?;
        self.mirror_upsert(person);
        self.status = Some(Status {
            text: format!("Added {name} to Group {group}"),
            kind: StatusKind::Success,
        });
        Ok(())
    }

    fn remove_person(&mut self, id: u64) -> Result<()> {
        if let Some(pos) = self.people.iter().position(|p| p.id == id) {
            let person = self.people.remove(pos);
            store::save(&self.csv_path, &self.people)?;
            self.mirror_delete(id);
            self.status = Some(Status {
                text: format!("Removed {} from Group {}", person.name, person.group),
                kind: StatusKind::Error,
            });
        }
        Ok(())
    }

    fn apply_snapshot(&mut self, people: Vec<Person>) -> Result<()> {
        self.people = people;
        store::save(&self.csv_path, &self.people)?;
        Ok(())
    }

    fn apply_changes(&mut self, changes: Vec<ChangeEvent>) -> Result<()> {
        if changes.is_empty() {
            return Ok(());
        }
        let mut upserts = 0;
        let mut deletes = 0;
        for change in changes {
            match change {
                ChangeEvent::Upsert(person) => {
                    upserts += 1;
                    match self.people.iter_mut().find(|p| p.id == person.id) {
                        Some(existing) => *existing = person,
                        None => self.people.push(person),
                    }
                }
                ChangeEvent::Delete(id) => {
                    deletes += 1;
                    self.people.retain(|p| p.id != id);
                }
            }
        }
        self.people.sort_by_key(|p| p.id);
        store::save(&self.csv_path, &self.people)?;
        self.status = Some(Status {
            text: format!("Synced from Turso: {upserts} updated, {deletes} removed"),
            kind: StatusKind::Success,
        });
        Ok(())
    }

    /// Fire-and-forget: push a create/update to Turso without blocking the UI.
    fn mirror_upsert(&self, person: Person) {
        let Some(conn) = self.db_conn.clone() else {
            return;
        };
        let tx = self.event_tx.clone();
        self.rt_handle.spawn(async move {
            if let Err(e) = db::upsert_person(&conn, &person).await {
                let _ = tx.send(AppEvent::DbStatus(ConnStatus::Failed(e.to_string())));
            }
        });
    }

    fn mirror_delete(&self, id: u64) {
        let Some(conn) = self.db_conn.clone() else {
            return;
        };
        let tx = self.event_tx.clone();
        self.rt_handle.spawn(async move {
            if let Err(e) = db::delete_person(&conn, id).await {
                let _ = tx.send(AppEvent::DbStatus(ConnStatus::Failed(e.to_string())));
            }
        });
    }

    /// Connect, seed the local roster from the remote table, then start the
    /// background CDC-poll loop (see `db` module docs for why this is a
    /// modest interval rather than true push).
    fn spawn_connect(&self, cfg: DbConfig) {
        let tx = self.event_tx.clone();
        self.rt_handle.spawn(async move {
            let conn = match tokio::time::timeout(Duration::from_secs(10), db::connect(&cfg)).await
            {
                Ok(Ok(conn)) => conn,
                Ok(Err(e)) => {
                    let _ = tx.send(AppEvent::DbStatus(ConnStatus::Failed(e.to_string())));
                    return;
                }
                Err(_) => {
                    let _ = tx.send(AppEvent::DbStatus(ConnStatus::Failed(
                        "Connection timed out after 10s — check the URL".to_string(),
                    )));
                    return;
                }
            };

            match db::fetch_all(&conn).await {
                Ok(people) => {
                    let _ = tx.send(AppEvent::DbSnapshot(people));
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::DbStatus(ConnStatus::Failed(e.to_string())));
                    return;
                }
            }

            let _ = tx.send(AppEvent::DbConnected(conn.clone()));
            let _ = tx.send(AppEvent::DbStatus(ConnStatus::Connected));

            // Start CDC polling from "now" so we don't replay history we
            // just fetched in full above.
            let mut since = current_cdc_cursor(&conn).await;
            let mut last_snapshot_signature = String::new();
            let mut interval = tokio::time::interval(Duration::from_secs(3));
            loop {
                interval.tick().await;
                match db::poll_changes(&conn, since).await {
                    Ok(Some((new_since, changes))) => {
                        since = new_since;
                        if !changes.is_empty() {
                            let _ = tx.send(AppEvent::DbChanges(changes));
                        }
                    }
                    Ok(None) => {
                        // CDC unsupported on this database: fall back to a
                        // coarse full refresh, deduped so an unchanged table
                        // doesn't cause a redraw every tick.
                        if let Ok(people) = db::fetch_all(&conn).await {
                            let signature = format!("{people:?}");
                            if signature != last_snapshot_signature {
                                last_snapshot_signature = signature;
                                let _ = tx.send(AppEvent::DbSnapshot(people));
                            }
                        }
                    }
                    Err(_) => { /* transient network error; retry next tick */ }
                }
            }
        });
    }
}

async fn current_cdc_cursor(conn: &Connection) -> i64 {
    let Ok(mut rows) = conn
        .query("SELECT COALESCE(MAX(change_id), 0) FROM turso_cdc", ())
        .await
    else {
        return 0;
    };
    match rows.next().await {
        Ok(Some(row)) => row.get::<i64>(0).unwrap_or(0),
        _ => 0,
    }
}

/// Spawn the crossterm input thread, forwarding key/resize events into the
/// shared channel. Runs for the lifetime of the process.
pub fn spawn_input_thread(tx: mpsc::Sender<AppEvent>) {
    std::thread::spawn(move || {
        loop {
            match event::read() {
                Ok(Event::Key(key)) => {
                    if tx.send(AppEvent::Key(key)).is_err() {
                        return;
                    }
                }
                Ok(Event::Resize(_, _)) => {
                    if tx.send(AppEvent::Redraw).is_err() {
                        return;
                    }
                }
                Ok(Event::Paste(text)) => {
                    if tx.send(AppEvent::Paste(text)).is_err() {
                        return;
                    }
                }
                Ok(_) => {}
                Err(_) => return,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    fn test_app() -> (App, tokio::runtime::Runtime) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (tx, _rx) = mpsc::channel();
        let csv_path = std::env::temp_dir().join(format!(
            "randomly-divide-render-test-{}.csv",
            crate::avatar::random_seed()
        ));
        let _ = std::fs::remove_file(&csv_path);
        let app = App::new(csv_path, rt.handle().clone(), tx, crate::db::DbConfig::default()).unwrap();
        (app, rt)
    }

    /// Every page (and the Add Person modal on top of Entry) must render
    /// without panicking — this is the cheapest regression net against the
    /// kind of layout/index bug that only shows up when a real Frame is
    /// drawn (e.g. an `areas()` constraint-count mismatch).
    #[test]
    fn every_page_and_the_modal_render_without_panicking() {
        let (mut app, _rt) = test_app();
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        for page in [Page::Entry, Page::Groups, Page::Stats, Page::Settings] {
            app.page = page;
            terminal.draw(|frame| ui::draw(frame, &app)).unwrap();
            if page == Page::Settings {
                let rendered: String = terminal
                    .backend()
                    .buffer()
                    .content()
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect();
                assert!(rendered.contains("TURSO_DATABASE_URL"));
                assert!(rendered.contains("TURSO_AUTH_TOKEN"));
            }
        }

        app.page = Page::Entry;
        app.add_person.open();
        terminal.draw(|frame| ui::draw(frame, &app)).unwrap();

        let _ = std::fs::remove_file(&app.csv_path);
    }

    #[test]
    fn groups_page_with_people_renders_detail_panel() {
        let (mut app, _rt) = test_app();
        app.people.push(Person {
            id: 1,
            name: "Test Person".to_string(),
            class: "8-B".to_string(),
            school: "Southlands".to_string(),
            gender: crate::person::Gender::Male,
            group: 1,
            avatar_seed: 7,
        });
        app.page = Page::Groups;
        app.groups.active_group = Some(1);

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| ui::draw(frame, &app)).unwrap();

        let _ = std::fs::remove_file(&app.csv_path);
    }

    /// Regression test: pasting a clipboard blob that happens to contain
    /// 'q' must never quit the app. Before bracketed-paste routing, pasted
    /// text was delivered as a flood of synthetic keystrokes and any 'q'
    /// in it triggered the global quit handler.
    #[test]
    fn pasting_text_containing_q_does_not_quit_or_leak_into_the_wrong_field() {
        let (mut app, _rt) = test_app();

        // On a page with no text field, paste is simply dropped.
        app.page = Page::Groups;
        app.handle_paste("quit this app please q q q");
        assert!(!app.exit);

        // Inside the Add Person modal, paste lands in the focused field
        // verbatim — not interpreted as keystrokes.
        app.page = Page::Entry;
        app.add_person.open();
        app.handle_paste("Quincy O'Brien");
        assert!(!app.exit);
        assert_eq!(app.add_person.name, "Quincy O'Brien");

        let _ = std::fs::remove_file(&app.csv_path);
    }

    fn key(code: crossterm::event::KeyCode) -> KeyEvent {
        use crossterm::event::{KeyEventState, KeyModifiers};
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn single_q_arms_but_does_not_quit() {
        use crossterm::event::KeyCode;
        let (mut app, _rt) = test_app();
        app.handle_key(key(KeyCode::Char('q'))).unwrap();
        assert!(!app.exit);
        assert!(app.quit_armed_at.is_some());
    }

    #[test]
    fn second_q_within_the_window_quits() {
        use crossterm::event::KeyCode;
        let (mut app, _rt) = test_app();
        app.handle_key(key(KeyCode::Char('q'))).unwrap();
        app.handle_key(key(KeyCode::Char('q'))).unwrap();
        assert!(app.exit);
    }

    #[test]
    fn any_other_key_disarms_the_pending_quit() {
        use crossterm::event::KeyCode;
        let (mut app, _rt) = test_app();
        app.handle_key(key(KeyCode::Char('q'))).unwrap();
        assert!(app.quit_armed_at.is_some());
        app.handle_key(key(KeyCode::F(2))).unwrap();
        assert!(app.quit_armed_at.is_none());
        // A lone follow-up `q` is now a fresh first tap, not a quit.
        app.handle_key(key(KeyCode::Char('q'))).unwrap();
        assert!(!app.exit);
    }

    #[test]
    fn settings_page_types_q_into_the_focused_field_instead_of_quitting() {
        use crossterm::event::KeyCode;
        let (mut app, _rt) = test_app();
        app.page = Page::Settings;
        app.handle_key(key(KeyCode::Char('q'))).unwrap();
        assert!(!app.exit);
        assert_eq!(app.settings.url, "q");
    }
}
