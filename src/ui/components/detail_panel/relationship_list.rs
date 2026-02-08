//! Relationship list component for detail panel.

use eframe::egui::{Ui, RichText};
use super::panel::DetailPanelAction;
use crate::db::entity_relationships::{Relationship, RelationType};
use crate::ui::components::filter::fuzzy_match_default;
use crate::ui::tokens::{colors, spacing};

/// A single relationship item that can be clicked.
#[derive(Debug, Clone)]
pub struct RelationshipItem {
    /// Display label.
    pub label: String,
    /// Optional secondary text (e.g., region, quantity).
    pub secondary: Option<String>,
    /// Action to trigger when clicked.
    pub action: DetailPanelAction,
}

impl RelationshipItem {
    pub fn new(label: impl Into<String>, action: DetailPanelAction) -> Self {
        Self {
            label: label.into(),
            secondary: None,
            action,
        }
    }

    pub fn with_secondary(mut self, secondary: impl Into<String>) -> Self {
        self.secondary = Some(secondary.into());
        self
    }
}

/// A section of related items.
#[derive(Debug, Clone)]
pub struct RelationshipSection {
    /// Section title.
    pub title: String,
    /// Items in this section.
    pub items: Vec<RelationshipItem>,
    /// Whether this section is expanded.
    pub expanded: bool,
}

impl RelationshipSection {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
            expanded: true,
        }
    }

    pub fn with_items(mut self, items: Vec<RelationshipItem>) -> Self {
        self.items = items;
        self
    }

    pub fn add_item(&mut self, item: RelationshipItem) {
        self.items.push(item);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Render a relationship section and return action if clicked.
pub fn relationship_section(
    ui: &mut Ui,
    section: &RelationshipSection,
) -> Option<DetailPanelAction> {
    relationship_section_filtered(ui, section, "")
}

/// Render a relationship section with optional search filtering and return action if clicked.
pub fn relationship_section_filtered(
    ui: &mut Ui,
    section: &RelationshipSection,
    search_query: &str,
) -> Option<DetailPanelAction> {
    if section.items.is_empty() {
        return None;
    }

    // Filter items based on search query
    let filtered_items: Vec<&RelationshipItem> = if search_query.is_empty() {
        section.items.iter().collect()
    } else {
        section.items.iter()
            .filter(|item| {
                fuzzy_match_default(&item.label, search_query)
                    || item.secondary.as_ref().map(|s| fuzzy_match_default(s, search_query)).unwrap_or(false)
            })
            .collect()
    };

    // Don't show empty sections after filtering
    if filtered_items.is_empty() {
        return None;
    }

    let mut action = None;

    ui.group(|ui| {
        // Section header with filtered count
        ui.horizontal(|ui| {
            ui.label(RichText::new(&section.title).strong().size(13.0));
            if search_query.is_empty() {
                ui.label(
                    RichText::new(format!("({})", section.items.len()))
                        .small()
                        .color(colors::TEXT_DISABLED),
                );
            } else {
                ui.label(
                    RichText::new(format!("({}/{})", filtered_items.len(), section.items.len()))
                        .small()
                        .color(colors::ACCENT_PRIMARY),
                );
            }
        });

        ui.add_space(spacing::XS);

        // Items
        for item in filtered_items {
            ui.horizontal(|ui| {
                ui.add_space(spacing::SM);

                // Highlight matching text if searching
                if !search_query.is_empty() {
                    let label_lower = item.label.to_lowercase();
                    let query_lower = search_query.to_lowercase();

                    if label_lower.contains(&query_lower) {
                        // Create a highlighted link
                        if ui.link(RichText::new(&item.label).color(colors::WARNING)).clicked() {
                            action = Some(item.action.clone());
                        }
                    } else if ui.link(&item.label).clicked() {
                        action = Some(item.action.clone());
                    }
                } else if ui.link(&item.label).clicked() {
                    action = Some(item.action.clone());
                }

                if let Some(secondary) = &item.secondary {
                    ui.label(RichText::new(secondary).small().color(colors::TEXT_SECONDARY));
                }
            });
        }
    });

    ui.add_space(spacing::SM);

    action
}

/// Build a MapGenie "External Links" section from a MapGenie location ID.
pub fn mapgenie_section(mapgenie_id: &str) -> RelationshipSection {
    let url = format!(
        "https://mapgenie.io/elden-ring/maps/the-lands-between?locationIds={}",
        mapgenie_id
    );
    RelationshipSection::new("External Links").with_items(vec![
        RelationshipItem::new(
            format!("View on MapGenie ({})", mapgenie_id),
            DetailPanelAction::OpenExternalUrl { url: url.clone() },
        )
        .with_secondary(url),
    ])
}

/// Build a RelationshipSection from a slice of Relationships, filtering by type.
///
/// Returns `None` if no relationships match the given type.
/// The `action_fn` maps each Relationship to the appropriate DetailPanelAction.
pub fn section_from_relationships(
    title: &str,
    relationships: &[Relationship],
    rel_type: RelationType,
    action_fn: impl Fn(&Relationship) -> DetailPanelAction,
) -> Option<RelationshipSection> {
    let items: Vec<_> = relationships
        .iter()
        .filter(|r| r.rel_type == rel_type)
        .map(|r| {
            let mut item = RelationshipItem::new(r.label.to_string(), action_fn(r));
            if let Some(secondary) = &r.secondary {
                item = item.with_secondary(secondary.clone());
            }
            item
        })
        .collect();

    if items.is_empty() {
        None
    } else {
        Some(RelationshipSection::new(title).with_items(items))
    }
}
