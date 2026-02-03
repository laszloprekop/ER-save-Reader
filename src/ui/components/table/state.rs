//! Table state management for sorting, selection, and scroll position.

use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use super::column::SortDirection;

/// Persistent state for a unified table
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TableState {
    /// Currently sorted column ID (if any)
    pub sort_column: Option<String>,
    /// Sort direction
    pub sort_direction: SortDirection,
    /// Selected row indices
    #[serde(skip)]
    pub selected_rows: HashSet<usize>,
    /// Last selected row index (for shift-click range selection)
    #[serde(skip)]
    pub last_selected: Option<usize>,
    /// Column widths by ID (for resized columns)
    pub column_widths: std::collections::HashMap<String, f32>,
    /// Scroll position (vertical offset)
    #[serde(skip)]
    pub scroll_offset: f32,
}

impl TableState {
    /// Create a new table state
    pub fn new() -> Self {
        Self::default()
    }

    /// Set initial sort
    pub fn with_sort(mut self, column: impl Into<String>, direction: SortDirection) -> Self {
        self.sort_column = Some(column.into());
        self.sort_direction = direction;
        self
    }

    /// Toggle sort on a column
    pub fn toggle_sort(&mut self, column_id: &str) {
        if self.sort_column.as_deref() == Some(column_id) {
            // Same column - toggle direction
            self.sort_direction = self.sort_direction.toggle();
        } else {
            // Different column - set ascending
            self.sort_column = Some(column_id.to_string());
            self.sort_direction = SortDirection::Ascending;
        }
    }

    /// Check if a column is the current sort column
    pub fn is_sorted_by(&self, column_id: &str) -> bool {
        self.sort_column.as_deref() == Some(column_id)
    }

    /// Select a single row, deselecting others
    pub fn select_row(&mut self, index: usize) {
        self.selected_rows.clear();
        self.selected_rows.insert(index);
        self.last_selected = Some(index);
    }

    /// Toggle selection of a row (Cmd+click)
    pub fn toggle_row(&mut self, index: usize) {
        if self.selected_rows.contains(&index) {
            self.selected_rows.remove(&index);
        } else {
            self.selected_rows.insert(index);
        }
        self.last_selected = Some(index);
    }

    /// Extend selection to a row (Shift+click)
    pub fn extend_selection(&mut self, index: usize) {
        if let Some(last) = self.last_selected {
            let (start, end) = if last <= index {
                (last, index)
            } else {
                (index, last)
            };
            for i in start..=end {
                self.selected_rows.insert(i);
            }
        } else {
            self.selected_rows.insert(index);
        }
        self.last_selected = Some(index);
    }

    /// Select all rows
    pub fn select_all(&mut self, row_count: usize) {
        self.selected_rows = (0..row_count).collect();
        self.last_selected = if row_count > 0 { Some(row_count - 1) } else { None };
    }

    /// Clear all selection
    pub fn clear_selection(&mut self) {
        self.selected_rows.clear();
        self.last_selected = None;
    }

    /// Check if a row is selected
    pub fn is_selected(&self, index: usize) -> bool {
        self.selected_rows.contains(&index)
    }

    /// Get the number of selected rows
    pub fn selection_count(&self) -> usize {
        self.selected_rows.len()
    }

    /// Check if any rows are selected
    pub fn has_selection(&self) -> bool {
        !self.selected_rows.is_empty()
    }
}
