//! Catppuccin Mocha theme
//!
//! Modern pastel theme with warm undertones.
//! https://catppuccin.com/

use ratatui::style::Color;

use super::ThemeColors;

pub const CATPPUCCIN_MOCHA: ThemeColors = ThemeColors {
    // Backgrounds
    bg_primary: Color::Rgb(30, 30, 46),   // #1e1e2e Base
    bg_secondary: Color::Rgb(24, 24, 37), // #181825 Mantle
    bg_surface: Color::Rgb(49, 50, 68),   // #313244 Surface0
    bg_highlight: Color::Rgb(69, 71, 90), // #45475a Surface1

    // Text
    text_primary: Color::Rgb(205, 214, 244),   // #cdd6f4 Text
    text_secondary: Color::Rgb(166, 173, 200), // #a6adc8 Subtext0
    text_dim: Color::Rgb(108, 112, 134),       // #6c7086 Overlay0
    text_accent: Color::Rgb(203, 166, 247),    // #cba6f7 Mauve

    // Borders
    border_normal: Color::Rgb(69, 71, 90),   // #45475a Surface1
    border_focus: Color::Rgb(203, 166, 247), // #cba6f7 Mauve
    border_dim: Color::Rgb(49, 50, 68),      // #313244 Surface0

    // Status
    success: Color::Rgb(166, 227, 161), // #a6e3a1 Green
    warning: Color::Rgb(249, 226, 175), // #f9e2af Yellow
    error: Color::Rgb(243, 139, 168),   // #f38ba8 Red
    info: Color::Rgb(137, 220, 235),    // #89dceb Sky

    // Transfer
    upload: Color::Rgb(166, 227, 161),        // #a6e3a1 Green
    download: Color::Rgb(137, 220, 235),      // #89dceb Sky
    progress_fill: Color::Rgb(203, 166, 247), // #cba6f7 Mauve
    progress_empty: Color::Rgb(49, 50, 68),   // #313244 Surface0
};
