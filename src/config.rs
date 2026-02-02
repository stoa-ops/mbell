use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to determine config directory")]
    NoConfigDir,
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Invalid configuration: {0}")]
    ValidationError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Interval between bells in minutes
    pub interval: u64,
    /// Volume level (0-100)
    pub volume: u8,
    /// Log level: error, warn, info, debug, trace
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval: 10,
            volume: 70,
            log_level: "info".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path()?;

        if !path.exists() {
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let contents = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::ValidationError(e.to_string()))?;
        fs::write(&path, contents)?;
        Ok(())
    }

    pub fn config_path() -> Result<PathBuf, ConfigError> {
        ProjectDirs::from("", "", "mbell")
            .map(|dirs| dirs.config_dir().join("config.toml"))
            .ok_or(ConfigError::NoConfigDir)
    }

    pub fn config_dir() -> Result<PathBuf, ConfigError> {
        ProjectDirs::from("", "", "mbell")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .ok_or(ConfigError::NoConfigDir)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.interval == 0 {
            return Err(ConfigError::ValidationError(
                "interval must be greater than 0".to_string(),
            ));
        }

        if self.volume > 100 {
            return Err(ConfigError::ValidationError(
                "volume must be between 0 and 100".to_string(),
            ));
        }

        let valid_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_levels.contains(&self.log_level.to_lowercase().as_str()) {
            return Err(ConfigError::ValidationError(format!(
                "log_level must be one of: {}",
                valid_levels.join(", ")
            )));
        }

        Ok(())
    }

    pub fn default_config_contents() -> String {
        r#"# Interval between bells in minutes
interval = 10

# Volume level (0-100)
volume = 70

# Log level: error, warn, info, debug, trace
log_level = "info"
"#
        .to_string()
    }
}
