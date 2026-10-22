use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::db::DbConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Url,
    Token,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnStatus {
    Unconfigured,
    Connecting,
    Connected,
    Failed(String),
}

#[derive(Debug)]
pub struct SettingsState {
    pub url: String,
    pub token: String,
    pub focus: Focus,
    pub status: ConnStatus,
}

impl SettingsState {
    pub fn from_config(cfg: &DbConfig) -> Self {
        Self {
            url: cfg.url.clone(),
            token: cfg.token.clone(),
            focus: Focus::Url,
            status: if cfg.is_usable() {
                ConnStatus::Connecting
            } else {
                ConnStatus::Unconfigured
            },
        }
    }

    pub fn config(&self) -> DbConfig {
        DbConfig {
            url: self.url.trim().to_string(),
            token: self.token.trim().to_string(),
        }
    }
}

pub enum SettingsOutcome {
    None,
    /// Save (and connect/reconnect) with the current field values.
    Save,
}

impl SettingsState {
    pub fn handle_key(&mut self, key: KeyEvent) -> SettingsOutcome {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') => return SettingsOutcome::Save,
                // Rotate the secret: clear the token and focus it for
                // re-entry (paste the new value issued by Turso), keeping
                // the URL untouched. Nothing is persisted until Ctrl+S.
                KeyCode::Char('r') => {
                    self.token.clear();
                    self.focus = Focus::Token;
                    return SettingsOutcome::None;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Tab | KeyCode::Down => {
                self.focus = match self.focus {
                    Focus::Url => Focus::Token,
                    Focus::Token => Focus::Url,
                };
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.focus = match self.focus {
                    Focus::Url => Focus::Token,
                    Focus::Token => Focus::Url,
                };
            }
            KeyCode::Backspace => {
                self.field_mut().pop();
            }
            KeyCode::Char(c) => {
                self.field_mut().push(c);
            }
            _ => {}
        }
        SettingsOutcome::None
    }

    /// Insert pasted text into whichever field has focus, stripping control
    /// characters.
    pub fn paste(&mut self, text: &str) {
        let clean: String = text.chars().filter(|c| !c.is_control()).collect();
        self.field_mut().push_str(&clean);
    }

    fn field_mut(&mut self) -> &mut String {
        match self.focus {
            Focus::Url => &mut self.url,
            Focus::Token => &mut self.token,
        }
    }
}

pub fn render(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &SettingsState) {
    use ratatui::layout::Constraint;
    use ratatui::layout::Layout;
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Paragraph};

    let [url_area, token_area, status_area, _rest] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .areas(area);

    let field_style = |focused: bool| {
        if focused {
            Style::new().fg(Color::Cyan)
        } else {
            Style::new()
        }
    };

    let url_focused = state.focus == Focus::Url;
    frame.render_widget(
        Paragraph::new(state.url.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(field_style(url_focused))
                .title(" TURSO_DATABASE_URL "),
        ),
        url_area,
    );

    let token_focused = state.focus == Focus::Token;
    let masked: String = "•".repeat(state.token.chars().count());
    frame.render_widget(
        Paragraph::new(masked).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(field_style(token_focused))
                .title(" TURSO_AUTH_TOKEN (Ctrl+R to rotate) "),
        ),
        token_area,
    );

    let (status_text, status_color) = match &state.status {
        ConnStatus::Unconfigured => (
            "Not configured — fill in a URL and token, then Ctrl+S to save & connect.".to_string(),
            Color::DarkGray,
        ),
        ConnStatus::Connecting => ("Connecting…".to_string(), Color::Yellow),
        ConnStatus::Connected => (
            "Connected — writes sync to Turso, changes elsewhere sync back here.".to_string(),
            Color::Green,
        ),
        ConnStatus::Failed(err) => (format!("Connection failed: {err}"), Color::Red),
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(status_text, Style::new().fg(status_color))))
            .block(Block::default().borders(Borders::ALL).title(" Status ")),
        status_area,
    );
}
