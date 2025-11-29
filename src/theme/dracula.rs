//! Dracula theme
//!
//! High contrast dark theme with cool colors.
//! https://draculatheme.com/

use ratatui::style::Color;

use super::ThemeColors;

pub const DRACULA: ThemeColors = ThemeColors {
    // Backgrounds
    bg_primary: Color::Rgb(40, 42, 54),     // #282a36
    bg_secondary: Color::Rgb(33, 34, 44),   // #21222c
    bg_surface: Color::Rgb(68, 71, 90),     // #44475a
    bg_highlight: Color::Rgb(98, 114, 164), // #6272a4

    // Text
    text_primary: Color::Rgb(248, 248, 242),   // #f8f8f2
    text_secondary: Color::Rgb(191, 191, 191), // #bfbfbf
    text_dim: Color::Rgb(98, 114, 164),        // #6272a4
    text_accent: Color::Rgb(189, 147, 249),    // #bd93f9

    // Borders
    border_normal: Color::Rgb(68, 71, 90),   // #44475a
    border_focus: Color::Rgb(189, 147, 249), // #bd93f9
    border_dim: Color::Rgb(33, 34, 44),      // #21222c

    // Status
    success: Color::Rgb(80, 250, 123),  // #50fa7b
    warning: Color::Rgb(241, 250, 140), // #f1fa8c
    error: Color::Rgb(255, 85, 85),     // #ff5555
    info: Color::Rgb(139, 233, 253),    // #8be9fd

    // Transfer
    upload: Color::Rgb(80, 250, 123),         // #50fa7b
    download: Color::Rgb(139, 233, 253),      // #8be9fd
    progress_fill: Color::Rgb(189, 147, 249), // #bd93f9
    progress_empty: Color::Rgb(68, 71, 90),   // #44475a
};
