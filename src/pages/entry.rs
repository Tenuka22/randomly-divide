use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::person::SCHOOLS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Name,
    School,
}

#[derive(Debug)]
pub struct EntryState {
    pub name: String,
    pub school_idx: usize,
    pub focus: Focus,
}

impl Default for EntryState {
    fn default() -> Self {
        Self {
            name: String::new(),
            school_idx: 0,
            focus: Focus::Name,
        }
    }
}

pub enum EntryOutcome {
    None,
    Submit { name: String, school: String },
}

impl EntryState {
    pub fn handle_key(&mut self, key: KeyEvent) -> EntryOutcome {
        match self.focus {
            Focus::Name => match key.code {
                KeyCode::Tab | KeyCode::Down => self.focus = Focus::School,
                KeyCode::Enter if !self.name.trim().is_empty() => self.focus = Focus::School,
                KeyCode::Backspace => {
                    self.name.pop();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.name.push(c);
                }
                _ => {}
            },
            Focus::School => match key.code {
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Esc => self.focus = Focus::Name,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.school_idx = if self.school_idx == 0 {
                        SCHOOLS.len() - 1
                    } else {
                        self.school_idx - 1
                    };
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.school_idx = (self.school_idx + 1) % SCHOOLS.len();
                }
                KeyCode::Enter => {
                    if !self.name.trim().is_empty() {
                        let name = std::mem::take(&mut self.name).trim().to_string();
                        let school = SCHOOLS[self.school_idx].to_string();
                        self.focus = Focus::Name;
                        return EntryOutcome::Submit { name, school };
                    }
                }
                _ => {}
            },
        }
        EntryOutcome::None
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyEventKind, KeyEventState};

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn type_str(state: &mut EntryState, s: &str) {
        for c in s.chars() {
            state.handle_key(key(KeyCode::Char(c)));
        }
    }

    #[test]
    fn enter_in_name_moves_focus_without_submitting() {
        let mut state = EntryState::default();
        type_str(&mut state, "Amal");
        let outcome = state.handle_key(key(KeyCode::Enter));
        assert!(matches!(outcome, EntryOutcome::None));
        assert_eq!(state.focus, Focus::School);
    }

    #[test]
    fn enter_in_school_submits_trimmed_name_and_resets_form() {
        let mut state = EntryState::default();
        type_str(&mut state, "  Amal  ");
        state.handle_key(key(KeyCode::Enter)); // -> School focus
        state.handle_key(key(KeyCode::Down)); // move to SCHOOLS[1]
        let outcome = state.handle_key(key(KeyCode::Enter));
        match outcome {
            EntryOutcome::Submit { name, school } => {
                assert_eq!(name, "Amal");
                assert_eq!(school, SCHOOLS[1]);
            }
            EntryOutcome::None => panic!("expected a submit outcome"),
        }
        assert_eq!(state.name, "");
        assert_eq!(state.focus, Focus::Name);
    }

    #[test]
    fn school_selection_wraps_in_both_directions() {
        let mut state = EntryState::default();
        state.focus = Focus::School;
        state.handle_key(key(KeyCode::Up)); // wrap to last
        assert_eq!(state.school_idx, SCHOOLS.len() - 1);
        state.handle_key(key(KeyCode::Down)); // wrap back to first
        assert_eq!(state.school_idx, 0);
    }

    #[test]
    fn empty_name_cannot_be_submitted() {
        let mut state = EntryState::default();
        state.focus = Focus::School;
        let outcome = state.handle_key(key(KeyCode::Enter));
        assert!(matches!(outcome, EntryOutcome::None));
    }
}

pub fn render(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &crate::app::App) {
    use ratatui::layout::{Constraint, Layout};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

    let [left, right] = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)])
        .areas(area);

    let [name_area, school_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(left);

    let state = &app.entry;

    let name_focused = state.focus == Focus::Name;
    let name_style = if name_focused {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new()
    };
    let cursor = if name_focused { "█" } else { "" };
    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(name_style)
        .title(" Name ");
    let name_para = Paragraph::new(Line::from(vec![
        Span::raw(state.name.as_str()),
        Span::styled(cursor, Style::new().add_modifier(Modifier::SLOW_BLINK)),
    ]))
    .block(name_block);
    frame.render_widget(name_para, name_area);

    let school_focused = state.focus == Focus::School;
    let school_style = if school_focused {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new()
    };
    let items: Vec<ListItem> = SCHOOLS
        .iter()
        .enumerate()
        .map(|(i, school)| {
            if i == state.school_idx {
                ListItem::new(format!("  {school}")).style(
                    Style::new()
                        .bg(Color::Cyan)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(format!("  {school}"))
            }
        })
        .collect();
    let school_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(school_style)
            .title(" School (↑/↓ or j/k, Enter to add) "),
    );
    frame.render_widget(school_list, school_area);

    let recent: Vec<ListItem> = app
        .people
        .iter()
        .rev()
        .take(right.height.saturating_sub(2) as usize)
        .map(|p| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("Group {:>2}", p.group), Style::new().fg(Color::Yellow)),
                Span::raw("  "),
                Span::styled(p.name.clone(), Style::new().bold()),
                Span::raw("  "),
                Span::styled(p.school.clone(), Style::new().fg(Color::DarkGray)),
            ]))
        })
        .collect();
    let title = format!(" Roster — {} total ", app.people.len());
    let recent_list = List::new(recent).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(recent_list, right);
}
