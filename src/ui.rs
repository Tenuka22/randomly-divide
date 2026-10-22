use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use crate::app::{App, Page, StatusKind};
use crate::pages::{add_person, entry, groups, settings, stats};

pub fn draw(frame: &mut Frame, app: &App) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .areas(frame.area());

    draw_header(frame, header, app);

    match app.page {
        Page::Entry => entry::render(frame, body, app),
        Page::Groups => groups::render(frame, body, app),
        Page::Stats => stats::render(frame, body, app),
        Page::Settings => settings::render(frame, body, &app.settings),
    }

    draw_footer(frame, footer, app);

    if app.add_person.open {
        add_person::render(frame, frame.area(), &app.add_person);
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let titles = ["Entry  [F1]", "Groups [F2]", "Stats  [F3]", "Settings [F4]"];
    let selected = match app.page {
        Page::Entry => 0,
        Page::Groups => 1,
        Page::Stats => 2,
        Page::Settings => 3,
    };
    let tabs = Tabs::new(titles.to_vec())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Randomly Divide ")
                .title_style(Style::new().bold()),
        )
        .style(Style::new().fg(Color::Gray))
        .highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan).bold())
        .select(selected)
        .divider(" ");
    frame.render_widget(tabs, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let hint = match app.page {
        Page::Entry => "a add person · q quit",
        Page::Groups => {
            "j/k half 1-5|6-10 · 1-5 pick group · Shift+1-9 pick member · o/p top/bottom · d/Del remove · q quit"
        }
        Page::Stats => "F1/F2/F3/F4 switch page · q quit",
        Page::Settings => "Tab switch field · type to edit · Ctrl+S save & connect · Ctrl+R rotate secret · q quit",
    };

    let line = if let Some(status) = &app.status {
        let color = match status.kind {
            StatusKind::Success => Color::Green,
            StatusKind::Error => Color::Red,
        };
        Line::from(Span::styled(
            status.text.clone(),
            Style::new().fg(color).add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(Span::styled(hint, Style::new().fg(Color::DarkGray)))
    };

    let footer = Paragraph::new(line).block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, area);
}
