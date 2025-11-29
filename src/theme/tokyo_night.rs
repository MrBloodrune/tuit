//! Tokyo Night theme
//!
//! Clean, minimal theme inspired by Tokyo's neon lights.
//! https://github.com/tokyo-night/tokyo-night-vscode-theme

use ratatui::style::Color;

use super::ThemeColors;

pub const TOKYO_NIGHT: ThemeColors = ThemeColors {
    // Backgrounds
    bg_primary: Color::Rgb(26, 27, 38),    // #1a1b26
    bg_secondary: Color::Rgb(22, 22, 30),  // #16161e
    bg_surface: Color::Rgb(36, 40, 59),    // #24283b
    bg_highlight: Color::Rgb(65, 72, 104), // #414868

    // Text
    text_primary: Color::Rgb(192, 202, 245),   // #c0caf5
    text_secondary: Color::Rgb(169, 177, 214), // #a9b1d6
    text_dim: Color::Rgb(86, 95, 137),         // #565f89
    text_accent: Color::Rgb(122, 162, 247),    // #7aa2f7

    // Borders
    border_normal: Color::Rgb(65, 72, 104),  // #414868
    border_focus: Color::Rgb(122, 162, 247), // #7aa2f7
    border_dim: Color::Rgb(36, 40, 59),      // #24283b

    // Status
    success: Color::Rgb(158, 206, 106), // #9ece6a
    warning: Color::Rgb(224, 175, 104), // #e0af68
    error: Color::Rgb(247, 118, 142),   // #f7768e
    info: Color::Rgb(125, 207, 255),    // #7dcfff

    // Transfer
    upload: Color::Rgb(158, 206, 106),        // #9ece6a
    download: Color::Rgb(125, 207, 255),      // #7dcfff
    progress_fill: Color::Rgb(122, 162, 247), // #7aa2f7
    progress_empty: Color::Rgb(36, 40, 59),   // #24283b
};
