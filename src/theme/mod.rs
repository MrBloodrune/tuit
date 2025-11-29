//! Theme system for Tuit
//!
//! Supports 5 dark themes: Catppuccin Mocha, Tokyo Night, Dracula, Nord, Gruvbox Dark

mod catppuccin;
mod dracula;
mod gruvbox;
mod nord;
mod tokyo_night;

pub use catppuccin::CATPPUCCIN_MOCHA;
pub use dracula::DRACULA;
pub use gruvbox::GRUVBOX_DARK;
pub use nord::NORD;
pub use tokyo_night::TOKYO_NIGHT;

use ratatui::style::{Color, Modifier, Style};

/// All available themes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeKind {
    CatppuccinMocha,
    #[default]
    TokyoNight,
    Dracula,
    Nord,
    GruvboxDark,
}

impl ThemeKind {
    pub const ALL: [ThemeKind; 5] = [
        ThemeKind::CatppuccinMocha,
        ThemeKind::TokyoNight,
        ThemeKind::Dracula,
        ThemeKind::Nord,
        ThemeKind::GruvboxDark,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ThemeKind::CatppuccinMocha => "Catppuccin Mocha",
            ThemeKind::TokyoNight => "Tokyo Night",
            ThemeKind::Dracula => "Dracula",
            ThemeKind::Nord => "Nord",
            ThemeKind::GruvboxDark => "Gruvbox Dark",
        }
    }

    /// Parse theme from name string (case-insensitive, flexible matching)
    pub fn from_name(name: &str) -> Self {
        let normalized = name.to_lowercase().replace(" ", "").replace("-", "");
        match normalized.as_str() {
            "catppuccin" | "catppuccinmocha" | "mocha" => ThemeKind::CatppuccinMocha,
            "tokyonight" | "tokyo" => ThemeKind::TokyoNight,
            "dracula" => ThemeKind::Dracula,
            "nord" => ThemeKind::Nord,
            "gruvbox" | "gruvboxdark" => ThemeKind::GruvboxDark,
            _ => ThemeKind::default(), // fallback to default
        }
    }

    pub fn colors(&self) -> &'static ThemeColors {
        match self {
            ThemeKind::CatppuccinMocha => &CATPPUCCIN_MOCHA,
            ThemeKind::TokyoNight => &TOKYO_NIGHT,
            ThemeKind::Dracula => &DRACULA,
            ThemeKind::Nord => &NORD,
            ThemeKind::GruvboxDark => &GRUVBOX_DARK,
        }
    }
}

/// Theme color palette
#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    // Backgrounds
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_surface: Color,
    pub bg_highlight: Color,

    // Text
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_dim: Color,
    pub text_accent: Color,

    // Borders
    pub border_normal: Color,
    pub border_focus: Color,
    pub border_dim: Color,

    // Status
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Transfer-specific
    pub upload: Color,
    pub download: Color,
    pub progress_fill: Color,
    pub progress_empty: Color,
}

impl ThemeColors {
    // Style helpers

    /// Primary text style
    pub fn text(&self) -> Style {
        Style::default().fg(self.text_primary)
    }

    /// Secondary/muted text
    pub fn text_muted(&self) -> Style {
        Style::default().fg(self.text_secondary)
    }

    /// Dim text (hints, disabled)
    pub fn text_dimmed(&self) -> Style {
        Style::default().fg(self.text_dim)
    }

    /// Accent text (highlighted)
    pub fn text_highlight(&self) -> Style {
        Style::default()
            .fg(self.text_accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Title style
    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.text_accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Focused border
    pub fn border_focused(&self) -> Style {
        Style::default().fg(self.border_focus)
    }

    /// Normal border
    pub fn border(&self) -> Style {
        Style::default().fg(self.border_normal)
    }

    /// Dim border
    pub fn border_dimmed(&self) -> Style {
        Style::default().fg(self.border_dim)
    }

    /// Selected item style
    pub fn selected(&self) -> Style {
        Style::default().fg(self.text_accent).bg(self.bg_highlight)
    }

    /// Success style
    pub fn success(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Warning style
    pub fn warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// Error style
    pub fn error(&self) -> Style {
        Style::default().fg(self.error)
    }

    /// Info style
    pub fn info(&self) -> Style {
        Style::default().fg(self.info)
    }

    /// Upload indicator
    pub fn upload(&self) -> Style {
        Style::default()
            .fg(self.upload)
            .add_modifier(Modifier::BOLD)
    }

    /// Download indicator
    pub fn download(&self) -> Style {
        Style::default()
            .fg(self.download)
            .add_modifier(Modifier::BOLD)
    }

    /// Active tab style
    pub fn tab_active(&self) -> Style {
        Style::default()
            .fg(self.text_accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }

    /// Inactive tab style
    pub fn tab_inactive(&self) -> Style {
        Style::default().fg(self.text_secondary)
    }

    /// Key hint style (the key itself)
    pub fn key(&self) -> Style {
        Style::default()
            .fg(self.text_accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Progress bar style
    pub fn progress(&self) -> Style {
        Style::default()
            .fg(self.progress_fill)
            .bg(self.progress_empty)
    }

    /// Progress text style
    pub fn progress_text(&self) -> Style {
        Style::default().fg(self.text_accent)
    }

    /// Panel background
    pub fn panel(&self) -> Style {
        Style::default().bg(self.bg_surface)
    }

    /// Main background
    pub fn background(&self) -> Style {
        Style::default().bg(self.bg_primary)
    }

    /// Header/footer background
    pub fn bar(&self) -> Style {
        Style::default().bg(self.bg_secondary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_all_array_has_five_themes() {
        assert_eq!(ThemeKind::ALL.len(), 5);
    }

    #[test]
    fn test_all_themes_have_names() {
        for theme in &ThemeKind::ALL {
            let name = theme.name();
            assert!(!name.is_empty(), "Theme {:?} has an empty name", theme);
        }
    }

    #[test]
    fn test_all_themes_unique_names() {
        let mut names = HashSet::new();
        for theme in &ThemeKind::ALL {
            let name = theme.name();
            assert!(names.insert(name), "Duplicate theme name found: {}", name);
        }
    }

    #[test]
    fn test_from_name_exact_matches() {
        // Test exact matches (case-sensitive form)
        assert_eq!(
            ThemeKind::from_name("Catppuccin Mocha"),
            ThemeKind::CatppuccinMocha
        );
        assert_eq!(ThemeKind::from_name("Tokyo Night"), ThemeKind::TokyoNight);
        assert_eq!(ThemeKind::from_name("Dracula"), ThemeKind::Dracula);
        assert_eq!(ThemeKind::from_name("Nord"), ThemeKind::Nord);
        assert_eq!(ThemeKind::from_name("Gruvbox Dark"), ThemeKind::GruvboxDark);
    }

    #[test]
    fn test_from_name_flexible_matching() {
        // Test hyphenated and space-less variants
        assert_eq!(ThemeKind::from_name("tokyo-night"), ThemeKind::TokyoNight);
        assert_eq!(ThemeKind::from_name("tokyonight"), ThemeKind::TokyoNight);
        assert_eq!(
            ThemeKind::from_name("catppuccin-mocha"),
            ThemeKind::CatppuccinMocha
        );
        assert_eq!(
            ThemeKind::from_name("catppuccinmocha"),
            ThemeKind::CatppuccinMocha
        );
        assert_eq!(ThemeKind::from_name("gruvbox-dark"), ThemeKind::GruvboxDark);
        assert_eq!(ThemeKind::from_name("gruvboxdark"), ThemeKind::GruvboxDark);

        // Test short aliases
        assert_eq!(ThemeKind::from_name("mocha"), ThemeKind::CatppuccinMocha);
        assert_eq!(ThemeKind::from_name("tokyo"), ThemeKind::TokyoNight);
        assert_eq!(ThemeKind::from_name("gruvbox"), ThemeKind::GruvboxDark);
    }

    #[test]
    fn test_from_name_case_insensitive() {
        // Test uppercase
        assert_eq!(ThemeKind::from_name("DRACULA"), ThemeKind::Dracula);
        assert_eq!(ThemeKind::from_name("TOKYO NIGHT"), ThemeKind::TokyoNight);
        assert_eq!(ThemeKind::from_name("NORD"), ThemeKind::Nord);

        // Test lowercase
        assert_eq!(ThemeKind::from_name("dracula"), ThemeKind::Dracula);
        assert_eq!(ThemeKind::from_name("tokyo night"), ThemeKind::TokyoNight);
        assert_eq!(ThemeKind::from_name("nord"), ThemeKind::Nord);

        // Test mixed case
        assert_eq!(ThemeKind::from_name("DrAcUlA"), ThemeKind::Dracula);
        assert_eq!(ThemeKind::from_name("ToKyO NiGhT"), ThemeKind::TokyoNight);
    }

    #[test]
    fn test_from_name_unknown_returns_default() {
        // Unknown names should return the default (TokyoNight)
        assert_eq!(ThemeKind::from_name("nonexistent"), ThemeKind::TokyoNight);
        assert_eq!(ThemeKind::from_name("invalid_theme"), ThemeKind::TokyoNight);
        assert_eq!(ThemeKind::from_name("foobar123"), ThemeKind::TokyoNight);
    }

    #[test]
    fn test_from_name_empty_string() {
        // Empty string should return default
        assert_eq!(ThemeKind::from_name(""), ThemeKind::TokyoNight);
    }

    #[test]
    fn test_colors_returns_valid_colors() {
        // Ensure colors() doesn't panic for any theme
        for theme in &ThemeKind::ALL {
            let colors = theme.colors();
            // Basic validation: accessing a field shouldn't panic
            let _ = colors.bg_primary;
            let _ = colors.text_primary;
            let _ = colors.border_normal;
            let _ = colors.success;
        }
    }

    #[test]
    fn test_style_helpers_set_foreground() {
        // Test that style helper methods set foreground colors correctly
        for theme in &ThemeKind::ALL {
            let colors = theme.colors();

            // Test text styles
            let text_style = colors.text();
            assert_eq!(text_style.fg, Some(colors.text_primary));

            let muted_style = colors.text_muted();
            assert_eq!(muted_style.fg, Some(colors.text_secondary));

            let dimmed_style = colors.text_dimmed();
            assert_eq!(dimmed_style.fg, Some(colors.text_dim));

            // Test status styles
            let error_style = colors.error();
            assert_eq!(error_style.fg, Some(colors.error));

            let success_style = colors.success();
            assert_eq!(success_style.fg, Some(colors.success));

            let warning_style = colors.warning();
            assert_eq!(warning_style.fg, Some(colors.warning));

            let info_style = colors.info();
            assert_eq!(info_style.fg, Some(colors.info));

            // Test transfer styles
            let upload_style = colors.upload();
            assert_eq!(upload_style.fg, Some(colors.upload));

            let download_style = colors.download();
            assert_eq!(download_style.fg, Some(colors.download));

            // Test border styles
            let border_style = colors.border();
            assert_eq!(border_style.fg, Some(colors.border_normal));

            let border_focused_style = colors.border_focused();
            assert_eq!(border_focused_style.fg, Some(colors.border_focus));

            let border_dimmed_style = colors.border_dimmed();
            assert_eq!(border_dimmed_style.fg, Some(colors.border_dim));
        }
    }

    #[test]
    fn test_each_theme_has_distinct_primary_bg() {
        // Collect all bg_primary colors
        let mut backgrounds = HashSet::new();

        for theme in &ThemeKind::ALL {
            let colors = theme.colors();
            let bg = colors.bg_primary;

            // Extract RGB values for comparison
            let rgb_value = match bg {
                Color::Rgb(r, g, b) => (r, g, b),
                _ => panic!("Expected RGB color for theme {:?}", theme),
            };

            assert!(
                backgrounds.insert(rgb_value),
                "Theme {:?} has duplicate bg_primary color {:?}",
                theme,
                bg
            );
        }

        // Verify we have 5 distinct backgrounds (one per theme)
        assert_eq!(backgrounds.len(), 5);
    }

    #[test]
    fn test_theme_kind_default() {
        // Verify that the default theme is TokyoNight
        let default_theme = ThemeKind::default();
        assert_eq!(default_theme, ThemeKind::TokyoNight);
    }

    #[test]
    fn test_style_helpers_with_modifiers() {
        // Test that certain style helpers add expected modifiers
        let colors = ThemeKind::TokyoNight.colors();

        // text_highlight should have BOLD
        let highlight = colors.text_highlight();
        assert!(highlight.add_modifier.contains(Modifier::BOLD));

        // title should have BOLD
        let title = colors.title();
        assert!(title.add_modifier.contains(Modifier::BOLD));

        // upload should have BOLD
        let upload = colors.upload();
        assert!(upload.add_modifier.contains(Modifier::BOLD));

        // download should have BOLD
        let download = colors.download();
        assert!(download.add_modifier.contains(Modifier::BOLD));

        // tab_active should have BOLD and UNDERLINED
        let tab_active = colors.tab_active();
        assert!(tab_active.add_modifier.contains(Modifier::BOLD));
        assert!(tab_active.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn test_style_helpers_with_background() {
        let colors = ThemeKind::TokyoNight.colors();

        // selected should have both fg and bg
        let selected = colors.selected();
        assert_eq!(selected.fg, Some(colors.text_accent));
        assert_eq!(selected.bg, Some(colors.bg_highlight));

        // background should have bg set
        let background = colors.background();
        assert_eq!(background.bg, Some(colors.bg_primary));

        // panel should have bg set
        let panel = colors.panel();
        assert_eq!(panel.bg, Some(colors.bg_surface));

        // bar should have bg set
        let bar = colors.bar();
        assert_eq!(bar.bg, Some(colors.bg_secondary));

        // progress should have both fg and bg
        let progress = colors.progress();
        assert_eq!(progress.fg, Some(colors.progress_fill));
        assert_eq!(progress.bg, Some(colors.progress_empty));
    }

    #[test]
    fn test_all_themes_present_in_all_array() {
        // Verify ALL contains each variant exactly once
        assert!(ThemeKind::ALL.contains(&ThemeKind::CatppuccinMocha));
        assert!(ThemeKind::ALL.contains(&ThemeKind::TokyoNight));
        assert!(ThemeKind::ALL.contains(&ThemeKind::Dracula));
        assert!(ThemeKind::ALL.contains(&ThemeKind::Nord));
        assert!(ThemeKind::ALL.contains(&ThemeKind::GruvboxDark));

        // Verify no duplicates by checking each variant appears exactly once
        assert_eq!(
            ThemeKind::ALL
                .iter()
                .filter(|&&t| t == ThemeKind::CatppuccinMocha)
                .count(),
            1
        );
        assert_eq!(
            ThemeKind::ALL
                .iter()
                .filter(|&&t| t == ThemeKind::TokyoNight)
                .count(),
            1
        );
        assert_eq!(
            ThemeKind::ALL
                .iter()
                .filter(|&&t| t == ThemeKind::Dracula)
                .count(),
            1
        );
        assert_eq!(
            ThemeKind::ALL
                .iter()
                .filter(|&&t| t == ThemeKind::Nord)
                .count(),
            1
        );
        assert_eq!(
            ThemeKind::ALL
                .iter()
                .filter(|&&t| t == ThemeKind::GruvboxDark)
                .count(),
            1
        );
    }
}
