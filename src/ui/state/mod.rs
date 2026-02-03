//! UI state management and persistence.
//!
//! This module handles saving and loading UI state (filters, sort order,
//! scroll positions, etc.) between application sessions.
//!
//! # Usage
//!
//! In the main App struct:
//!
//! ```rust
//! use crate::ui::state::UiState;
//!
//! struct App {
//!     ui_state: UiState,
//!     // ...
//! }
//!
//! impl App {
//!     fn new() -> Self {
//!         Self {
//!             ui_state: UiState::load(),
//!             // ...
//!         }
//!     }
//! }
//!
//! impl eframe::App for App {
//!     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//!         // Access page state
//!         let page = self.ui_state.get_page("spells");
//!
//!         // Mark dirty when state changes
//!         if filter_changed {
//!             self.ui_state.mark_dirty();
//!         }
//!
//!         // Debounced save
//!         self.ui_state.save_if_needed();
//!     }
//!
//!     fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
//!         self.ui_state.force_save();
//!     }
//! }
//! ```

pub mod view_state;
pub mod persistence;
pub mod recent_files;

pub use view_state::PageState;
pub use persistence::{UiState, UiPreferences};
pub use recent_files::{RecentFile, RecentFilesManager};
