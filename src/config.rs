//! Configuration management for Tuit.
//!
//! This module handles loading, saving, and managing application configuration
//! using TOML format stored in the XDG config directory.

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration structure containing all configuration sections.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub persistence: PersistenceConfig,
    pub preferences: PreferencesConfig,
    pub transfer: TransferConfig,
}

/// Configuration for data persistence features.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PersistenceConfig {
    /// Whether to maintain transfer history.
    pub history: bool,
}

/// User interface and behavior preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PreferencesConfig {
    /// Theme name to use for the TUI.
    pub theme: String,

    /// Key binding preset name.
    pub key_preset: String,

    /// Directory to save received files.
    /// If None, defaults to ~/Downloads at runtime.
    pub receive_dir: Option<PathBuf>,
}

/// Transfer-related configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TransferConfig {
    /// Maximum number of concurrent send operations.
    pub max_concurrent_sends: usize,

    /// Maximum number of concurrent receive operations.
    pub max_concurrent_receives: usize,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self { history: true }
    }
}

impl Default for PreferencesConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            key_preset: "arrows".to_string(),
            receive_dir: None,
        }
    }
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            max_concurrent_sends: 50,
            max_concurrent_receives: 50,
        }
    }
}

impl Config {
    /// Returns the default configuration file path using XDG conventions.
    ///
    /// Returns None if the project directories cannot be determined.
    pub fn default_path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "intuit")
            .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
    }

    /// Loads configuration from the default XDG config path.
    ///
    /// If the config file doesn't exist or cannot be read, returns default configuration.
    /// Errors during parsing are logged but do not cause panics.
    pub fn load() -> Self {
        Self::load_from(Self::default_path())
    }

    /// Loads configuration from a specific path.
    ///
    /// If path is None or the file doesn't exist, returns default configuration.
    /// Any errors during loading or parsing result in returning defaults.
    ///
    /// # Arguments
    ///
    /// * `path` - Optional path to the configuration file
    pub fn load_from(path: Option<PathBuf>) -> Self {
        let Some(config_path) = path else {
            return Self::default();
        };

        match fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse config file at {}: {}. Using defaults.",
                        config_path.display(),
                        e
                    );
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File doesn't exist, use defaults silently
                Self::default()
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to read config file at {}: {}. Using defaults.",
                    config_path.display(),
                    e
                );
                Self::default()
            }
        }
    }

    /// Saves the current configuration to the default XDG config path.
    ///
    /// Creates parent directories if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The default config path cannot be determined
    /// - Parent directories cannot be created
    /// - The file cannot be written
    /// - Serialization to TOML fails
    pub fn save(&self) -> Result<()> {
        let config_path = Self::default_path().context("Failed to determine config directory")?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let toml_string =
            toml::to_string_pretty(self).context("Failed to serialize config to TOML")?;

        fs::write(&config_path, toml_string).context(format!(
            "Failed to write config file to {}",
            config_path.display()
        ))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.persistence.history);
        assert_eq!(config.preferences.theme, "default");
        assert_eq!(config.preferences.key_preset, "arrows");
        assert_eq!(config.preferences.receive_dir, None);
        assert_eq!(config.transfer.max_concurrent_sends, 50);
        assert_eq!(config.transfer.max_concurrent_receives, 50);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let config = Config::load_from(Some(PathBuf::from("/nonexistent/path/config.toml")));
        // Should return defaults without panicking
        assert!(config.persistence.history);
    }

    #[test]
    fn test_load_invalid_toml() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "invalid toml content {{{{{{").unwrap();

        let config = Config::load_from(Some(temp_file.path().to_path_buf()));
        // Should return defaults on parse error
        assert!(config.persistence.history);
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut config = Config::default();
        config.persistence.history = false;
        config.preferences.theme = "custom".to_string();
        config.transfer.max_concurrent_sends = 10;

        // Manual save to temp location
        let toml_string = toml::to_string_pretty(&config).unwrap();
        fs::write(&config_path, toml_string).unwrap();

        // Load it back
        let loaded = Config::load_from(Some(config_path));
        assert!(!loaded.persistence.history);
        assert_eq!(loaded.preferences.theme, "custom");
        assert_eq!(loaded.transfer.max_concurrent_sends, 10);
    }

    #[test]
    fn test_partial_config() {
        // Test that missing fields use defaults due to #[serde(default)]
        let toml_str = r#"
            [persistence]
            history = false
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(!config.persistence.history);
        assert_eq!(config.preferences.theme, "default"); // default value
        assert_eq!(config.transfer.max_concurrent_sends, 50); // default value
    }

    #[test]
    fn test_empty_file_uses_defaults() {
        let toml_str = "";
        let config: Config = toml::from_str(toml_str).unwrap();
        let default_config = Config::default();

        assert_eq!(
            config.persistence.history,
            default_config.persistence.history
        );
        assert_eq!(config.preferences.theme, default_config.preferences.theme);
        assert_eq!(
            config.preferences.key_preset,
            default_config.preferences.key_preset
        );
        assert_eq!(
            config.preferences.receive_dir,
            default_config.preferences.receive_dir
        );
        assert_eq!(
            config.transfer.max_concurrent_sends,
            default_config.transfer.max_concurrent_sends
        );
        assert_eq!(
            config.transfer.max_concurrent_receives,
            default_config.transfer.max_concurrent_receives
        );
    }

    #[test]
    fn test_unknown_fields_ignored() {
        let toml_str = r#"
            [persistence]
            history = false
            future_feature = true

            [preferences]
            theme = "dark"

            [completely_unknown_section]
            some_field = "value"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(!config.persistence.history);
        assert_eq!(config.preferences.theme, "dark");
        assert_eq!(config.preferences.key_preset, "arrows"); // default
    }

    #[test]
    fn test_type_mismatch_falls_back() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
            [persistence]
            history = "yes"
        "#
        )
        .unwrap();

        let config = Config::load_from(Some(temp_file.path().to_path_buf()));
        // Should return defaults without panicking
        assert!(config.persistence.history);
    }

    #[test]
    fn test_nested_partial_config() {
        let toml_str = r#"
            [transfer]
            max_concurrent_sends = 100
            max_concurrent_receives = 75
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.transfer.max_concurrent_sends, 100);
        assert_eq!(config.transfer.max_concurrent_receives, 75);
        assert!(config.persistence.history); // default
        assert_eq!(config.preferences.theme, "default"); // default
    }

    #[test]
    fn test_path_with_special_chars() {
        let toml_str = r#"
            [preferences]
            receive_dir = "/home/user/My Documents/downloads"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.preferences.receive_dir,
            Some(PathBuf::from("/home/user/My Documents/downloads"))
        );
    }
}
