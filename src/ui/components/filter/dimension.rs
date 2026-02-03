//! Filter dimension types for the unified filter bar.

use serde::{Deserialize, Serialize};

/// Standard completion status filter values
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionStatus {
    #[default]
    All,
    AutoDetected,
    Manual,
    InPossession,
    Missing,
    Unverified,
    /// Collected/discovered items only
    Collected,
    /// Not collected/not discovered items only
    NotCollected,
}

impl CompletionStatus {
    pub fn label(&self) -> &'static str {
        match self {
            CompletionStatus::All => "All",
            CompletionStatus::AutoDetected => "Auto-detected",
            CompletionStatus::Manual => "Manual",
            CompletionStatus::InPossession => "In Possession",
            CompletionStatus::Missing => "Missing",
            CompletionStatus::Unverified => "Unverified",
            CompletionStatus::Collected => "Collected",
            CompletionStatus::NotCollected => "Not Collected",
        }
    }

    /// Get all variants for iteration
    pub fn all_variants() -> &'static [CompletionStatus] {
        &[
            CompletionStatus::All,
            CompletionStatus::Collected,
            CompletionStatus::NotCollected,
            CompletionStatus::Unverified,
        ]
    }

    /// Get variants with possession semantics
    pub fn possession_variants() -> &'static [CompletionStatus] {
        &[
            CompletionStatus::All,
            CompletionStatus::InPossession,
            CompletionStatus::Missing,
            CompletionStatus::Unverified,
        ]
    }
}

/// A dropdown option for category/region filters
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterOption {
    pub value: String,
    pub label: String,
}

impl FilterOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }

    /// Create an "All" option
    pub fn all() -> Self {
        Self {
            value: "All".to_string(),
            label: "All".to_string(),
        }
    }

    /// Create from a string (value = label)
    pub fn from_str(s: impl Into<String>) -> Self {
        let s = s.into();
        Self {
            value: s.clone(),
            label: s,
        }
    }
}
