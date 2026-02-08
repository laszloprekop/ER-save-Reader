//! Detail panel component for showing entity details and relationships.
//!
//! Provides a right-side panel that displays detailed information about
//! selected entities and clickable links to related entities.

mod panel;
mod relationship_list;

pub use panel::{DetailPanelState, SelectedEntity, DetailPanelAction, detail_panel};
pub use relationship_list::{RelationshipItem, RelationshipSection, mapgenie_section, section_from_relationships};
