//! Active transfers screen

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, TransferDirection, TransferStatus};
use crate::theme::ThemeColors;
use crate::ui::widgets::transfer_item::TransferItem;

pub fn draw(frame: &mut Frame, app: &App, theme: &ThemeColors, area: Rect) {
    // Split into main transfers area and recent completed panel
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(area);

    draw_transfers(frame, app, theme, chunks[0]);
    draw_recent(frame, app, theme, chunks[1]);
}

fn draw_transfers(frame: &mut Frame, app: &App, theme: &ThemeColors, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" Active Transfers ", theme.title()))
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .border_set(symbols::border::ROUNDED)
        .style(theme.panel());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.transfers.is_empty() {
        draw_empty(frame, theme, inner);
        return;
    }

    // Group transfers by direction
    let uploads: Vec<_> = app
        .transfers
        .iter()
        .enumerate()
        .filter(|(_, t)| t.direction == TransferDirection::Upload)
        .collect();
    let downloads: Vec<_> = app
        .transfers
        .iter()
        .enumerate()
        .filter(|(_, t)| t.direction == TransferDirection::Download)
        .collect();

    let mut y = inner.y;

    // Uploads section
    if !uploads.is_empty() {
        let header = Paragraph::new(Line::from(Span::styled(
            format!("↑ SENDING ({})", uploads.len()),
            theme.upload(),
        )));
        frame.render_widget(
            header,
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;

        // Separator
        let sep = Paragraph::new(Line::from(Span::styled(
            "─".repeat(inner.width as usize),
            theme.border_dimmed(),
        )));
        frame.render_widget(
            sep,
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;

        for (idx, transfer) in &uploads {
            if y + 2 > inner.y + inner.height.saturating_sub(2) {
                break;
            }
            let item = TransferItem::new(transfer, theme).selected(*idx == app.transfer_cursor);
            frame.render_widget(
                item,
                Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 2,
                },
            );
            y += 3;
        }
    }

    // Downloads section
    if !downloads.is_empty() {
        if !uploads.is_empty() {
            y += 1; // Extra spacing between sections
        }

        let header = Paragraph::new(Line::from(Span::styled(
            format!("↓ RECEIVING ({})", downloads.len()),
            theme.download(),
        )));
        frame.render_widget(
            header,
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;

        let sep = Paragraph::new(Line::from(Span::styled(
            "─".repeat(inner.width as usize),
            theme.border_dimmed(),
        )));
        frame.render_widget(
            sep,
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;

        for (idx, transfer) in &downloads {
            if y + 2 > inner.y + inner.height.saturating_sub(2) {
                break;
            }
            let item = TransferItem::new(transfer, theme).selected(*idx == app.transfer_cursor);
            frame.render_widget(
                item,
                Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 2,
                },
            );
            y += 3;
        }
    }

    // Summary at bottom
    draw_summary(frame, app, theme, inner);
}

fn draw_empty(frame: &mut Frame, theme: &ThemeColors, area: Rect) {
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled("No active transfers", theme.text_dimmed())),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", theme.text_dimmed()),
            Span::styled("1", theme.key()),
            Span::styled(" to send or ", theme.text_dimmed()),
            Span::styled("2", theme.key()),
            Span::styled(" to receive", theme.text_dimmed()),
        ]),
    ])
    .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(content, area);
}

fn draw_summary(frame: &mut Frame, app: &App, theme: &ThemeColors, area: Rect) {
    let upload_bytes: u64 = app
        .transfers
        .iter()
        .filter(|t| t.direction == TransferDirection::Upload)
        .map(|t| t.total_bytes)
        .sum();
    let download_bytes: u64 = app
        .transfers
        .iter()
        .filter(|t| t.direction == TransferDirection::Download)
        .map(|t| t.total_bytes)
        .sum();

    let active = app
        .transfers
        .iter()
        .filter(|t| t.status == TransferStatus::Active)
        .count();
    let queued = app
        .transfers
        .iter()
        .filter(|t| t.status == TransferStatus::Queued)
        .count();

    let upload_str = humansize::format_size(upload_bytes, humansize::BINARY);
    let download_str = humansize::format_size(download_bytes, humansize::BINARY);

    let summary = Paragraph::new(Line::from(vec![
        Span::styled("Total: ", theme.text_dimmed()),
        Span::styled(format!("↑ {}", upload_str), theme.upload()),
        Span::styled("  ", theme.text_dimmed()),
        Span::styled(format!("↓ {}", download_str), theme.download()),
        Span::styled(
            format!("   Active: {}  Queued: {}", active, queued),
            theme.text_dimmed(),
        ),
    ]));

    let summary_area = Rect {
        y: area.y + area.height.saturating_sub(1),
        height: 1,
        ..area
    };

    frame.render_widget(summary, summary_area);
}

/// Draw the recent completed transfers panel
fn draw_recent(frame: &mut Frame, app: &App, theme: &ThemeColors, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" Recent ", theme.title()))
        .borders(Borders::ALL)
        .border_style(theme.border())
        .border_set(symbols::border::ROUNDED)
        .style(theme.panel());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.session_history.is_empty() {
        let hint = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No completed transfers",
                theme.text_dimmed(),
            )),
            Line::from(""),
            Line::from(Span::styled("  this session", theme.text_dimmed())),
        ]);
        frame.render_widget(hint, inner);
        return;
    }

    let mut y = inner.y;

    // Show recent completed transfers (newest first)
    for transfer in app.session_history.iter().rev() {
        if y >= inner.y + inner.height {
            break;
        }

        let dir_symbol = match transfer.direction {
            TransferDirection::Upload => "^",
            TransferDirection::Download => "v",
        };
        let dir_style = match transfer.direction {
            TransferDirection::Upload => theme.upload(),
            TransferDirection::Download => theme.download(),
        };

        let size = humansize::format_size(transfer.total_bytes, humansize::BINARY);
        let duration = transfer
            .duration_secs
            .map(|d| {
                if d < 1.0 {
                    format!("{:.0}ms", d * 1000.0)
                } else {
                    format!("{:.1}s", d)
                }
            })
            .unwrap_or_default();

        // First line: direction + name
        let line1 = Paragraph::new(Line::from(vec![
            Span::styled(format!(" {} ", dir_symbol), dir_style),
            Span::styled(&transfer.name, theme.text()),
        ]));
        frame.render_widget(
            line1,
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;

        if y >= inner.y + inner.height {
            break;
        }

        // Second line: size + duration
        let line2 = Paragraph::new(Line::from(vec![
            Span::styled(format!("   {} ", size), theme.text_muted()),
            Span::styled(duration, theme.text_dimmed()),
        ]));
        frame.render_widget(
            line2,
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 2; // Extra spacing between items
    }
}
