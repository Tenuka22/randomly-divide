use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::App;
use crate::{avatar, person::Gender};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let [button_area, roster_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);

    let button = Paragraph::new(Line::from(Span::styled(
        "  + Add Person   (a)  ",
        Style::new().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
    )))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(button, button_area);

    let items: Vec<ListItem> = app
        .people
        .iter()
        .rev()
        .map(|p| {
            let gender_color = match p.gender {
                Gender::Male => Color::Blue,
                Gender::Female => Color::Magenta,
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("Group {:>2}", p.group), Style::new().fg(Color::Yellow)),
                Span::raw("  "),
                Span::styled("██", Style::new().fg(avatar::color(p.avatar_seed))),
                Span::raw(" "),
                Span::styled(format!(" {} ", p.gender.short()), Style::new().fg(gender_color).bold()),
                Span::raw(" "),
                Span::styled(p.name.clone(), Style::new().bold()),
                Span::raw("  "),
                Span::styled(format!("Class {}", p.class), Style::new().fg(Color::DarkGray)),
                Span::raw("  "),
                Span::styled(p.school.clone(), Style::new().fg(Color::DarkGray)),
            ]))
        })
        .collect();
    let title = format!(" Roster — {} total ", app.people.len());
    let recent_list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(recent_list, roster_area);
}
