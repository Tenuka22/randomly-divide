use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::person::Person;

/// How many member rows are visible at once in the detail list — this is
/// also the span that `shift+1..9` addresses.
pub const PAGE_SIZE: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Half {
    /// Groups 1-5
    First,
    /// Groups 6-10
    Second,
}

#[derive(Debug)]
pub struct GroupsState {
    pub half: Half,
    pub active_group: Option<u8>,
    pub scroll: usize,
    pub selected: Option<usize>,
}

impl Default for GroupsState {
    fn default() -> Self {
        Self {
            half: Half::First,
            active_group: None,
            scroll: 0,
            selected: None,
        }
    }
}

pub enum GroupsOutcome {
    None,
    Remove(u64),
}

/// Map the symbol a shifted digit produces on a standard US layout
/// (`shift+1` => `!`, …, `shift+9` => `(`) back to `1..=9`. Terminals that
/// report the modifier directly are handled separately in `handle_key`.
fn shifted_digit(c: char) -> Option<u8> {
    "!@#$%^&*(".find(c).map(|i| (i + 1) as u8)
}

pub fn group_members(people: &[Person], group: u8) -> Vec<&Person> {
    people.iter().filter(|p| p.group == group).collect()
}

impl GroupsState {
    pub fn handle_key(&mut self, key: KeyEvent, people: &[Person]) -> GroupsOutcome {
        match key.code {
            KeyCode::Char('j') => self.half = Half::First,
            KeyCode::Char('k') => self.half = Half::Second,
            // Group pick: 1..5 within whichever half is active. The CONTROL
            // modifier isn't required (many terminals swallow ctrl+digit),
            // but is honoured transparently when a terminal does send it.
            KeyCode::Char(c @ '1'..='5') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                let n = c.to_digit(10).unwrap() as u8;
                let group = match self.half {
                    Half::First => n,
                    Half::Second => n + 5,
                };
                self.select_group(group);
            }
            // Member pick: shift+1..9, selecting row `n` of the visible page.
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' && key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.select_member(c.to_digit(10).unwrap() as usize, people);
            }
            KeyCode::Char(c) if shifted_digit(c).is_some() => {
                self.select_member(shifted_digit(c).unwrap() as usize, people);
            }
            KeyCode::Char('o') => self.scroll_top(),
            KeyCode::Char('p') => self.scroll_bottom(people),
            KeyCode::Up => {
                if self.scroll > 0 {
                    self.scroll -= 1;
                }
            }
            KeyCode::Down => self.scroll_down_one(people),
            KeyCode::Char('d') | KeyCode::Delete | KeyCode::Backspace => {
                if let (Some(group), Some(row)) = (self.active_group, self.selected) {
                    let idx = self.scroll + row;
                    if let Some(person) = group_members(people, group).get(idx) {
                        let id = person.id;
                        self.selected = None;
                        return GroupsOutcome::Remove(id);
                    }
                }
            }
            _ => {}
        }
        GroupsOutcome::None
    }

    fn select_group(&mut self, group: u8) {
        self.active_group = Some(group);
        self.scroll = 0;
        self.selected = None;
    }

    fn select_member(&mut self, row_one_based: usize, people: &[Person]) {
        let Some(group) = self.active_group else {
            return;
        };
        let len = group_members(people, group).len();
        let idx = row_one_based - 1;
        if self.scroll + idx < len {
            self.selected = Some(idx);
        }
    }

    fn scroll_top(&mut self) {
        self.scroll = 0;
        self.selected = None;
    }

    fn scroll_bottom(&mut self, people: &[Person]) {
        let Some(group) = self.active_group else {
            return;
        };
        let len = group_members(people, group).len();
        self.scroll = len.saturating_sub(PAGE_SIZE);
        self.selected = None;
    }

    fn scroll_down_one(&mut self, people: &[Person]) {
        let Some(group) = self.active_group else {
            return;
        };
        let len = group_members(people, group).len();
        if self.scroll + PAGE_SIZE < len {
            self.scroll += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyEventState;

    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn plain(code: KeyCode) -> KeyEvent {
        key(code, KeyModifiers::NONE)
    }

    fn people_in_groups(counts: &[(u8, usize)]) -> Vec<Person> {
        let mut id = 1;
        let mut people = Vec::new();
        for &(group, n) in counts {
            for i in 0..n {
                people.push(Person {
                    id,
                    name: format!("P{id}-{i}"),
                    class: "8-B".to_string(),
                    school: "Southlands".to_string(),
                    gender: crate::person::Gender::Male,
                    group,
                    avatar_seed: id,
                });
                id += 1;
            }
        }
        people
    }

    #[test]
    fn half_toggle_and_group_pick_map_to_correct_group() {
        let people = people_in_groups(&[]);
        let mut s = GroupsState::default();
        s.handle_key(plain(KeyCode::Char('3')), &people);
        assert_eq!(s.active_group, Some(3)); // default half = First

        s.handle_key(plain(KeyCode::Char('k')), &people); // Second half
        s.handle_key(plain(KeyCode::Char('3')), &people);
        assert_eq!(s.active_group, Some(8)); // 3 + 5

        s.handle_key(plain(KeyCode::Char('j')), &people); // back to First
        s.handle_key(plain(KeyCode::Char('1')), &people);
        assert_eq!(s.active_group, Some(1));
    }

    #[test]
    fn shifted_digit_selects_member_row_within_bounds() {
        let people = people_in_groups(&[(1, 3)]);
        let mut s = GroupsState::default();
        s.handle_key(plain(KeyCode::Char('1')), &people); // select group 1

        // shift+2 => '@' on a US layout => row index 1 (0-based)
        s.handle_key(plain(KeyCode::Char('@')), &people);
        assert_eq!(s.selected, Some(1));

        // shift+9 => '(' but group only has 3 members: out of range, ignored
        s.handle_key(plain(KeyCode::Char('(')), &people);
        assert_eq!(s.selected, Some(1));
    }

    #[test]
    fn remove_targets_the_exact_selected_person_not_just_first() {
        let people = people_in_groups(&[(1, 3)]);
        let mut s = GroupsState::default();
        s.handle_key(plain(KeyCode::Char('1')), &people);
        s.handle_key(plain(KeyCode::Char('@')), &people); // select row 1 (0-based) => 2nd member, id=2

        let outcome = s.handle_key(plain(KeyCode::Char('d')), &people);
        match outcome {
            GroupsOutcome::Remove(id) => assert_eq!(id, 2),
            GroupsOutcome::None => panic!("expected a removal outcome"),
        }
    }

    #[test]
    fn scroll_top_and_bottom_jump_to_window_edges() {
        let people = people_in_groups(&[(1, 20)]);
        let mut s = GroupsState::default();
        s.handle_key(plain(KeyCode::Char('1')), &people);

        s.handle_key(plain(KeyCode::Char('p')), &people); // bottom
        assert_eq!(s.scroll, 20 - PAGE_SIZE);

        s.handle_key(plain(KeyCode::Char('o')), &people); // top
        assert_eq!(s.scroll, 0);
    }

    #[test]
    fn ctrl_digit_is_also_honoured_when_a_terminal_reports_it() {
        let people = people_in_groups(&[]);
        let mut s = GroupsState::default();
        s.handle_key(key(KeyCode::Char('4'), KeyModifiers::CONTROL), &people);
        assert_eq!(s.active_group, Some(4));
    }
}

pub fn render(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &crate::app::App) {
    use ratatui::layout::{Constraint, Layout};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

    let people = &app.people;
    let state = &app.groups;

    let [grid_area, detail_area] =
        Layout::vertical([Constraint::Length(8), Constraint::Min(0)]).areas(area);
    let [row1, row2] = Layout::vertical([Constraint::Length(4), Constraint::Length(4)]).areas(grid_area);
    let cols = [Constraint::Ratio(1, 5); 5];
    let row1_cells: [_; 5] = Layout::horizontal(cols).areas(row1);
    let row2_cells: [_; 5] = Layout::horizontal(cols).areas(row2);

    for (i, cell) in row1_cells.into_iter().chain(row2_cells).enumerate() {
        let group = (i + 1) as u8;
        let in_half = match state.half {
            Half::First => group <= 5,
            Half::Second => group > 5,
        };
        let is_active = state.active_group == Some(group);
        let count = group_members(people, group).len();

        let border_style = if is_active {
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if in_half {
            Style::new().fg(Color::White)
        } else {
            Style::new().fg(Color::DarkGray)
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {group} "));
        let text = Paragraph::new(Line::from(Span::styled(
            format!("{count} people"),
            if is_active {
                Style::new().fg(Color::Cyan).bold()
            } else {
                Style::new()
            },
        )))
        .block(block);
        frame.render_widget(text, cell);
    }

    let title = match state.active_group {
        Some(g) => format!(" Group {g} members (Shift+1-9 select · d/Del remove · o/p top/bottom) "),
        None => " Pick a group: j/k choose half, then 1-5 ".to_string(),
    };
    let block = Block::default().borders(Borders::ALL).title(title);

    let Some(group) = state.active_group else {
        frame.render_widget(block, detail_area);
        return;
    };

    let members = group_members(people, group);
    let window_end = (state.scroll + PAGE_SIZE).min(members.len());
    let visible = &members[state.scroll..window_end];

    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(row, person)| {
            let gender_color = match person.gender {
                crate::person::Gender::Male => Color::Blue,
                crate::person::Gender::Female => Color::Magenta,
            };
            let line = Line::from(vec![
                Span::styled(format!("{:>2}. ", row + 1), Style::new().fg(Color::DarkGray)),
                Span::styled("██ ", Style::new().fg(crate::avatar::color(person.avatar_seed))),
                Span::styled(format!("{} ", person.gender.short()), Style::new().fg(gender_color).bold()),
                Span::styled(person.name.clone(), Style::new().bold()),
                Span::raw("  "),
                Span::styled(format!("Class {}", person.class), Style::new().fg(Color::DarkGray)),
                Span::raw("  "),
                Span::styled(person.school.clone(), Style::new().fg(Color::DarkGray)),
            ]);
            if state.selected == Some(row) {
                ListItem::new(line).style(Style::new().bg(Color::Cyan).fg(Color::Black))
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let more_above = state.scroll > 0;
    let more_below = window_end < members.len();
    let scroll_hint = match (more_above, more_below) {
        (true, true) => " ▲ more above / ▼ more below ",
        (true, false) => " ▲ more above ",
        (false, true) => " ▼ more below ",
        (false, false) => "",
    };
    let title = format!(
        " Group {group} — {} members{scroll_hint} ",
        members.len()
    );
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(list, detail_area);
}
