use ratatui::layout::{Constraint, Direction, Layout, Rect};
use unicode_width::UnicodeWidthChar;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, Panel};
use crate::diff::{RowKind, SideBySideDiff, SideBySideRow};
use crate::git::FileStatus;
use crate::tree::VisibleItemKind;

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

    if let Some(ref diff) = app.diff_content {
        // Render the block to get the inner area
        let inner = block.inner(area);
        frame.render_widget(block, area);
        render_side_by_side(frame, diff, &app.diff_scroll, inner);
    } else {
        let paragraph = Paragraph::new("No file selected")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, area);
    }
}

/// Compute the width needed for line number gutters based on max line number.
fn lineno_width(max_lineno: usize) -> usize {
    if max_lineno == 0 {
        1
    } else {
        max_lineno.ilog10() as usize + 1
    }
}

fn render_side_by_side(
    frame: &mut Frame,
    diff: &SideBySideDiff,
    scroll: &crate::scroll::ScrollState,
    area: Rect,
) {
    if area.width < 10 || area.height == 0 {
        return;
    }

    let ln_width = lineno_width(diff.max_lineno);
    // separator is 3 chars: " │ "
    let separator_width: u16 = 3;
    let ln_col_width = ln_width as u16;
    // Each side: lineno + space + content
    // Total: ln + 1 + content_left + sep + ln + 1 + content_right
    let fixed = ln_col_width * 2 + 2 + separator_width;
    let content_width = if area.width > fixed {
        (area.width - fixed) / 2
    } else {
        1
    };

    // Use cursor directly as viewport start (no visible cursor in diff panel)
    let start = scroll.cursor.min(diff.rows.len());
    let end = (start + area.height as usize).min(diff.rows.len());

    for (i, row) in diff.rows[start..end].iter().enumerate() {
        let y = area.y + i as u16;
        if y >= area.y + area.height {
            break;
        }

        let line = match row {
            SideBySideRow::HunkSeparator => {
                render_hunk_separator_line(area.width, ln_width, content_width, separator_width)
            }
            SideBySideRow::Line { left, right } => {
                render_diff_line(left, right, ln_width, content_width)
            }
        };

        frame.render_widget(
            Paragraph::new(line),
            Rect::new(area.x, y, area.width, 1),
        );
    }
}

fn render_hunk_separator_line(
    total_width: u16,
    ln_width: usize,
    content_width: u16,
    separator_width: u16,
) -> Line<'static> {
    let sep_style = Style::default().fg(Color::DarkGray);
    let left_ln = format!("{:>width$}", "~", width = ln_width);
    let left_content = "~".repeat(content_width as usize);
    let right_ln = format!("{:>width$}", "~", width = ln_width);

    // Calculate remaining width for right content
    let used = ln_width as u16 + 1 + content_width + separator_width + ln_width as u16 + 1;
    let right_content_width = if total_width > used {
        (total_width - used) as usize
    } else {
        content_width as usize
    };
    let right_content = "~".repeat(right_content_width);

    Line::from(vec![
        Span::styled(left_ln, sep_style),
        Span::styled(" ", sep_style),
        Span::styled(left_content, sep_style),
        Span::styled(" │ ", sep_style),
        Span::styled(right_ln, sep_style),
        Span::styled(" ", sep_style),
        Span::styled(right_content, sep_style),
    ])
}

fn render_diff_line<'a>(
    left: &Option<crate::diff::SideContent>,
    right: &Option<crate::diff::SideContent>,
    ln_width: usize,
    content_width: u16,
) -> Line<'a> {
    let mut spans = Vec::new();

    // Left side
    render_side(&mut spans, left, ln_width, content_width);

    // Separator
    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));

    // Right side
    render_side(&mut spans, right, ln_width, content_width);

    Line::from(spans)
}

fn render_side(
    spans: &mut Vec<Span<'static>>,
    side: &Option<crate::diff::SideContent>,
    ln_width: usize,
    content_width: u16,
) {
    let ln_style = Style::default().fg(Color::DarkGray);

    match side {
        Some(content) => {
            let (fg, bg) = match content.kind {
                RowKind::Context => (Color::Reset, Color::Reset),
                RowKind::Added => (Color::Reset, Color::Rgb(0, 40, 0)),
                RowKind::Deleted => (Color::Reset, Color::Rgb(40, 0, 0)),
            };
            let content_style = Style::default().fg(fg).bg(bg);

            // Line number
            let ln_text = format!("{:>width$}", content.lineno, width = ln_width);
            spans.push(Span::styled(ln_text, ln_style));
            spans.push(Span::raw(" "));

            // Content, truncated to fit
            let (text, visual_width) = truncate_to_width(&content.content, content_width as usize);
            let padding = content_width as usize - visual_width;
            spans.push(Span::styled(text, content_style));
            if padding > 0 {
                spans.push(Span::styled(" ".repeat(padding), content_style));
            }
        }
        None => {
            // Blank padding
            let width = ln_width + 1 + content_width as usize;
            spans.push(Span::raw(" ".repeat(width)));
        }
    }
}

/// Truncate a string to fit within the given character width.
/// Returns the truncated string and its visual width.
fn truncate_to_width(s: &str, max_width: usize) -> (String, usize) {
    let mut result = String::new();
    let mut width = 0;
    for ch in s.chars() {
        let ch_width = if ch == '\t' { 4 } else { ch.width().unwrap_or(0) };
        if width + ch_width > max_width {
            break;
        }
        if ch == '\t' {
            result.push_str(&" ".repeat(4));
        } else {
            result.push(ch);
        }
        width += ch_width;
    }
    (result, width)
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
            Constraint::Percentage(75),
        ])
        .split(chunks[0]);

    render_file_list(frame, app, main_chunks[0]);
    render_diff(frame, app, main_chunks[1]);

    // Status bar
    let help = match app.active_panel {
        Panel::FileList => " j/k: navigate  J/K: scroll diff  Enter/Tab: diff  q/Esc: quit",
        Panel::Diff => " j/k: scroll  PgUp/PgDn: page  g/G: top/bottom  Tab/Esc: files  q: quit",
    };

    let status = Paragraph::new(Line::from(vec![
        Span::styled(help, Style::default().fg(Color::DarkGray)),
    ]));

    frame.render_widget(status, chunks[1]);
}
