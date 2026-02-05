//! Navigation stack for history management.
//!
//! Tracks navigation history for back/forward navigation in Database Explorer.

use crate::ui::menu::menu::Route;

/// Reference to a specific entity for cross-table navigation.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityReference {
    /// An item by (category, id)
    Item { category: String, id: u32 },
    /// A site of grace by event flag
    Grace { event_flag: u32 },
    /// A merchant shop entry
    Merchant { shop_id: u32 },
    /// A boss by defeat flag
    Boss { defeat_flag: u32 },
    /// A world pickup by flag ID
    Pickup { flag_id: u32 },
    /// An event flag
    EventFlag { flag_id: u32 },
}

/// An entry in the navigation history.
#[derive(Debug, Clone)]
pub struct NavigationEntry {
    /// The route being navigated to.
    pub route: Route,
    /// Optional entity reference for detail view.
    pub entity: Option<EntityReference>,
    /// Display label for breadcrumb.
    pub label: String,
}

impl NavigationEntry {
    pub fn new(route: Route, label: impl Into<String>) -> Self {
        Self {
            route,
            entity: None,
            label: label.into(),
        }
    }

    pub fn with_entity(mut self, entity: EntityReference) -> Self {
        self.entity = Some(entity);
        self
    }
}

/// Navigation stack for managing history.
#[derive(Debug, Default)]
pub struct NavigationStack {
    /// History entries (past + current).
    history: Vec<NavigationEntry>,
    /// Current position in history (0-indexed).
    current_index: usize,
}

impl NavigationStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new entry, clearing any forward history.
    pub fn push(&mut self, entry: NavigationEntry) {
        // Remove any forward history
        if self.current_index + 1 < self.history.len() {
            self.history.truncate(self.current_index + 1);
        }

        self.history.push(entry);
        self.current_index = self.history.len().saturating_sub(1);
    }

    /// Navigate back in history. Returns the entry if possible.
    pub fn back(&mut self) -> Option<&NavigationEntry> {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.history.get(self.current_index)
        } else {
            None
        }
    }

    /// Navigate forward in history. Returns the entry if possible.
    pub fn forward(&mut self) -> Option<&NavigationEntry> {
        if self.current_index + 1 < self.history.len() {
            self.current_index += 1;
            self.history.get(self.current_index)
        } else {
            None
        }
    }

    /// Get the current entry.
    pub fn current(&self) -> Option<&NavigationEntry> {
        self.history.get(self.current_index)
    }

    /// Check if we can go back.
    pub fn can_go_back(&self) -> bool {
        self.current_index > 0
    }

    /// Check if we can go forward.
    pub fn can_go_forward(&self) -> bool {
        self.current_index + 1 < self.history.len()
    }

    /// Get the last N entries for breadcrumb display.
    pub fn breadcrumb_entries(&self, max_count: usize) -> &[NavigationEntry] {
        let start = self.history.len().saturating_sub(max_count);
        let end = self.current_index + 1;
        &self.history[start.min(end)..end]
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.history.clear();
        self.current_index = 0;
    }

    /// Get the number of entries in history.
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Check if history is empty.
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_current() {
        let mut stack = NavigationStack::new();
        assert!(stack.current().is_none());

        stack.push(NavigationEntry::new(Route::DatabaseSelect, "Database"));
        assert_eq!(stack.current().unwrap().label, "Database");
    }

    #[test]
    fn test_back_and_forward() {
        let mut stack = NavigationStack::new();
        stack.push(NavigationEntry::new(Route::DatabaseSelect, "Database"));
        stack.push(NavigationEntry::new(Route::DatabaseSpells, "Spells"));
        stack.push(NavigationEntry::new(Route::DatabaseNpcs, "NPCs"));

        // Go back
        assert!(stack.can_go_back());
        let entry = stack.back().unwrap();
        assert_eq!(entry.label, "Spells");

        // Go forward
        assert!(stack.can_go_forward());
        let entry = stack.forward().unwrap();
        assert_eq!(entry.label, "NPCs");
    }

    #[test]
    fn test_push_clears_forward_history() {
        let mut stack = NavigationStack::new();
        stack.push(NavigationEntry::new(Route::DatabaseSelect, "A"));
        stack.push(NavigationEntry::new(Route::DatabaseSpells, "B"));
        stack.push(NavigationEntry::new(Route::DatabaseNpcs, "C"));

        // Go back twice
        stack.back();
        stack.back();
        assert_eq!(stack.current().unwrap().label, "A");

        // Push new entry - should clear B and C
        stack.push(NavigationEntry::new(Route::DatabaseShopItems, "D"));
        assert_eq!(stack.len(), 2);
        assert!(!stack.can_go_forward());
    }
}
