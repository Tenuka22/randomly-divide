use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use crate::app::{App, Page, StatusKind};
use crate::pages::{entry, groups, stats};

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
    }

    draw_footer(frame, footer, app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let titles = ["Entry  [F1]", "Groups [F2]", "Stats  [F3]"];
    let selected = match app.page {
        Page::Entry => 0,
        Page::Groups => 1,
        Page::Stats => 2,
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
        Page::Entry => {
            "Type name · Tab/↓/Enter next field · ↑/↓ or j/k pick school · Enter submit · q quit"
        }
        Page::Groups => {
            "j/k half 1-5|6-10 · 1-5 pick group · Shift+1-9 pick member · o/p top/bottom · d/Del remove · q quit"
        }
        Page::Stats => "F1/F2/F3 switch page · q quit",
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
