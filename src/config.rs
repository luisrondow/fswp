//! User configuration and preferences

use crate::error::{FileTinderError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UserConfig {
    /// Whether the welcome dialog has been shown
    pub welcome_shown: bool,
}

impl UserConfig {
    /// Get the config file path (~/.config/fswp/config.json)
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("fswp").join("config.json"))
    }

    /// Load config from file, or create default if doesn't exist
    pub fn load() -> Result<Self> {
        let path = Self::config_path().ok_or_else(|| {
            FileTinderError::ConfigError("Could not determine config directory".to_string())
        })?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path).map_err(|e| {
            FileTinderError::ConfigError(format!("Failed to read config file: {}", e))
        })?;

        serde_json::from_str(&contents).map_err(|e| {
            FileTinderError::ConfigError(format!("Failed to parse config file: {}", e))
        })
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path().ok_or_else(|| {
            FileTinderError::ConfigError("Could not determine config directory".to_string())
        })?;

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                FileTinderError::ConfigError(format!("Failed to create config directory: {}", e))
            })?;
        }

        let contents = serde_json::to_string_pretty(self).map_err(|e| {
            FileTinderError::ConfigError(format!("Failed to serialize config: {}", e))
        })?;

        fs::write(&path, contents).map_err(|e| {
            FileTinderError::ConfigError(format!("Failed to write config file: {}", e))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = UserConfig::default();
        assert!(!config.welcome_shown);
    }

    #[test]
    fn test_config_serialization() {
        let config = UserConfig {
            welcome_shown: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: UserConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.welcome_shown, true);
    }
}
