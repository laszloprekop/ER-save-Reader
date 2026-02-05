//! State management for validation view.

use serde::Serialize;

/// Severity level for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Severity {
    /// Likely corruption or invalid data.
    Error,
    /// Inconsistency detected that may indicate issues.
    Warning,
    /// Informational only, not necessarily a problem.
    Info,
}

impl Severity {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Warning => "Warning",
            Self::Info => "Info",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Error => egui_phosphor::regular::X_CIRCLE,
            Self::Warning => egui_phosphor::regular::WARNING,
            Self::Info => egui_phosphor::regular::INFO,
        }
    }
}

/// A single validation issue.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    /// Category of the issue.
    pub category: String,
    /// Slot index where the issue was found.
    pub slot: usize,
    /// Brief description of the issue.
    pub message: String,
    /// Detailed information about the issue.
    pub details: String,
    /// Severity level.
    pub severity: Severity,
}

impl ValidationIssue {
    pub fn error(category: &str, slot: usize, message: &str, details: &str) -> Self {
        Self {
            category: category.to_string(),
            slot,
            message: message.to_string(),
            details: details.to_string(),
            severity: Severity::Error,
        }
    }

    pub fn warning(category: &str, slot: usize, message: &str, details: &str) -> Self {
        Self {
            category: category.to_string(),
            slot,
            message: message.to_string(),
            details: details.to_string(),
            severity: Severity::Warning,
        }
    }

    pub fn info(category: &str, slot: usize, message: &str, details: &str) -> Self {
        Self {
            category: category.to_string(),
            slot,
            message: message.to_string(),
            details: details.to_string(),
            severity: Severity::Info,
        }
    }
}

/// Complete validation report.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ValidationReport {
    /// Whether the save file is considered valid overall.
    pub is_valid: bool,
    /// List of errors found.
    pub errors: Vec<ValidationIssue>,
    /// List of warnings found.
    pub warnings: Vec<ValidationIssue>,
    /// Informational notes.
    pub info: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }

    pub fn add_error(&mut self, issue: ValidationIssue) {
        self.is_valid = false;
        self.errors.push(issue);
    }

    pub fn add_warning(&mut self, issue: ValidationIssue) {
        self.warnings.push(issue);
    }

    pub fn add_info(&mut self, issue: ValidationIssue) {
        self.info.push(issue);
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    pub fn info_count(&self) -> usize {
        self.info.len()
    }

    pub fn total_issues(&self) -> usize {
        self.errors.len() + self.warnings.len() + self.info.len()
    }

    pub fn all_issues(&self) -> Vec<&ValidationIssue> {
        let mut all = Vec::new();
        all.extend(self.errors.iter());
        all.extend(self.warnings.iter());
        all.extend(self.info.iter());
        all
    }
}

/// State for the validation view.
#[derive(Debug, Default)]
pub struct ValidationState {
    /// The validation report (None if not yet run).
    pub report: Option<ValidationReport>,
    /// Whether validation is in progress.
    pub running: bool,
    /// Filter by severity.
    pub show_errors: bool,
    pub show_warnings: bool,
    pub show_info: bool,
    /// Search filter.
    pub search_query: String,
    /// Expanded issue indices.
    pub expanded_issues: std::collections::HashSet<usize>,
}

impl ValidationState {
    pub fn new() -> Self {
        Self {
            report: None,
            running: false,
            show_errors: true,
            show_warnings: true,
            show_info: true,
            search_query: String::new(),
            expanded_issues: std::collections::HashSet::new(),
        }
    }

    pub fn reset(&mut self) {
        self.report = None;
        self.running = false;
        self.expanded_issues.clear();
    }

    pub fn set_report(&mut self, report: ValidationReport) {
        self.report = Some(report);
        self.running = false;
    }
}
