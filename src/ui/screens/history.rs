//! History screen

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, TransferDirection, TransferStatus};
use crate::theme::ThemeColors;
use crate::transfer::ConflictResolution;

pub fn draw(frame: &mut Frame, app: &App, theme: &ThemeColors, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    draw_list(frame, app, theme, chunks[0]);
    draw_details(frame, app, theme, chunks[1]);
}

fn draw_list(frame: &mut Frame, app: &App, theme: &ThemeColors, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" History ", theme.title()))
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .border_set(symbols::border::ROUNDED)
        .style(theme.panel());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.history.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  No transfer history", theme.text_dimmed())),
            Line::from(""),
            Line::from(Span::styled(
                "  Completed transfers appear here",
                theme.text_dimmed(),
            )),
        ]);
        frame.render_widget(empty, inner);
        return;
    }

    let items: Vec<ListItem> = app
        .history
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let is_selected = i == app.history_cursor;

            let status_icon = match t.status {
                TransferStatus::Complete => "✓",
                TransferStatus::Failed => "✗",
                _ => " ",
            };
            let status_style = match t.status {
                TransferStatus::Complete => theme.success(),
                TransferStatus::Failed => theme.error(),
                _ => theme.text_dimmed(),
            };

            let dir_icon = match t.direction {
                TransferDirection::Upload => "↑",
                TransferDirection::Download => "↓",
            };
            let dir_style = match t.direction {
                TransferDirection::Upload => theme.upload(),
                TransferDirection::Download => theme.download(),
            };

            let size = humansize::format_size(t.total_bytes, humansize::BINARY);

            let style = if is_selected {
                theme.selected()
            } else {
                theme.text()
            };

            ListItem::new(Line::from(vec![
                Span::raw(if is_selected { "> " } else { "  " }),
                Span::styled(format!("{} ", status_icon), status_style),
                Span::styled(&t.name, style),
                Span::raw("  "),
                Span::styled(dir_icon, dir_style),
                Span::styled(format!(" {}", size), theme.text_muted()),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

fn draw_details(frame: &mut Frame, app: &App, theme: &ThemeColors, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" Details ", theme.title()))
        .borders(Borders::ALL)
        .border_style(theme.border())
        .border_set(symbols::border::ROUNDED)
        .style(theme.panel());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.history.is_empty() {
        let hint = Paragraph::new(Span::styled("  Select a transfer", theme.text_dimmed()));
        frame.render_widget(hint, inner);
        return;
    }

    let transfer = match app.history.get(app.history_cursor) {
        Some(t) => t,
        None => return,
    };

    let dir_label = match transfer.direction {
        TransferDirection::Upload => "Sent ↑",
        TransferDirection::Download => "Received ↓",
    };

    let status_label = match transfer.status {
        TransferStatus::Complete => "Completed",
        TransferStatus::Failed => "Failed",
        _ => "Unknown",
    };

    let size = humansize::format_size(transfer.total_bytes, humansize::BINARY);

    // Conflict resolution info
    let conflict_info = transfer.conflict_resolution.as_ref().map(|r| match r {
        ConflictResolution::Rename => ("Renamed (added suffix)", theme.info()),
        ConflictResolution::Overwrite => ("Overwrote existing", theme.warning()),
        ConflictResolution::Skip => ("Skipped existing", theme.text_dimmed()),
        ConflictResolution::Cancel => ("Cancelled", theme.error()),
    });

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Name: ", theme.text_dimmed()),
            Span::styled(&transfer.name, theme.text()),
        ]),
        Line::from(vec![
            Span::styled("  Size: ", theme.text_dimmed()),
            Span::styled(&size, theme.text()),
        ]),
        Line::from(vec![
            Span::styled("  Direction: ", theme.text_dimmed()),
            Span::styled(dir_label, theme.text()),
        ]),
        Line::from(vec![
            Span::styled("  Status: ", theme.text_dimmed()),
            Span::styled(status_label, theme.text()),
        ]),
    ];

    // Add duration if available
    if let Some(duration) = transfer.duration_secs {
        let duration_str = if duration < 1.0 {
            format!("{:.0}ms", duration * 1000.0)
        } else if duration < 60.0 {
            format!("{:.1}s", duration)
        } else if duration < 3600.0 {
            let mins = (duration / 60.0).floor();
            let secs = duration % 60.0;
            format!("{:.0}m {:.0}s", mins, secs)
        } else {
            let hours = (duration / 3600.0).floor();
            let mins = ((duration % 3600.0) / 60.0).floor();
            format!("{:.0}h {:.0}m", hours, mins)
        };

        // Calculate average speed
        let avg_speed = if duration > 0.0 {
            transfer.total_bytes as f64 / duration
        } else {
            0.0
        };
        let speed_str = humansize::format_size(avg_speed as u64, humansize::BINARY);

        lines.push(Line::from(vec![
            Span::styled("  Duration: ", theme.text_dimmed()),
            Span::styled(duration_str, theme.text()),
            Span::styled(format!(" ({}/s avg)", speed_str), theme.text_muted()),
        ]));
    }

    // Add conflict resolution line if present
    if let Some((label, style)) = conflict_info {
        lines.push(Line::from(vec![
            Span::styled("  Conflict: ", theme.text_dimmed()),
            Span::styled(label, style),
        ]));
    }

    // Add file list if multiple files
    let total_files = transfer.total_file_count();
    if total_files > 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  Files ({}):", total_files),
            theme.text_dimmed(),
        )));

        let max_display = (inner.height as usize).saturating_sub(lines.len() + 6);
        for file in transfer.files.iter().take(max_display) {
            let size = humansize::format_size(file.size, humansize::BINARY);
            lines.push(Line::from(vec![
                Span::styled("    ", theme.text()),
                Span::styled(&file.name, theme.text()),
                Span::styled(format!(" ({})", size), theme.text_muted()),
            ]));
        }
        let remaining = total_files.saturating_sub(transfer.files.len().min(max_display));
        if remaining > 0 {
            lines.push(Line::from(Span::styled(
                format!("    ... and {} more", remaining),
                theme.text_dimmed(),
            )));
        }
    }

    lines.extend(vec![
        Line::from(""),
        Line::from(vec![Span::styled("  Ticket: ", theme.text_dimmed())]),
        Line::from(vec![Span::styled(
            format!(
                "  {}",
                transfer
                    .ticket
                    .as_ref()
                    .map(|t| truncate(t, inner.width as usize - 4))
                    .unwrap_or_else(|| "N/A".to_string())
            ),
            theme.text_muted(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [r]", theme.key()),
            Span::styled(" Resend  ", theme.text_dimmed()),
            Span::styled("[c]", theme.key()),
            Span::styled(" Copy  ", theme.text_dimmed()),
            Span::styled("[d]", theme.key()),
            Span::styled(" Delete", theme.text_dimmed()),
        ]),
    ]);

    let content = Paragraph::new(lines);

    frame.render_widget(content, inner);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
