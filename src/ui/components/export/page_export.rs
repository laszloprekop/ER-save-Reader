//! Per-page export data structures.

use serde::Serialize;
use chrono::Local;

/// Metadata about an export
#[derive(Clone, Debug, Serialize)]
pub struct PageExportMetadata {
    /// Page/view name
    pub page_name: String,
    /// Export timestamp
    pub export_date: String,
    /// Character name (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_name: Option<String>,
    /// Slot index (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_index: Option<usize>,
    /// Total item count
    pub total_count: usize,
    /// Filtered item count
    pub filtered_count: usize,
}

impl PageExportMetadata {
    pub fn new(page_name: impl Into<String>) -> Self {
        Self {
            page_name: page_name.into(),
            export_date: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            character_name: None,
            slot_index: None,
            total_count: 0,
            filtered_count: 0,
        }
    }

    pub fn with_character(mut self, name: impl Into<String>, slot: usize) -> Self {
        self.character_name = Some(name.into());
        self.slot_index = Some(slot);
        self
    }

    pub fn with_counts(mut self, total: usize, filtered: usize) -> Self {
        self.total_count = total;
        self.filtered_count = filtered;
        self
    }
}

/// Description of an applied filter
#[derive(Clone, Debug, Serialize)]
pub struct FilterDescription {
    pub name: String,
    pub value: String,
}

impl FilterDescription {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// Summary statistics for the export
#[derive(Clone, Debug, Default, Serialize)]
pub struct PageSummary {
    /// Total items in the dataset
    pub total: usize,
    /// Items passing the filter
    pub filtered: usize,
    /// Items that are collected/discovered
    pub collected: usize,
    /// Items that are not collected
    pub not_collected: usize,
    /// Items with unverified status
    pub unverified: usize,
}

/// Complete page export structure
#[derive(Clone, Debug, Serialize)]
pub struct PageExport<T: Serialize> {
    /// Export metadata
    pub metadata: PageExportMetadata,
    /// Applied filters
    pub filters_applied: Vec<FilterDescription>,
    /// Summary statistics
    pub summary: PageSummary,
    /// The actual data
    pub data: T,
}

impl<T: Serialize> PageExport<T> {
    pub fn new(metadata: PageExportMetadata, data: T) -> Self {
        Self {
            metadata,
            filters_applied: Vec::new(),
            summary: PageSummary::default(),
            data,
        }
    }

    pub fn with_filters(mut self, filters: Vec<FilterDescription>) -> Self {
        self.filters_applied = filters;
        self
    }

    pub fn with_summary(mut self, summary: PageSummary) -> Self {
        self.summary = summary;
        self
    }
}
