use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, Panel};
use crate::git::FileStatus;
use crate::tree::VisibleItemKind;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(50),
        ])
        .split(frame.area());

    render_commit_list(frame, app, chunks[0]);
    render_file_list(frame, app, chunks[1]);
    render_diff(frame, app, chunks[2]);
}

fn render_commit_list(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.active_panel == Panel::CommitList;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(" Commits ({}) ", app.commits.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let (start, end) = app.commit_scroll.visible_range(app.commits.len());

    let items: Vec<ListItem> = app.commits[start..end]
        .iter()
        .enumerate()
        .map(|(i, commit)| {
            let is_selected = (start + i) == app.commit_scroll.cursor;
            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", commit.short_hash()),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(&commit.subject),
            ]);

            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_file_list(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.active_panel == Panel::FileList;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(" Files ({}) ", app.files.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let (start, end) = app.file_scroll.visible_range(app.visible_items.len());

    let items: Vec<ListItem> = app.visible_items[start..end]
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = (start + i) == app.file_scroll.cursor;
            let indent = "  ".repeat(item.depth);

            let line = match &item.kind {
                VisibleItemKind::Directory { expanded } => {
                    let indicator = if *expanded { "\u{25bc}" } else { "\u{25b6}" };
                    Line::from(vec![
                        Span::raw(format!("{}{} {}/", indent, indicator, item.name)),
                    ])
                }
                VisibleItemKind::File { status, .. } => {
                    let status_color = match status {
                        FileStatus::Added => Color::Green,
                        FileStatus::Modified => Color::Yellow,
                        FileStatus::Deleted => Color::Red,
                    };
                    Line::from(vec![
                        Span::raw(indent),
                        Span::styled(
                            format!("{} ", status),
                            Style::default().fg(status_color),
                        ),
                        Span::raw(&item.name),
                    ])
                }
            };

            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_diff(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.active_panel == Panel::Diff;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if let Some(file) = app.selected_file() {
        format!(" {} ", file.path)
    } else {
        " Diff ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if let Some(ref content) = app.diff_content {
        let paragraph = Paragraph::new(content.clone())
            .block(block)
            .scroll((app.diff_scroll.cursor as u16, 0));
        frame.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new("No file selected")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, area);
    }
}

/// Render a single-line status bar at the bottom.
pub fn render_status_bar(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if area.height < 3 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    // Re-render main content in the top area
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(50),
        ])
        .split(chunks[0]);

    render_commit_list(frame, app, main_chunks[0]);
    render_file_list(frame, app, main_chunks[1]);
    render_diff(frame, app, main_chunks[2]);

    // Status bar
    let help = match app.active_panel {
        Panel::CommitList => " j/k: navigate  Tab: files  g/G: first/last  q: quit",
        Panel::FileList => " j/k: navigate  Tab: diff  Esc: commits  g/G: first/last  q: quit",
        Panel::Diff => " j/k: scroll  PgUp/PgDn: page  g/G: top/bottom  Tab/Esc: back to files",
    };

    let description = app
        .selected_commit()
        .map(|c| format!(" {} {} ", c.short_hash(), c.subject))
        .unwrap_or_else(|| " no commits ".into());

    let status = Paragraph::new(Line::from(vec![
        Span::styled(
            description,
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(help, Style::default().fg(Color::DarkGray)),
    ]));

    frame.render_widget(status, chunks[1]);
}
