//! Filter bar state management.

use serde::{Deserialize, Serialize};
use super::dimension::CompletionStatus;

/// Persistent state for a filter bar
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FilterBarState {
    /// Completion status filter
    pub completion: CompletionStatus,
    /// Category filter (e.g., item type)
    pub category: String,
    /// Region/area filter
    pub region: String,
    /// Search query text
    pub search: String,
    /// Any additional custom filters
    #[serde(flatten)]
    pub custom: std::collections::HashMap<String, String>,
}

impl FilterBarState {
    /// Create a new filter bar state
    pub fn new() -> Self {
        Self {
            completion: CompletionStatus::All,
            category: "All".to_string(),
            region: "All".to_string(),
            search: String::new(),
            custom: std::collections::HashMap::new(),
        }
    }

    /// Check if any filters are active (non-default)
    pub fn has_active_filters(&self) -> bool {
        self.completion != CompletionStatus::All
            || self.category != "All"
            || self.region != "All"
            || !self.search.is_empty()
            || !self.custom.is_empty()
    }

    /// Reset all filters to defaults
    pub fn reset(&mut self) {
        self.completion = CompletionStatus::All;
        self.category = "All".to_string();
        self.region = "All".to_string();
        self.search.clear();
        self.custom.clear();
    }

    /// Set a custom filter value
    pub fn set_custom(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.custom.insert(key.into(), value.into());
    }

    /// Get a custom filter value
    pub fn get_custom(&self, key: &str) -> Option<&String> {
        self.custom.get(key)
    }

    /// Clear a custom filter
    pub fn clear_custom(&mut self, key: &str) {
        self.custom.remove(key);
    }
}
