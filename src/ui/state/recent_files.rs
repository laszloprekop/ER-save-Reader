//! Recent files management for the landing page.
//!
//! Tracks recently opened save files and persists them to disk.

use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Maximum number of recent files to track
const MAX_RECENT_FILES: usize = 10;

/// A recently opened save file
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecentFile {
    /// Path to the save file
    pub path: PathBuf,
    /// When the file was last opened
    pub last_opened: DateTime<Utc>,
    /// Character names found in the save (for preview)
    pub character_names: Vec<String>,
}

impl RecentFile {
    /// Create a new recent file entry
    pub fn new(path: PathBuf, character_names: Vec<String>) -> Self {
        Self {
            path,
            last_opened: Utc::now(),
            character_names,
        }
    }

    /// Get a display-friendly filename
    pub fn display_name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }
}

/// Manages the list of recently opened files
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RecentFilesManager {
    files: Vec<RecentFile>,
}

impl RecentFilesManager {
    /// Create a new empty manager
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    /// Get the config file path
    fn config_path() -> PathBuf {
        let mut path = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        path.push(".er-save-reader");
        path.push("config.json");
        path
    }

    /// Load recent files from disk
    pub fn load() -> Self {
        let path = Self::config_path();

        if !path.exists() {
            return Self::new();
        }

        match fs::read_to_string(&path) {
            Ok(content) => {
                serde_json::from_str(&content).unwrap_or_else(|_| Self::new())
            }
            Err(_) => Self::new(),
        }
    }

    /// Save recent files to disk
    pub fn save(&self) {
        let path = Self::config_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, content);
        }
    }

    /// Add a file to the recent list (or update if exists)
    pub fn add(&mut self, path: &std::path::Path, character_names: &[String]) {
        // Remove existing entry for this path
        self.files.retain(|f| f.path != path);

        // Add new entry at the front
        self.files.insert(
            0,
            RecentFile::new(path.to_path_buf(), character_names.to_vec()),
        );

        // Trim to max size
        self.files.truncate(MAX_RECENT_FILES);

        // Persist to disk
        self.save();
    }

    /// Get the list of recent files
    pub fn get_recent(&self) -> &[RecentFile] {
        &self.files
    }

    /// Remove files that no longer exist on disk
    pub fn prune_missing(&mut self) {
        let before_count = self.files.len();
        self.files.retain(|f| f.path.exists());

        // Save if we removed any
        if self.files.len() != before_count {
            self.save();
        }
    }

    /// Check if there are any recent files
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}
