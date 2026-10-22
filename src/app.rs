use std::path::PathBuf;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;

use crate::pages::entry::{EntryOutcome, EntryState, Focus};
use crate::pages::groups::{GroupsOutcome, GroupsState};
use crate::person::Person;
use crate::random::random_group;
use crate::{store, ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Entry,
    Groups,
    Stats,
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

pub struct App {
    pub page: Page,
    pub people: Vec<Person>,
    pub csv_path: PathBuf,
    pub exit: bool,
    pub status: Option<Status>,
    pub entry: EntryState,
    pub groups: GroupsState,
}

impl App {
    pub fn new(csv_path: PathBuf) -> Result<Self> {
        let people = store::load(&csv_path)?;
        Ok(Self {
            page: Page::Entry,
            people,
            csv_path,
            exit: false,
            status: None,
            entry: EntryState::default(),
            groups: GroupsState::default(),
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| ui::draw(frame, self))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                self.handle_key(key)?;
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.exit = true;
            return Ok(());
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
            KeyCode::Char('q')
                if !(self.page == Page::Entry && self.entry.focus == Focus::Name) =>
            {
                self.exit = true;
                return Ok(());
            }
            _ => {}
        }

        match self.page {
            Page::Entry => {
                if let EntryOutcome::Submit { name, school } = self.entry.handle_key(key) {
                    self.add_person(name, school)?;
                }
            }
            Page::Groups => {
                if let GroupsOutcome::Remove(id) = self.groups.handle_key(key, &self.people) {
                    self.remove_person(id)?;
                }
            }
            Page::Stats => {}
        }
        Ok(())
    }

    fn add_person(&mut self, name: String, school: String) -> Result<()> {
        let id = self.people.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        let group = random_group();
        self.people.push(Person {
            id,
            name: name.clone(),
            school,
            group,
        });
        store::save(&self.csv_path, &self.people)?;
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
            self.status = Some(Status {
                text: format!("Removed {} from Group {}", person.name, person.group),
                kind: StatusKind::Error,
            });
        }
        Ok(())
    }
}
