//! Export components for page-level and full-report exports.
//!
//! # Usage
//!
//! ```text
//! use crate::ui::components::export::{
//!     ExportToolbar, ExportFormat, PageExport, PageExportMetadata,
//!     to_json, to_csv, to_markdown
//! };
//!
//! // In the view
//! let mut export_format = ExportFormat::Json;
//! let mut filtered_only = false;
//!
//! let response = ExportToolbar::new("spells_export", &mut export_format, &mut filtered_only)
//!     .has_filters(state.filters.has_active_filters())
//!     .show(ui);
//!
//! if response.export_clicked {
//!     let data = if filtered_only { &filtered_spells } else { &all_spells };
//!
//!     let export = PageExport::new(
//!         PageExportMetadata::new("Spells")
//!             .with_counts(all_spells.len(), filtered_spells.len()),
//!         data,
//!     );
//!
//!     let content = match export_format {
//!         ExportFormat::Json => to_json(&export).unwrap(),
//!         ExportFormat::Csv => to_csv(&headers, &rows),
//!         ExportFormat::Markdown => to_markdown(&headers, &rows),
//!     };
//!
//!     // Save or copy content
//! }
//! ```

pub mod formats;
pub mod page_export;
pub mod toolbar;

pub use formats::{ExportFormat, to_json, to_csv, to_markdown};
pub use page_export::{PageExport, PageExportMetadata};
pub use toolbar::ExportToolbar;
