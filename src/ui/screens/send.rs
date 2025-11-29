//! Send mode screen - tree browser and selection

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use tui_tree_widget::Tree;

use crate::app::App;
use crate::theme::ThemeColors;

pub fn draw(frame: &mut Frame, app: &mut App, theme: &ThemeColors, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    draw_tree_browser(frame, app, theme, chunks[0]);
    draw_selection(frame, app, theme, chunks[1]);
}

fn draw_tree_browser(frame: &mut Frame, app: &mut App, theme: &ThemeColors, area: Rect) {
    let title = if app.tree_browser.search_active {
        format!(
            " Search: {} (Enter:done  Space:sel  Esc:cancel) ",
            app.tree_browser.search_query
        )
    } else if app.tree_browser.has_search_results() {
        format!(
            " Results: {} (Space:sel  Esc:clear) ",
            app.tree_browser.search_results.len()
        )
    } else {
        " Files ".to_string()
    };

    let block = Block::default()
        .title(Span::styled(title, theme.title()))
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .border_set(symbols::border::ROUNDED)
        .style(theme.panel());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let dir_str = app.tree_browser.root_dir.display().to_string();
    let dir_line = Paragraph::new(Line::from(Span::styled(&dir_str, theme.text_muted())));
    frame.render_widget(dir_line, Rect { height: 1, ..inner });

    let tree_area = Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(1),
        ..inner
    };

    let items = app.tree_browser.tree_items();
    let tree = Tree::new(&items)
        .expect("valid tree")
        .highlight_style(theme.selected())
        .highlight_symbol("> ");

    frame.render_stateful_widget(tree, tree_area, &mut app.tree_browser.state);
}

fn draw_selection(frame: &mut Frame, app: &App, theme: &ThemeColors, area: Rect) {
    let count = app.tree_browser.selected.len();
    let title = format!(" Selected ({}) ", count);

    let block = Block::default()
        .title(Span::styled(title, theme.title()))
        .borders(Borders::ALL)
        .border_style(theme.border())
        .border_set(symbols::border::ROUNDED)
        .style(theme.panel());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.tree_browser.selected.is_empty() {
        let hint = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  No files selected", theme.text_dimmed())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Press ", theme.text_dimmed()),
                Span::styled("Space", theme.key()),
                Span::styled(" to select", theme.text_dimmed()),
            ]),
        ]);
        frame.render_widget(hint, inner);
        return;
    }

    let total_size: u64 = app
        .tree_browser
        .selected
        .iter()
        .filter_map(|p| p.metadata().ok())
        .map(|m| m.len())
        .sum();
    let size_str = humansize::format_size(total_size, humansize::BINARY);

    let list_height = inner.height.saturating_sub(4);
    let items: Vec<ListItem> = app
        .tree_browser
        .selected
        .iter()
        .take(list_height as usize)
        .map(|path| {
            let is_dir = path.is_dir();
            let icon = if is_dir { "📁 " } else { "📄 " };
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            ListItem::new(Line::from(Span::styled(
                format!("  {}{}", icon, name),
                theme.text(),
            )))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(
        list,
        Rect {
            height: list_height,
            ..inner
        },
    );

    let bottom_area = Rect {
        y: inner.y + inner.height.saturating_sub(3),
        height: 3,
        ..inner
    };

    let bottom = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("  Total: {}", size_str),
            theme.text_muted(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "[ SEND ]",
                theme.success().add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("  s or Enter", theme.text_dimmed()),
        ]),
    ]);
    frame.render_widget(bottom, bottom_area);
}
