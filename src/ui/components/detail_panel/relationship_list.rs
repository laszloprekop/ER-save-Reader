//! Relationship list component for detail panel.

use eframe::egui::{Ui, RichText};
use super::panel::DetailPanelAction;
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

/// Helper to create a "Found At" section for items.
pub fn found_at_section(locations: Vec<(u32, String, String)>) -> RelationshipSection {
    let items = locations
        .into_iter()
        .map(|(flag_id, item_name, region)| {
            RelationshipItem::new(item_name.clone(), DetailPanelAction::NavigateToPickup { flag_id, name: item_name })
                .with_secondary(region)
        })
        .collect();

    RelationshipSection::new("Found At").with_items(items)
}

/// Helper to create a "Sold By" section for items.
pub fn sold_by_section(merchants: Vec<(u32, String, u32)>) -> RelationshipSection {
    let items = merchants
        .into_iter()
        .map(|(shop_id, merchant_name, price)| {
            RelationshipItem::new(
                &merchant_name,
                DetailPanelAction::NavigateToItem {
                    category: "Good".to_string(),
                    id: shop_id,
                    name: merchant_name.clone(),
                },
            )
            .with_secondary(format!("{} runes", price))
        })
        .collect();

    RelationshipSection::new("Sold By").with_items(items)
}

/// Helper to create a "Dropped By" section for items.
pub fn dropped_by_section(enemies: Vec<(u32, String)>) -> RelationshipSection {
    let items = enemies
        .into_iter()
        .map(|(defeat_flag, boss_name)| {
            RelationshipItem::new(boss_name.clone(), DetailPanelAction::NavigateToBoss { defeat_flag, name: boss_name })
        })
        .collect();

    RelationshipSection::new("Dropped By").with_items(items)
}

/// Helper to create a "Nearby Graces" section.
pub fn nearby_graces_section(graces: Vec<(u32, String)>) -> RelationshipSection {
    let items = graces
        .into_iter()
        .map(|(event_flag, name)| {
            RelationshipItem::new(name.clone(), DetailPanelAction::NavigateToGrace { event_flag, name })
        })
        .collect();

    RelationshipSection::new("Nearby Graces").with_items(items)
}
