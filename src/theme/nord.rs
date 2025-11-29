//! Nord theme
//!
//! Arctic, north-bluish color palette.
//! https://www.nordtheme.com/

use ratatui::style::Color;

use super::ThemeColors;

pub const NORD: ThemeColors = ThemeColors {
    // Backgrounds (Polar Night)
    bg_primary: Color::Rgb(46, 52, 64),    // #2e3440 nord0
    bg_secondary: Color::Rgb(59, 66, 82),  // #3b4252 nord1
    bg_surface: Color::Rgb(67, 76, 94),    // #434c5e nord2
    bg_highlight: Color::Rgb(76, 86, 106), // #4c566a nord3

    // Text (Snow Storm)
    text_primary: Color::Rgb(236, 239, 244), // #eceff4 nord6
    text_secondary: Color::Rgb(216, 222, 233), // #d8dee9 nord4
    text_dim: Color::Rgb(76, 86, 106),       // #4c566a nord3
    text_accent: Color::Rgb(136, 192, 208),  // #88c0d0 nord8

    // Borders
    border_normal: Color::Rgb(76, 86, 106),  // #4c566a nord3
    border_focus: Color::Rgb(136, 192, 208), // #88c0d0 nord8
    border_dim: Color::Rgb(67, 76, 94),      // #434c5e nord2

    // Status (Aurora)
    success: Color::Rgb(163, 190, 140), // #a3be8c nord14
    warning: Color::Rgb(235, 203, 139), // #ebcb8b nord13
    error: Color::Rgb(191, 97, 106),    // #bf616a nord11
    info: Color::Rgb(129, 161, 193),    // #81a1c1 nord9

    // Transfer
    upload: Color::Rgb(163, 190, 140),        // #a3be8c nord14
    download: Color::Rgb(129, 161, 193),      // #81a1c1 nord9
    progress_fill: Color::Rgb(136, 192, 208), // #88c0d0 nord8
    progress_empty: Color::Rgb(67, 76, 94),   // #434c5e nord2
};
