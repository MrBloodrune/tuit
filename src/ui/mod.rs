//! UI rendering for Tuit

pub mod layout;
pub mod screens;
pub mod widgets;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
    Frame,
};

use crate::app::{App, ConflictPopup, ConnectionStatus, KeyPresetPopup, Mode, ThemePopup};
use crate::input::KeyPreset;
use crate::theme::{ThemeColors, ThemeKind};

/// Main draw function
pub fn draw(frame: &mut Frame, app: &mut App) {
    let theme = app.theme.colors();
    let size = frame.area();

    // Clear with background
    frame.render_widget(Block::default().style(theme.background()), size);

    // Layout: header, tabs, content, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Length(3), // Tabs
            Constraint::Min(5),    // Content
            Constraint::Length(1), // Footer
        ])
        .split(size);

    draw_header(frame, app, theme, chunks[0]);
    draw_tabs(frame, app, theme, chunks[1]);
    draw_content(frame, app, theme, chunks[2]);
    draw_footer(frame, app, theme, chunks[3]);

    // Help overlay
    if app.show_help {
        draw_help(frame, theme, size);
    }

    // Ticket popup (for SSH where clipboard doesn't work)
    if let Some(ref ticket) = app.show_ticket_popup {
        draw_ticket_popup(frame, theme, ticket, size);
    }

    // Conflict resolution popup
    if let Some(ref popup) = app.conflict_popup {
        draw_conflict_popup(frame, theme, popup, size);
    }

    // Theme picker popup
    if let Some(ref popup) = app.theme_popup {
        draw_theme_popup(frame, theme, popup, size);
    }

    // Key preset popup
    if let Some(ref popup) = app.key_preset_popup {
        draw_key_preset_popup(frame, theme, popup, size);
    }
}

fn draw_header(frame: &mut Frame, app: &App, theme: &ThemeColors, area: Rect) {
    let time = chrono::Local::now().format("%I:%M %p").to_string();

    let conn_style = match app.connection {
        ConnectionStatus::P2P => theme.success(),
        ConnectionStatus::Relay => theme.warning(),
        ConnectionStatus::Connecting => theme.info(),
        ConnectionStatus::Ready => theme.text_muted(),
    };

    // Incognito indicator and privacy hint
    let (incognito_badge, privacy_hint) = if app.incognito {
        (" ⌐■-■ (incognito)", "  stay safe: use a VPN")
    } else {
        ("", "")
    };

    let conn_text = format!("[{} {}]", app.connection.label(), app.connection.symbol());
    // Calculate actual content length: "  Tuit" + badge + "  " + conn + hint + time + "  "
    let content_len =
        6 + incognito_badge.len() + 2 + conn_text.len() + privacy_hint.len() + time.len() + 2;
    let padding = area.width.saturating_sub(content_len as u16) as usize;

    let header = Line::from(vec![
        Span::styled("  Tuit", theme.title()),
        Span::styled(incognito_badge, theme.warning()),
        Span::raw("  "),
        Span::styled(conn_text, conn_style),
        Span::styled(privacy_hint, theme.text_dimmed()),
        Span::raw(" ".repeat(padding)),
        Span::styled(&time, theme.text_muted()),
        Span::raw("  "),
    ]);

    frame.render_widget(Paragraph::new(header).style(theme.bar()), area);
}

fn draw_tabs(frame: &mut Frame, app: &App, theme: &ThemeColors, area: Rect) {
    let active_count = app.transfers.len();

    let titles: Vec<Line> = Mode::ALL
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let active = *m == app.mode;
            let style = if active {
                theme.tab_active()
            } else {
                theme.tab_inactive()
            };
            let marker = if active { " ●" } else { "" };
            // Show count badge on Active tab when transfers are running
            let badge = if *m == Mode::Active && active_count > 0 {
                format!(" ({})", active_count)
            } else {
                String::new()
            };
            Line::from(Span::styled(
                format!("[{}] {}{}{}", i + 1, m.label(), badge, marker),
                style,
            ))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(theme.border_dimmed())
                .style(theme.bar()),
        )
        .select(app.mode.index())
        .divider(Span::styled("  ", theme.text_dimmed()));

    frame.render_widget(tabs, area);
}

fn draw_content(frame: &mut Frame, app: &mut App, theme: &ThemeColors, area: Rect) {
    match app.mode {
        Mode::Send => screens::send::draw(frame, app, theme, area),
        Mode::Receive => screens::receive::draw(frame, app, theme, area),
        Mode::Active => screens::active::draw(frame, app, theme, area),
        Mode::History => screens::history::draw(frame, app, theme, area),
    }
}

fn draw_footer(frame: &mut Frame, app: &App, theme: &ThemeColors, area: Rect) {
    let symlink_status = if app.follow_symlinks {
        "S:symlinks[ON]"
    } else {
        "S:symlinks"
    };
    let hints = match app.mode {
        Mode::Send => format!(
            "Space:sel  a:all  c:clr  /:search  g/G:jump  s:send  {}  ?:help",
            symlink_status
        ),
        Mode::Receive => "Enter:input  Ctrl+V:paste  ?:help  t:theme  B:keys  q:quit".to_string(),
        Mode::Active => {
            "c:copy  p:pause  x:cancel  r:retry  ?:help  t:theme  B:keys  q:quit".to_string()
        }
        Mode::History => "r:resend  c:copy  d:delete  ?:help  t:theme  B:keys  q:quit".to_string(),
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(&hints, theme.text_dimmed()),
    ]))
    .style(theme.bar());

    frame.render_widget(footer, area);
}

fn draw_help(frame: &mut Frame, theme: &ThemeColors, area: Rect) {
    let width = 52.min(area.width.saturating_sub(4));
    let height = 18.min(area.height.saturating_sub(4));
    let popup = centered_rect(width, height, area);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(" Help ", theme.title()))
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .border_set(symbols::border::ROUNDED)
        .style(theme.panel());

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let help = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(" Global", theme.text_highlight())),
        Line::from(vec![
            Span::styled("  q       ", theme.key()),
            Span::styled("Quit", theme.text()),
        ]),
        Line::from(vec![
            Span::styled("  ?       ", theme.key()),
            Span::styled("Toggle help", theme.text()),
        ]),
        Line::from(vec![
            Span::styled("  t       ", theme.key()),
            Span::styled("Cycle theme", theme.text()),
        ]),
        Line::from(vec![
            Span::styled("  1-4     ", theme.key()),
            Span::styled("Switch tabs", theme.text()),
        ]),
        Line::from(vec![
            Span::styled("  Tab     ", theme.key()),
            Span::styled("Next tab", theme.text()),
        ]),
        Line::from(""),
        Line::from(Span::styled(" Navigation", theme.text_highlight())),
        Line::from(vec![
            Span::styled("  j/k     ", theme.key()),
            Span::styled("Move down/up", theme.text()),
        ]),
        Line::from(vec![
            Span::styled("  h/l     ", theme.key()),
            Span::styled("Parent/Enter dir", theme.text()),
        ]),
        Line::from(vec![
            Span::styled("  Enter   ", theme.key()),
            Span::styled("Select/Confirm", theme.text()),
        ]),
        Line::from(""),
        Line::from(Span::styled(" Press any key to close", theme.text_dimmed())),
    ])
    .wrap(Wrap { trim: false });

    frame.render_widget(help, inner);
}

/// Create a centered rectangle
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

/// Generate QR code as lines of text using half-block characters
fn generate_qr_lines(data: &str) -> Option<Vec<String>> {
    use qrcode::{EcLevel, QrCode};

    let code = QrCode::with_error_correction_level(data, EcLevel::L).ok()?;
    let modules = code.to_colors();
    let width = code.width();

    let mut lines = Vec::new();

    // Process two rows at a time using half-block characters
    // Upper half block: ▀, Lower half block: ▄, Full block: █, Space for white
    for y in (0..width).step_by(2) {
        let mut line = String::new();
        // Add quiet zone (white border)
        line.push(' ');

        for x in 0..width {
            let top = modules[y * width + x] == qrcode::Color::Dark;
            let bottom = if y + 1 < width {
                modules[(y + 1) * width + x] == qrcode::Color::Dark
            } else {
                false
            };

            // Using inverted colors (dark background terminal)
            // Dark module = white character, Light module = space
            let ch = match (top, bottom) {
                (true, true) => '█',   // Both dark
                (true, false) => '▀',  // Top dark, bottom light
                (false, true) => '▄',  // Top light, bottom dark
                (false, false) => ' ', // Both light
            };
            line.push(ch);
        }
        line.push(' '); // Quiet zone
        lines.push(line);
    }

    Some(lines)
}

/// Draw a popup showing the full ticket with QR code
fn draw_ticket_popup(frame: &mut Frame, theme: &ThemeColors, ticket: &str, area: Rect) {
    // Try to generate QR code
    let qr_lines = generate_qr_lines(ticket);

    let (width, height) = if let Some(ref qr) = qr_lines {
        // QR code width + padding, height for QR + ticket text + instructions
        let qr_width = qr.first().map(|l| l.len()).unwrap_or(0) as u16;
        let qr_height = qr.len() as u16;
        let w = (qr_width + 4).max(60).min(area.width.saturating_sub(4));
        let h = (qr_height + 8).min(area.height.saturating_sub(4));
        (w, h)
    } else {
        // Fallback to text-only popup
        let w = (ticket.len() as u16 + 6)
            .min(area.width.saturating_sub(4))
            .max(50);
        let h = 9.min(area.height.saturating_sub(4));
        (w, h)
    };

    let popup = centered_rect(width, height, area);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(" Ticket - Scan or Copy ", theme.title()))
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .border_set(symbols::border::DOUBLE)
        .style(theme.panel());

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let mut lines: Vec<Line> = vec![];

    if let Some(qr) = qr_lines {
        // Add QR code lines
        for qr_line in qr {
            lines.push(Line::from(Span::styled(qr_line, theme.text())));
        }
        lines.push(Line::from(""));
    }

    // Add ticket text (wrapped if needed)
    let line_width = inner.width.saturating_sub(2) as usize;
    lines.push(Line::from(Span::styled("Ticket:", theme.text_dimmed())));
    for chunk in ticket.as_bytes().chunks(line_width.max(1)) {
        if let Ok(s) = std::str::from_utf8(chunk) {
            lines.push(Line::from(Span::styled(s, theme.text_muted())));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press any key to close",
        theme.text_dimmed(),
    )));

    let content = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(content, inner);
}

/// Draw the conflict resolution popup
fn draw_conflict_popup(frame: &mut Frame, theme: &ThemeColors, popup: &ConflictPopup, area: Rect) {
    let width = 60.min(area.width.saturating_sub(4));
    let height = 16.min(area.height.saturating_sub(4));
    let popup_area = centered_rect(width, height, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(" File Conflict ", theme.warning()))
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .border_set(symbols::border::DOUBLE)
        .style(theme.panel());

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Build content
    let mut lines: Vec<Line> = vec![];

    // Show conflict count and size
    let size_str = format_bytes(popup.total_bytes);
    lines.push(Line::from(vec![
        Span::styled(
            format!("{} file(s) already exist", popup.conflicts.len()),
            theme.warning(),
        ),
        Span::styled(format!(" ({})", size_str), theme.text_dimmed()),
    ]));
    lines.push(Line::from(""));

    // Show first few conflicting files
    let max_show = 3;
    for (name, _path) in popup.conflicts.iter().take(max_show) {
        lines.push(Line::from(Span::styled(
            format!("  {} {}", "•", name),
            theme.text_muted(),
        )));
    }
    if popup.conflicts.len() > max_show {
        lines.push(Line::from(Span::styled(
            format!("  ... and {} more", popup.conflicts.len() - max_show),
            theme.text_dimmed(),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Choose action:", theme.text())));
    lines.push(Line::from(""));

    // Options
    let options = [
        ("1", "Rename", "Add (1), (2), etc. to new files"),
        ("2", "Overwrite", "Replace existing files"),
        ("3", "Skip", "Don't download existing files"),
        ("4", "Cancel", "Abort the transfer"),
    ];

    for (i, (key, label, desc)) in options.iter().enumerate() {
        let is_selected = i == popup.selected;
        let prefix = if is_selected { "▸ " } else { "  " };
        let style = if is_selected {
            theme.text_highlight()
        } else {
            theme.text()
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(format!("[{}] ", key), theme.key()),
            Span::styled(*label, style),
            Span::styled(format!(" - {}", desc), theme.text_dimmed()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "↑/↓ to select, Enter to confirm, Esc to cancel",
        theme.text_dimmed(),
    )));

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Draw the theme picker popup
fn draw_theme_popup(frame: &mut Frame, theme: &ThemeColors, popup: &ThemePopup, area: Rect) {
    let width = 40.min(area.width.saturating_sub(4));
    let height = 12.min(area.height.saturating_sub(4));
    let popup_area = centered_rect(width, height, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(" Select Theme ", theme.title()))
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .border_set(symbols::border::ROUNDED)
        .style(theme.panel());

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines: Vec<Line> = vec![Line::from("")];

    for (i, theme_kind) in ThemeKind::ALL.iter().enumerate() {
        let is_selected = i == popup.selected;
        let prefix = if is_selected { "▸ " } else { "  " };
        let style = if is_selected {
            theme.text_highlight()
        } else {
            theme.text()
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(format!("[{}] ", i + 1), theme.key()),
            Span::styled(theme_kind.name(), style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "↑/↓ select, Enter confirm, Esc cancel",
        theme.text_dimmed(),
    )));

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);
}

/// Draw the key preset picker popup
fn draw_key_preset_popup(
    frame: &mut Frame,
    theme: &ThemeColors,
    popup: &KeyPresetPopup,
    area: Rect,
) {
    let width = 40.min(area.width.saturating_sub(4));
    let height = 10.min(area.height.saturating_sub(4));
    let popup_area = centered_rect(width, height, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(" Keybindings ", theme.title()))
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .border_set(symbols::border::ROUNDED)
        .style(theme.panel());

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines: Vec<Line> = vec![Line::from("")];

    for (i, preset) in KeyPreset::ALL.iter().enumerate() {
        let is_selected = i == popup.selected;
        let prefix = if is_selected { "▸ " } else { "  " };
        let style = if is_selected {
            theme.text_highlight()
        } else {
            theme.text()
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(format!("[{}] ", i + 1), theme.key()),
            Span::styled(preset.name(), style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "↑/↓ select, Enter confirm, Esc cancel",
        theme.text_dimmed(),
    )));

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);
}
