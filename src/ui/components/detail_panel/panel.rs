//! Main detail panel component.

use eframe::egui::{self, SidePanel, RichText, ScrollArea, TextEdit};
use super::relationship_list::{RelationshipSection, relationship_section_filtered};
use crate::ui::components::legend::{entity_icons, nav_icons};
use crate::ui::tokens::{colors, spacing};

/// Entity selected for detail view.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectedEntity {
    /// An item (category, id, name).
    Item { category: String, id: u32, name: String },
    /// A site of grace (event_flag, name).
    Grace { event_flag: u32, name: String },
    /// A merchant item (shop_id, merchant_name, item_name).
    Merchant { shop_id: u32, merchant_name: String, item_name: String },
    /// A boss (defeat_flag, name).
    Boss { defeat_flag: u32, name: String },
    /// A world pickup (flag_id, item_name).
    Pickup { flag_id: u32, item_name: String },
    /// A quest chain (id, name, category).
    QuestChain { id: u32, name: String, category: String },
}

impl SelectedEntity {
    /// Get the display name for this entity.
    pub fn name(&self) -> &str {
        match self {
            Self::Item { name, .. } => name,
            Self::Grace { name, .. } => name,
            Self::Merchant { item_name, .. } => item_name,
            Self::Boss { name, .. } => name,
            Self::Pickup { item_name, .. } => item_name,
            Self::QuestChain { name, .. } => name,
        }
    }

    /// Get the entity type as a string.
    pub fn entity_type(&self) -> &'static str {
        match self {
            Self::Item { .. } => "Item",
            Self::Grace { .. } => "Grace",
            Self::Merchant { .. } => "Shop Item",
            Self::Boss { .. } => "Boss",
            Self::Pickup { .. } => "Pickup",
            Self::QuestChain { category, .. } => {
                // Return the category for quest chains
                match category.as_str() {
                    "NPC Questline" => "NPC Quest",
                    "Main Story" => "Main Quest",
                    _ => "Quest",
                }
            }
        }
    }

    /// Get the icon for this entity type.
    pub fn entity_icon(&self) -> &'static str {
        match self {
            Self::Item { category, .. } => {
                // Map item category to appropriate icon
                match category.as_str() {
                    "Weapon" => entity_icons::WEAPON,
                    "Protector" | "Armor" => entity_icons::ARMOR,
                    "Accessory" | "Talisman" => entity_icons::TALISMAN,
                    "Good" => entity_icons::KEY_ITEM,
                    "Gem" => entity_icons::ASH_OF_WAR,
                    _ => entity_icons::ITEM,
                }
            }
            Self::Grace { .. } => entity_icons::GRACE,
            Self::Merchant { .. } => entity_icons::MERCHANT,
            Self::Boss { .. } => entity_icons::BOSS,
            Self::Pickup { .. } => entity_icons::PICKUP,
            Self::QuestChain { .. } => entity_icons::QUEST,
        }
    }
}

/// State for the detail panel.
#[derive(Debug, Default)]
pub struct DetailPanelState {
    /// Whether the panel is currently open.
    pub open: bool,
    /// Current width of the panel.
    pub width: f32,
    /// Currently selected entity.
    pub selected: Option<SelectedEntity>,
    /// Relationship sections to display.
    pub relationship_sections: Vec<RelationshipSection>,
    /// Search query for filtering relationship items.
    pub search_query: String,
}

impl DetailPanelState {
    pub fn new() -> Self {
        Self {
            open: false,
            width: 300.0,
            selected: None,
            relationship_sections: Vec::new(),
            search_query: String::new(),
        }
    }

    /// Select an entity and open the panel.
    pub fn select(&mut self, entity: SelectedEntity) {
        self.selected = Some(entity);
        self.open = true;
        self.relationship_sections.clear();
        self.search_query.clear();
    }

    /// Select entity and provide relationship sections.
    pub fn select_with_relationships(
        &mut self,
        entity: SelectedEntity,
        relationships: Vec<RelationshipSection>,
    ) {
        self.selected = Some(entity);
        self.relationship_sections = relationships;
        self.open = true;
        self.search_query.clear();
    }

    /// Close the panel.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Toggle the panel.
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Clear selection.
    pub fn clear(&mut self) {
        self.selected = None;
        self.relationship_sections.clear();
        self.search_query.clear();
    }

    /// Check if search is active
    pub fn has_search(&self) -> bool {
        !self.search_query.is_empty()
    }
}

/// Action returned from detail panel interaction.
#[derive(Debug, Clone)]
pub enum DetailPanelAction {
    /// Navigate to a grace.
    NavigateToGrace { event_flag: u32, name: String },
    /// Navigate to a boss.
    NavigateToBoss { defeat_flag: u32, name: String },
    /// Navigate to an item.
    NavigateToItem { category: String, id: u32, name: String },
    /// Navigate to a pickup.
    NavigateToPickup { flag_id: u32, name: String },
    /// Navigate to a merchant shop entry.
    NavigateToMerchant { shop_id: u32, name: String },
    /// Open an external URL in the default browser.
    OpenExternalUrl { url: String },
    /// No action.
    None,
}

/// Render the detail panel.
///
/// Returns an action if a navigation link was clicked.
pub fn detail_panel(
    ctx: &egui::Context,
    state: &mut DetailPanelState,
) -> DetailPanelAction {
    if !state.open || state.selected.is_none() {
        return DetailPanelAction::None;
    }

    let mut action = DetailPanelAction::None;

    SidePanel::right("detail_panel")
        .resizable(true)
        .default_width(state.width)
        .min_width(200.0)
        .max_width(500.0)
        .show(ctx, |ui| {
            // Header with icon and entity type
            ui.horizontal(|ui| {
                if let Some(entity) = &state.selected {
                    // Entity type icon
                    ui.label(RichText::new(entity.entity_icon()).size(14.0).color(colors::TEXT_SECONDARY));
                    ui.add_space(spacing::XS);
                    ui.label(RichText::new(entity.entity_type()).small().color(colors::TEXT_SECONDARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(nav_icons::CLOSE).clicked() {
                            state.close();
                        }
                    });
                }
            });

            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                if let Some(entity) = &state.selected {
                    // Title
                    ui.label(RichText::new(entity.name()).heading());
                    ui.add_space(spacing::SM);

                    // Entity-specific details
                    match entity {
                        SelectedEntity::Item { category, id, .. } => {
                            ui.label(RichText::new(format!("Category: {}", category)).color(colors::TEXT_SECONDARY));
                            ui.label(RichText::new(format!("ID: {}", id)).color(colors::TEXT_SECONDARY));
                        }
                        SelectedEntity::Grace { event_flag, .. } => {
                            ui.label(RichText::new(format!("Event Flag: {}", event_flag)).color(colors::TEXT_SECONDARY));
                        }
                        SelectedEntity::Merchant { shop_id, merchant_name, .. } => {
                            ui.label(RichText::new(format!("Merchant: {}", merchant_name)).color(colors::TEXT_SECONDARY));
                            ui.label(RichText::new(format!("Shop ID: {}", shop_id)).color(colors::TEXT_SECONDARY));
                        }
                        SelectedEntity::Boss { defeat_flag, .. } => {
                            ui.label(RichText::new(format!("Defeat Flag: {}", defeat_flag)).color(colors::TEXT_SECONDARY));
                        }
                        SelectedEntity::Pickup { flag_id, .. } => {
                            ui.label(RichText::new(format!("Flag ID: {}", flag_id)).color(colors::TEXT_SECONDARY));
                        }
                        SelectedEntity::QuestChain { id, category, .. } => {
                            ui.label(RichText::new(format!("Category: {}", category)).color(colors::TEXT_SECONDARY));
                            ui.label(RichText::new(format!("Quest ID: {}", id)).color(colors::TEXT_SECONDARY));
                        }
                    }

                    ui.add_space(spacing::LG);

                    // Search box for filtering relationships (only show if there are sections)
                    if !state.relationship_sections.is_empty() {
                        let total_items: usize = state.relationship_sections.iter().map(|s| s.items.len()).sum();
                        if total_items > 5 {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(nav_icons::SEARCH).color(colors::TEXT_SECONDARY));
                                let response = ui.add(
                                    TextEdit::singleline(&mut state.search_query)
                                        .hint_text("Filter relationships...")
                                        .desired_width(ui.available_width() - 24.0)
                                );
                                if state.has_search() {
                                    if ui.small_button(nav_icons::CLOSE).clicked() {
                                        state.search_query.clear();
                                        response.request_focus();
                                    }
                                }
                            });
                            ui.add_space(spacing::SM);
                        }
                    }

                    // Relationship sections with optional filtering
                    let search_query = state.search_query.clone();
                    for section in &state.relationship_sections {
                        if let Some(clicked) = relationship_section_filtered(ui, section, &search_query) {
                            action = clicked;
                        }
                    }
                }
            });
        });

    action
}
