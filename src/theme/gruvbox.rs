//! Gruvbox Dark theme
//!
//! Warm retro groove color palette.
//! https://github.com/morhetz/gruvbox

use ratatui::style::Color;

use super::ThemeColors;

pub const GRUVBOX_DARK: ThemeColors = ThemeColors {
    // Backgrounds
    bg_primary: Color::Rgb(40, 40, 40),   // #282828 bg0
    bg_secondary: Color::Rgb(29, 32, 33), // #1d2021 bg0_h
    bg_surface: Color::Rgb(60, 56, 54),   // #3c3836 bg1
    bg_highlight: Color::Rgb(80, 73, 69), // #504945 bg2

    // Text
    text_primary: Color::Rgb(235, 219, 178),   // #ebdbb2 fg1
    text_secondary: Color::Rgb(213, 196, 161), // #d5c4a1 fg2
    text_dim: Color::Rgb(102, 92, 84),         // #665c54 bg4
    text_accent: Color::Rgb(215, 153, 33),     // #d79921 yellow

    // Borders
    border_normal: Color::Rgb(80, 73, 69),  // #504945 bg2
    border_focus: Color::Rgb(215, 153, 33), // #d79921 yellow
    border_dim: Color::Rgb(60, 56, 54),     // #3c3836 bg1

    // Status
    success: Color::Rgb(184, 187, 38), // #b8bb26 green
    warning: Color::Rgb(250, 189, 47), // #fabd2f yellow bright
    error: Color::Rgb(251, 73, 52),    // #fb4934 red bright
    info: Color::Rgb(131, 165, 152),   // #83a598 blue

    // Transfer
    upload: Color::Rgb(184, 187, 38),        // #b8bb26 green
    download: Color::Rgb(131, 165, 152),     // #83a598 blue
    progress_fill: Color::Rgb(215, 153, 33), // #d79921 yellow
    progress_empty: Color::Rgb(60, 56, 54),  // #3c3836 bg1
};
