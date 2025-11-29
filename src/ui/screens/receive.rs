//! Receive mode screen - ticket input

use ratatui::{
    layout::Rect,
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::theme::ThemeColors;
use crate::ui::layout::centered;

pub fn draw(frame: &mut Frame, app: &App, theme: &ThemeColors, area: Rect) {
    // Center the input box - taller to accommodate wrapped tickets
    let width = 60.min(area.width.saturating_sub(4));
    let height = 12;
    let popup = centered(width, height, area);

    let block = Block::default()
        .title(Span::styled(" Receive ", theme.title()))
        .borders(Borders::ALL)
        .border_style(if app.input_active {
            theme.border_focused()
        } else {
            theme.border()
        })
        .border_set(symbols::border::ROUNDED)
        .style(theme.panel());

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // Input field
    let cursor = if app.input_active { "█" } else { "" };
    let display = if app.ticket_input.is_empty() && !app.input_active {
        "Press Enter to paste ticket...".to_string()
    } else {
        format!("{}{}", app.ticket_input, cursor)
    };

    let input_style = if app.input_active {
        theme.text_highlight()
    } else {
        theme.text_dimmed()
    };

    // Hint text changes based on whether there's input
    let hint = if app.ticket_input.is_empty() {
        "  Ctrl+V paste  •  Enter to receive"
    } else {
        "  Ctrl+V paste  •  Ctrl+U clear  •  Enter to receive"
    };

    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Paste a ticket to receive files:",
            theme.text_muted(),
        )),
        Line::from(""),
        Line::from(vec![Span::raw("  "), Span::styled(&display, input_style)]),
        Line::from(""),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(hint, theme.text_dimmed())),
    ])
    .wrap(Wrap { trim: false });

    frame.render_widget(content, inner);

    // Save directory info below
    let save_area = Rect {
        y: popup.y + height + 1,
        x: popup.x,
        width,
        height: 2,
    };

    let save_info = Paragraph::new(vec![Line::from(vec![
        Span::styled("  Save to: ", theme.text_dimmed()),
        Span::styled(app.receive_dir.display().to_string(), theme.text()),
    ])]);

    if save_area.y + save_area.height <= area.y + area.height {
        frame.render_widget(save_info, save_area);
    }
}
