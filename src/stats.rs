use chrono::{DateTime, Local, NaiveDate, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use thiserror::Error;
use tokio::fs;
use tracing::{debug, warn};

static PROJECT_DIRS: OnceLock<Option<ProjectDirs>> = OnceLock::new();

fn get_project_dirs() -> Option<&'static ProjectDirs> {
    PROJECT_DIRS
        .get_or_init(|| ProjectDirs::from("", "", "mbell"))
        .as_ref()
}

#[derive(Error, Debug)]
pub enum StatsError {
    #[error("Failed to determine data directory")]
    NoDataDir,
    #[error("Failed to read stats file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse stats file: {0}")]
    ParseError(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Stats {
    /// Total number of bells rung
    pub total_bells: u64,
    /// Number of unique days the bell has been active
    pub days_active: u64,
    /// Current consecutive day streak
    pub current_streak: u64,
    /// Longest consecutive day streak ever
    pub longest_streak: u64,
    /// Last time the bell was rung
    pub last_ring: Option<DateTime<Utc>>,
    /// Date of the last activity (for streak calculation)
    #[serde(default)]
    last_active_date: Option<NaiveDate>,
}

impl Stats {
    pub fn load() -> Result<Self, StatsError> {
        let path = Self::stats_path()?;
        let temp_path = path.with_extension("json.tmp");

        // Check for stale temp file from interrupted save and recover if possible
        if temp_path.exists() && !path.exists() {
            debug!("Found stale temp file, attempting recovery");
            if let Err(e) = std::fs::rename(&temp_path, &path) {
                warn!("Failed to recover from temp file: {}", e);
                // Clean up the temp file
                let _ = std::fs::remove_file(&temp_path);
            } else {
                debug!("Successfully recovered stats from temp file");
            }
        } else if temp_path.exists() {
            // Both exist, main file takes precedence - clean up stale temp
            debug!("Cleaning up stale temp file");
            let _ = std::fs::remove_file(&temp_path);
        }

        if !path.exists() {
            debug!("Stats file does not exist, creating default");
            return Ok(Stats::default());
        }

        let contents = std::fs::read_to_string(&path)?;
        let stats: Stats = serde_json::from_str(&contents)?;
        Ok(stats)
    }

    pub async fn save(&self) -> Result<(), StatsError> {
        let path = Self::stats_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Write atomically by writing to temp file first
        let temp_path = path.with_extension("json.tmp");
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&temp_path, &contents).await?;
        fs::rename(&temp_path, &path).await?;

        debug!("Stats saved successfully");
        Ok(())
    }

    pub fn stats_path() -> Result<PathBuf, StatsError> {
        get_project_dirs()
            .map(|dirs| dirs.data_dir().join("stats.json"))
            .ok_or(StatsError::NoDataDir)
    }

    pub async fn record_bell(&mut self) {
        let now = Utc::now();
        let today = Local::now().date_naive();

        self.total_bells += 1;
        self.last_ring = Some(now);

        // Update streak calculation
        if let Some(last_date) = self.last_active_date {
            let days_diff = (today - last_date).num_days();

            if days_diff == 0 {
                // Same day, no change to streak
            } else if days_diff == 1 {
                // Consecutive day
                self.current_streak += 1;
                self.days_active += 1;
            } else {
                // Streak broken
                self.current_streak = 1;
                self.days_active += 1;
            }
        } else {
            // First bell ever
            self.current_streak = 1;
            self.days_active = 1;
        }

        self.last_active_date = Some(today);

        // Update longest streak if current is longer
        if self.current_streak > self.longest_streak {
            self.longest_streak = self.current_streak;
        }

        if let Err(e) = self.save().await {
            warn!("Failed to save stats: {}", e);
        }
    }

    pub async fn reset(&mut self) -> Result<(), StatsError> {
        *self = Stats::default();
        self.save().await
    }

    pub fn display(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("Total bells:    {}\n", self.total_bells));
        output.push_str(&format!("Days active:    {}\n", self.days_active));
        output.push_str(&format!("Current streak: {} days\n", self.current_streak));
        output.push_str(&format!("Longest streak: {} days\n", self.longest_streak));

        if let Some(last) = self.last_ring {
            let local: DateTime<Local> = last.into();
            output.push_str(&format!(
                "Last ring:      {}",
                local.format("%Y-%m-%d %H:%M:%S")
            ));
        } else {
            output.push_str("Last ring:      Never");
        }

        output
    }
}
