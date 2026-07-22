//! UI state persistence to disk.

use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};
use super::view_state::PageState;

/// User preferences that persist across sessions
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UiPreferences {
    /// Last used directory for file dialogs
    pub last_directory: Option<String>,
    /// Window position (x, y)
    pub window_position: Option<(f32, f32)>,
    /// Window size (width, height)
    pub window_size: Option<(f32, f32)>,
    /// Last selected slot index
    pub last_slot_index: Option<usize>,
}

/// Root UI state container
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UiState {
    /// Version of the state format (for migrations)
    pub version: u32,
    /// Per-page state
    pub pages: HashMap<String, PageState>,
    /// User preferences
    pub preferences: UiPreferences,
    /// Dirty flag (not serialized)
    #[serde(skip)]
    pub dirty: bool,
    /// Last save time (not serialized, uses default on load)
    #[serde(skip)]
    last_save_ms: u64,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            pages: HashMap::new(),
            preferences: UiPreferences::default(),
            dirty: false,
            last_save_ms: 0,
        }
    }
}

/// Current state format version
const STATE_VERSION: u32 = 1;

/// Debounce interval for saving (5 seconds)
const SAVE_DEBOUNCE_MS: u128 = 5000;

impl UiState {
    /// Create a new UI state
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the state file path
    pub fn state_file_path() -> PathBuf {
        let mut path = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        path.push(".er-save-reader");
        path.push("ui_state.json");
        path
    }

    /// Load state from disk
    pub fn load() -> Self {
        let path = Self::state_file_path();

        if !path.exists() {
            return Self::new();
        }

        match fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<UiState>(&content) {
                    Ok(mut state) => {
                        state.dirty = false;
                        state.last_save_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0);
                        state
                    }
                    Err(_) => Self::new(),
                }
            }
            Err(_) => Self::new(),
        }
    }

    /// Save state to disk
    pub fn save(&mut self) -> Result<(), std::io::Error> {
        let path = Self::state_file_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;

        self.dirty = false;
        self.last_save_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Ok(())
    }

    /// Mark state as dirty (needs saving)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Check if enough time has passed for a debounced save
    pub fn should_save(&self) -> bool {
        if !self.dirty {
            return false;
        }
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        now_ms.saturating_sub(self.last_save_ms) >= SAVE_DEBOUNCE_MS as u64
    }

    /// Save if dirty and debounce time has passed
    pub fn save_if_needed(&mut self) {
        if self.should_save() {
            let _ = self.save();
        }
    }

    /// Force save (for app exit)
    pub fn force_save(&mut self) {
        if self.dirty {
            let _ = self.save();
        }
    }

    /// Get or create page state
    pub fn get_page(&mut self, page_id: &str) -> &mut PageState {
        if !self.pages.contains_key(page_id) {
            self.pages.insert(page_id.to_string(), PageState::new());
        }
        self.pages.get_mut(page_id).unwrap()
    }

    /// Get page state immutably (if exists)
    pub fn get_page_ref(&self, page_id: &str) -> Option<&PageState> {
        self.pages.get(page_id)
    }
}
