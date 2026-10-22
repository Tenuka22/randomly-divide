use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::avatar;
use crate::person::{Gender, SCHOOLS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Name,
    Class,
    School,
    Gender,
}

#[derive(Debug)]
pub struct AddPersonState {
    pub open: bool,
    pub name: String,
    pub class: String,
    pub school_idx: usize,
    pub gender_idx: usize,
    pub avatar_seed: u64,
    pub focus: Focus,
}

impl Default for AddPersonState {
    fn default() -> Self {
        Self {
            open: false,
            name: String::new(),
            class: String::new(),
            school_idx: 0,
            gender_idx: 0,
            avatar_seed: avatar::random_seed(),
            focus: Focus::Name,
        }
    }
}

pub enum AddPersonOutcome {
    None,
    Cancelled,
    Submit {
        name: String,
        class: String,
        school: String,
        gender: Gender,
        avatar_seed: u64,
    },
}

impl AddPersonState {
    /// Open a blank modal with a freshly rolled avatar.
    pub fn open(&mut self) {
        *self = Self {
            open: true,
            ..Self::default()
        };
    }

    fn gender(&self) -> Gender {
        Gender::ALL[self.gender_idx]
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AddPersonOutcome {
        if !self.open {
            return AddPersonOutcome::None;
        }

        // Reroll the avatar without colliding with typed text.
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r') {
            self.avatar_seed = avatar::random_seed();
            return AddPersonOutcome::None;
        }

        if key.code == KeyCode::Esc {
            self.open = false;
            return AddPersonOutcome::Cancelled;
        }

        match self.focus {
            Focus::Name => match key.code {
                KeyCode::Tab | KeyCode::Down => self.focus = Focus::Class,
                KeyCode::Enter if !self.name.trim().is_empty() => self.focus = Focus::Class,
                KeyCode::Backspace => {
                    self.name.pop();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.name.push(c);
                }
                _ => {}
            },
            Focus::Class => match key.code {
                KeyCode::Tab | KeyCode::Down => self.focus = Focus::School,
                KeyCode::BackTab => self.focus = Focus::Name,
                KeyCode::Enter if !self.class.trim().is_empty() => self.focus = Focus::School,
                KeyCode::Backspace => {
                    self.class.pop();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.class.push(c);
                }
                _ => {}
            },
            Focus::School => match key.code {
                KeyCode::Tab => self.focus = Focus::Gender,
                KeyCode::BackTab => self.focus = Focus::Class,
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
                KeyCode::Enter => self.focus = Focus::Gender,
                _ => {}
            },
            Focus::Gender => match key.code {
                KeyCode::BackTab => self.focus = Focus::School,
                KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
                | KeyCode::Char('h') | KeyCode::Char('l')
                | KeyCode::Char('j') | KeyCode::Char('k') => {
                    self.gender_idx = (self.gender_idx + 1) % Gender::ALL.len();
                }
                KeyCode::Tab | KeyCode::Enter => {
                    if self.name.trim().is_empty() || self.class.trim().is_empty() {
                        self.focus = Focus::Name;
                        return AddPersonOutcome::None;
                    }
                    let name = self.name.trim().to_string();
                    let class = self.class.trim().to_string();
                    let school = SCHOOLS[self.school_idx].to_string();
                    let gender = self.gender();
                    let avatar_seed = self.avatar_seed;
                    self.open = false;
                    return AddPersonOutcome::Submit {
                        name,
                        class,
                        school,
                        gender,
                        avatar_seed,
                    };
                }
                _ => {}
            },
        }
        AddPersonOutcome::None
    }
}

pub fn render(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &AddPersonState) {
    use ratatui::layout::{Alignment, Constraint, Layout};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

    // Centered floating "window" over the current page.
    let popup = centered_rect(74, 70, area);
    frame.render_widget(Clear, popup);

    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" Add Person — Esc cancel · Ctrl+R reroll avatar ")
        .border_style(Style::new().fg(Color::Cyan));
    let inner = outer.inner(popup);
    frame.render_widget(outer, popup);

    let [left, right] =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).areas(inner);

    let [name_area, class_area, school_area, gender_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(8),
        Constraint::Length(3),
    ])
    .areas(left);

    let field_style = |focused: bool| {
        if focused {
            Style::new().fg(Color::Cyan)
        } else {
            Style::new()
        }
    };
    let cursor = |focused: bool| if focused { "█" } else { "" };

    let name_focused = state.focus == Focus::Name;
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(state.name.as_str()),
            Span::styled(cursor(name_focused), Style::new().add_modifier(Modifier::SLOW_BLINK)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(field_style(name_focused))
                .title(" Name "),
        ),
        name_area,
    );

    let class_focused = state.focus == Focus::Class;
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(state.class.as_str()),
            Span::styled(cursor(class_focused), Style::new().add_modifier(Modifier::SLOW_BLINK)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(field_style(class_focused))
                .title(" Class (e.g. 8-B) "),
        ),
        class_area,
    );

    let school_focused = state.focus == Focus::School;
    let school_items: Vec<ListItem> = SCHOOLS
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
    frame.render_widget(
        List::new(school_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(field_style(school_focused))
                .title(" School (↑/↓ or j/k) "),
        ),
        school_area,
    );

    let gender_focused = state.focus == Focus::Gender;
    let gender_line = Line::from(
        Gender::ALL
            .iter()
            .flat_map(|&g| {
                let selected = g == Gender::ALL[state.gender_idx];
                let style = if selected {
                    Style::new().bg(Color::Cyan).fg(Color::Black).bold()
                } else {
                    Style::new()
                };
                [Span::styled(format!(" {} ", g.label()), style), Span::raw("  ")]
            })
            .collect::<Vec<_>>(),
    );
    frame.render_widget(
        Paragraph::new(gender_line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(field_style(gender_focused))
                .title(" Gender (←/→ or h/l, Enter submits) "),
        ),
        gender_area,
    );

    render_avatar_preview(frame, right, state.avatar_seed, Gender::ALL[state.gender_idx]);

    let _ = Alignment::Center; // silence unused import if layout trims later
}

/// Render the identicon + gender glyph + hex seed in the given area,
/// centered. Shared with the Groups/Entry detail panels so a person's
/// avatar looks the same everywhere it's shown.
pub fn render_avatar_preview(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    seed: u64,
    gender: Gender,
) {
    use ratatui::layout::Alignment;
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Paragraph};

    let cells = avatar::grid(seed);
    let fg = avatar::color(seed);

    let mut lines: Vec<Line> = cells
        .iter()
        .map(|row| {
            let spans: Vec<Span> = row
                .iter()
                .map(|&filled| {
                    if filled {
                        Span::styled("██", Style::new().fg(fg))
                    } else {
                        Span::raw("  ")
                    }
                })
                .collect();
            Line::from(spans).alignment(Alignment::Center)
        })
        .collect();

    lines.push(Line::raw(""));
    let gender_color = match gender {
        Gender::Male => Color::Blue,
        Gender::Female => Color::Magenta,
    };
    lines.push(
        Line::from(Span::styled(gender.label(), Style::new().fg(gender_color).bold()))
            .alignment(Alignment::Center),
    );
    lines.push(
        Line::from(Span::styled(
            format!("#{seed:016x}"),
            Style::new().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Center),
    );

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Avatar "),
    );
    frame.render_widget(para, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    use ratatui::layout::{Constraint, Flex, Layout};

    let [area] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(area);
    area
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

    fn type_str(state: &mut AddPersonState, s: &str) {
        for c in s.chars() {
            state.handle_key(key(KeyCode::Char(c)));
        }
    }

    #[test]
    fn esc_cancels_without_submitting() {
        let mut state = AddPersonState::default();
        state.open();
        type_str(&mut state, "Amal");
        let outcome = state.handle_key(key(KeyCode::Esc));
        assert!(matches!(outcome, AddPersonOutcome::Cancelled));
        assert!(!state.open);
    }

    #[test]
    fn full_flow_submits_all_fields() {
        let mut state = AddPersonState::default();
        state.open();
        type_str(&mut state, "Amal");
        state.handle_key(key(KeyCode::Enter)); // -> Class
        type_str(&mut state, "8-B");
        state.handle_key(key(KeyCode::Enter)); // -> School
        state.handle_key(key(KeyCode::Down)); // School idx 1
        state.handle_key(key(KeyCode::Enter)); // -> Gender
        state.handle_key(key(KeyCode::Right)); // toggle to Female
        let outcome = state.handle_key(key(KeyCode::Enter)); // submit
        match outcome {
            AddPersonOutcome::Submit {
                name,
                class,
                school,
                gender,
                ..
            } => {
                assert_eq!(name, "Amal");
                assert_eq!(class, "8-B");
                assert_eq!(school, SCHOOLS[1]);
                assert_eq!(gender, Gender::Female);
            }
            _ => panic!("expected submit"),
        }
        assert!(!state.open);
    }

    #[test]
    fn ctrl_r_rerolls_avatar_seed() {
        let mut state = AddPersonState::default();
        state.open();
        let before = state.avatar_seed;
        state.handle_key(KeyEvent {
            code: KeyCode::Char('r'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });
        // Statistically distinct; both draw from a 64-bit CSPRNG.
        assert_ne!(before, state.avatar_seed);
    }
}
