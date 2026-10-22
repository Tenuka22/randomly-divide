use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Bar, BarChart, BarGroup, Block, Borders, Paragraph};

use crate::app::App;
use crate::person::{GROUP_COUNT, SCHOOLS};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let [top, bottom] =
        Layout::vertical([Constraint::Percentage(60), Constraint::Percentage(40)]).areas(area);
    render_group_chart(frame, top, app);

    let [schools_area, summary_area] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(bottom);
    render_school_chart(frame, schools_area, app);
    render_summary(frame, summary_area, app);
}

fn render_group_chart(frame: &mut Frame, area: Rect, app: &App) {
    let counts: Vec<u64> = (1..=GROUP_COUNT)
        .map(|g| app.people.iter().filter(|p| p.group == g).count() as u64)
        .collect();
    let max = counts.iter().copied().max().unwrap_or(0).max(1);

    let bars: Vec<Bar> = counts
        .iter()
        .enumerate()
        .map(|(i, &count)| {
            Bar::default()
                .label(Line::from(format!("{}", i + 1)))
                .value(count)
                .text_value(count.to_string())
                .style(Style::new().fg(Color::Cyan))
                .value_style(Style::new().fg(Color::Black).bg(Color::Cyan))
        })
        .collect();

    let chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" People per group "),
        )
        .bar_width(5)
        .bar_gap(2)
        .max(max)
        .data(BarGroup::new(bars));
    frame.render_widget(chart, area);
}

fn render_school_chart(frame: &mut Frame, area: Rect, app: &App) {
    let counts: Vec<u64> = SCHOOLS
        .iter()
        .map(|&school| app.people.iter().filter(|p| p.school == school).count() as u64)
        .collect();
    let max = counts.iter().copied().max().unwrap_or(0).max(1);

    let bars: Vec<Bar> = SCHOOLS
        .iter()
        .zip(counts)
        .map(|(&school, count)| {
            Bar::default()
                .label(Line::from(school))
                .value(count)
                .text_value(count.to_string())
                .style(Style::new().fg(Color::Magenta))
                .value_style(Style::new().fg(Color::Black).bg(Color::Magenta))
        })
        .collect();

    let chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" People per school "),
        )
        .direction(ratatui::layout::Direction::Horizontal)
        .bar_width(1)
        .bar_gap(1)
        .max(max)
        .data(BarGroup::new(bars));
    frame.render_widget(chart, area);
}

fn render_summary(frame: &mut Frame, area: Rect, app: &App) {
    let total = app.people.len();
    let counts: Vec<usize> = (1..=GROUP_COUNT)
        .map(|g| app.people.iter().filter(|p| p.group == g).count())
        .collect();
    let min = counts.iter().copied().min().unwrap_or(0);
    let max = counts.iter().copied().max().unwrap_or(0);
    let avg = if GROUP_COUNT > 0 {
        total as f64 / GROUP_COUNT as f64
    } else {
        0.0
    };
    let filled = counts.iter().filter(|&&c| c > 0).count();

    let lines = vec![
        Line::from(vec![
            Span::raw("Total people:   "),
            Span::styled(total.to_string(), Style::new().bold().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::raw("Groups filled:  "),
            Span::styled(
                format!("{filled}/{GROUP_COUNT}"),
                Style::new().bold().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("Avg per group:  "),
            Span::styled(format!("{avg:.1}"), Style::new().bold().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::raw("Smallest group: "),
            Span::styled(min.to_string(), Style::new().bold().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::raw("Largest group:  "),
            Span::styled(max.to_string(), Style::new().bold().fg(Color::Yellow)),
        ]),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Summary "),
    );
    frame.render_widget(para, area);
}
