//! Transfer item display widget

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::app::{Transfer, TransferDirection, TransferStatus};
use crate::theme::ThemeColors;
use crate::transfer::ConflictResolution;

use super::progress::{format_eta, format_speed};

/// Renders a single transfer item (2-3 lines)
pub struct TransferItem<'a> {
    transfer: &'a Transfer,
    theme: &'a ThemeColors,
    selected: bool,
}

impl<'a> TransferItem<'a> {
    pub fn new(transfer: &'a Transfer, theme: &'a ThemeColors) -> Self {
        Self {
            transfer,
            theme,
            selected: false,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl Widget for TransferItem<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 {
            return;
        }

        let t = self.transfer;
        let theme = self.theme;

        // Direction icon and style
        let (dir_icon, dir_style) = match t.direction {
            TransferDirection::Upload => ("↑", theme.upload()),
            TransferDirection::Download => ("↓", theme.download()),
        };

        // Status indicator
        let status_style = match t.status {
            TransferStatus::Preparing => theme.text_dimmed(),
            TransferStatus::Connecting => theme.info(),
            TransferStatus::Active => theme.text(),
            TransferStatus::Paused => theme.warning(),
            TransferStatus::Stalled => theme.warning(),
            TransferStatus::Queued => theme.text_dimmed(),
            TransferStatus::Failed => theme.error(),
            TransferStatus::Complete => theme.success(),
        };

        // Use symbol + label for status display
        let status_symbol = t.status.symbol();
        let status_label = if status_symbol.is_empty() {
            t.status.label().to_string()
        } else {
            format!("{} {}", status_symbol, t.status.label())
        };

        // Selection marker
        let marker = if self.selected { "> " } else { "  " };

        // Line 1: marker, direction, name, progress bar, percent, status
        let progress = t.progress_percent();
        let progress_width = 20.min(area.width.saturating_sub(40) as usize);

        // Build progress bar string
        let filled = (progress_width as f64 * progress / 100.0).round() as usize;
        let empty = progress_width.saturating_sub(filled);
        let bar: String = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

        let line1 = Line::from(vec![
            Span::styled(
                marker,
                if self.selected {
                    theme.selected()
                } else {
                    theme.text()
                },
            ),
            Span::styled(format!("{} ", dir_icon), dir_style),
            Span::styled(
                truncate(&t.name, 20),
                if self.selected {
                    theme.selected()
                } else {
                    theme.text()
                },
            ),
            Span::raw(" "),
            Span::styled(&bar, theme.progress()),
            Span::styled(format!(" {:>3.0}%", progress), theme.progress_text()),
            Span::raw("  "),
            Span::styled(&status_label, status_style),
        ]);

        let para1 = Paragraph::new(line1);
        para1.render(Rect { height: 1, ..area }, buf);

        if area.height < 2 {
            return;
        }

        // Line 2: size info, speed, ETA, connection OR ticket
        let total = humansize::format_size(t.total_bytes, humansize::BINARY);
        let remaining = humansize::format_size(t.remaining_bytes(), humansize::BINARY);
        let speed = format_speed(t.speed_bps);
        let eta = t.eta_seconds().map(format_eta).unwrap_or_default();

        // Build conflict resolution indicator
        let conflict_info = t.conflict_resolution.as_ref().map(|r| {
            let (label, style) = match r {
                ConflictResolution::Rename => ("renamed", theme.info()),
                ConflictResolution::Overwrite => ("overwritten", theme.warning()),
                ConflictResolution::Skip => ("skipped existing", theme.text_dimmed()),
                ConflictResolution::Cancel => ("cancelled", theme.error()),
            };
            (label, style)
        });

        // For uploads with a ticket, show the ticket prominently
        let line2 = if t.direction == TransferDirection::Upload {
            if let Some(ref ticket) = t.ticket {
                // Show ticket (truncated to fit)
                let max_ticket_len = area.width.saturating_sub(10) as usize;
                let display_ticket = if ticket.len() > max_ticket_len {
                    format!("{}…", &ticket[..max_ticket_len.saturating_sub(1)])
                } else {
                    ticket.clone()
                };
                Line::from(vec![
                    Span::styled("    Ticket: ", theme.text_dimmed()),
                    Span::styled(display_ticket, theme.text_highlight()),
                    Span::styled(" (copied)", theme.success()),
                ])
            } else {
                Line::from(Span::styled(
                    "    Generating ticket...",
                    theme.text_dimmed(),
                ))
            }
        } else if t.status == TransferStatus::Active && t.speed_bps > 0 {
            let mut spans = vec![Span::styled(
                format!(
                    "    {} → {} left    {}  ETA {}  {}",
                    total,
                    remaining,
                    speed,
                    eta,
                    t.connection.label()
                ),
                theme.text_muted(),
            )];
            if let Some((label, style)) = conflict_info {
                spans.push(Span::styled(format!("  [{}]", label), style));
            }
            Line::from(spans)
        } else if let Some(ref err) = t.error_message {
            Line::from(Span::styled(format!("    {}", err), theme.error()))
        } else {
            let mut spans = vec![Span::styled(
                format!("    {} → {} left", total, remaining),
                theme.text_muted(),
            )];
            // Show conflict resolution for completed downloads
            if let Some((label, style)) = conflict_info {
                spans.push(Span::styled(format!("  [{}]", label), style));
            }
            Line::from(spans)
        };

        let para2 = Paragraph::new(line2);
        para2.render(
            Rect {
                y: area.y + 1,
                height: 1,
                ..area
            },
            buf,
        );
    }
}

/// Truncate string to max length with ellipsis
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{:<width$}", s, width = max)
    } else {
        format!("{}…", &s[..max - 1])
    }
}
