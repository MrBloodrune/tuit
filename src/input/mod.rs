//! Input handling
//!
//! Keybinding presets for different navigation styles.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};

/// Keybinding preset styles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum KeyPreset {
    /// Vim-style (j/k/h/l) + arrow keys (default)
    #[default]
    Vim,
    /// Arrow keys only
    Arrows,
    /// Emacs-style (Ctrl+n/p/f/b) + arrow keys
    Emacs,
}

impl KeyPreset {
    pub const ALL: [KeyPreset; 3] = [KeyPreset::Vim, KeyPreset::Arrows, KeyPreset::Emacs];

    pub fn name(&self) -> &'static str {
        match self {
            KeyPreset::Vim => "Vim (hjkl)",
            KeyPreset::Arrows => "Arrows",
            KeyPreset::Emacs => "Emacs (C-npfb)",
        }
    }

    /// Parse key preset from name string (case-insensitive)
    pub fn from_name(name: &str) -> Self {
        let normalized = name.to_lowercase();
        match normalized.as_str() {
            "vim" | "vi" => KeyPreset::Vim,
            "arrows" | "arrow" => KeyPreset::Arrows,
            "emacs" | "emac" => KeyPreset::Emacs,
            _ => KeyPreset::default(), // fallback to default
        }
    }

    /// Check if key matches "move up" action
    pub fn is_up(&self, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Up => true,
            KeyCode::Char('k') if *self == KeyPreset::Vim => true,
            KeyCode::Char('p')
                if *self == KeyPreset::Emacs && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                true
            }
            _ => false,
        }
    }

    /// Check if key matches "move down" action
    pub fn is_down(&self, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Down => true,
            KeyCode::Char('j') if *self == KeyPreset::Vim => true,
            KeyCode::Char('n')
                if *self == KeyPreset::Emacs && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                true
            }
            _ => false,
        }
    }

    /// Check if key matches "move left" / "go back" action
    pub fn is_left(&self, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Left | KeyCode::Backspace => true,
            KeyCode::Char('h') if *self == KeyPreset::Vim => true,
            KeyCode::Char('b')
                if *self == KeyPreset::Emacs && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                true
            }
            _ => false,
        }
    }

    /// Check if key matches "move right" / "enter" action
    pub fn is_right(&self, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Right => true,
            KeyCode::Char('l') if *self == KeyPreset::Vim => true,
            KeyCode::Char('f')
                if *self == KeyPreset::Emacs && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn key_ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn test_vim_hjkl_mapping() {
        let preset = KeyPreset::Vim;
        assert!(preset.is_down(&key(KeyCode::Char('j'))));
        assert!(preset.is_up(&key(KeyCode::Char('k'))));
        assert!(preset.is_left(&key(KeyCode::Char('h'))));
        assert!(preset.is_right(&key(KeyCode::Char('l'))));
    }

    #[test]
    fn test_vim_arrow_keys_also_work() {
        let preset = KeyPreset::Vim;
        assert!(preset.is_up(&key(KeyCode::Up)));
        assert!(preset.is_down(&key(KeyCode::Down)));
        assert!(preset.is_left(&key(KeyCode::Left)));
        assert!(preset.is_right(&key(KeyCode::Right)));
    }

    #[test]
    fn test_arrows_preset_ignores_letters() {
        let preset = KeyPreset::Arrows;
        assert!(!preset.is_down(&key(KeyCode::Char('j'))));
        assert!(!preset.is_up(&key(KeyCode::Char('k'))));
        assert!(!preset.is_left(&key(KeyCode::Char('h'))));
        assert!(!preset.is_right(&key(KeyCode::Char('l'))));

        // But arrow keys should work
        assert!(preset.is_down(&key(KeyCode::Down)));
        assert!(preset.is_up(&key(KeyCode::Up)));
    }

    #[test]
    fn test_emacs_requires_ctrl() {
        let preset = KeyPreset::Emacs;
        assert!(!preset.is_down(&key(KeyCode::Char('n'))));
        assert!(!preset.is_up(&key(KeyCode::Char('p'))));
        assert!(!preset.is_right(&key(KeyCode::Char('f'))));
        assert!(!preset.is_left(&key(KeyCode::Char('b'))));
    }

    #[test]
    fn test_emacs_ctrl_npfb() {
        let preset = KeyPreset::Emacs;
        assert!(preset.is_down(&key_ctrl('n')));
        assert!(preset.is_up(&key_ctrl('p')));
        assert!(preset.is_right(&key_ctrl('f')));
        assert!(preset.is_left(&key_ctrl('b')));
    }

    #[test]
    fn test_backspace_is_left_all_presets() {
        let backspace = key(KeyCode::Backspace);
        assert!(KeyPreset::Vim.is_left(&backspace));
        assert!(KeyPreset::Arrows.is_left(&backspace));
        assert!(KeyPreset::Emacs.is_left(&backspace));
    }

    #[test]
    fn test_from_name_case_insensitive() {
        assert_eq!(KeyPreset::from_name("VIM"), KeyPreset::Vim);
        assert_eq!(KeyPreset::from_name("vim"), KeyPreset::Vim);
        assert_eq!(KeyPreset::from_name("Vim"), KeyPreset::Vim);
        assert_eq!(KeyPreset::from_name("ARROWS"), KeyPreset::Arrows);
        assert_eq!(KeyPreset::from_name("arrows"), KeyPreset::Arrows);
        assert_eq!(KeyPreset::from_name("EMACS"), KeyPreset::Emacs);
    }

    #[test]
    fn test_from_name_aliases() {
        assert_eq!(KeyPreset::from_name("vi"), KeyPreset::Vim);
        assert_eq!(KeyPreset::from_name("arrow"), KeyPreset::Arrows);
        assert_eq!(KeyPreset::from_name("emac"), KeyPreset::Emacs);
    }

    #[test]
    fn test_from_name_unknown_fallback() {
        assert_eq!(KeyPreset::from_name("qwerty"), KeyPreset::Vim);
        assert_eq!(KeyPreset::from_name(""), KeyPreset::Vim);
    }

    #[test]
    fn test_preset_names_non_empty() {
        assert!(!KeyPreset::Vim.name().is_empty());
        assert!(!KeyPreset::Arrows.name().is_empty());
        assert!(!KeyPreset::Emacs.name().is_empty());
    }

    #[test]
    fn test_preset_all_constant() {
        assert_eq!(KeyPreset::ALL.len(), 3);
        assert!(KeyPreset::ALL.contains(&KeyPreset::Vim));
        assert!(KeyPreset::ALL.contains(&KeyPreset::Arrows));
        assert!(KeyPreset::ALL.contains(&KeyPreset::Emacs));
    }

    #[test]
    fn test_default_preset_is_vim() {
        assert_eq!(KeyPreset::default(), KeyPreset::Vim);
    }
}
