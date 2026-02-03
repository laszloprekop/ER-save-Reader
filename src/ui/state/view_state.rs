//! Per-page view state container.

use serde::{Deserialize, Serialize};
use crate::ui::components::filter::FilterBarState;
use crate::ui::components::table::TableState;

/// State container for a single page/view
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PageState {
    /// Filter bar state
    pub filters: FilterBarState,
    /// Table state (sort, selection, column widths)
    pub table: TableState,
    /// Custom state (page-specific data)
    #[serde(default)]
    pub custom: serde_json::Value,
}

impl PageState {
    /// Create a new page state
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a custom value
    pub fn set_custom<T: Serialize>(&mut self, key: &str, value: &T) {
        let obj = self.custom.as_object_mut();
        if let Some(obj) = obj {
            if let Ok(v) = serde_json::to_value(value) {
                obj.insert(key.to_string(), v);
            }
        } else {
            // Initialize as object if not already
            let mut map = serde_json::Map::new();
            if let Ok(v) = serde_json::to_value(value) {
                map.insert(key.to_string(), v);
            }
            self.custom = serde_json::Value::Object(map);
        }
    }

    /// Get a custom value
    pub fn get_custom<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.custom.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}
