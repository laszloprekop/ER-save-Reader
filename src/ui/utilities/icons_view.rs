//! Icomoon font glyph reference grid for Elden Map marker icons.
//!
//! Displays all icomoon glyphs with their MapGenie category names,
//! clickable to copy glyph properties for easy referencing.

use crate::ui::tokens::colors;
use eframe::egui::{
    self, Color32, FontFamily, FontId, RichText, Rounding, Sense, Stroke, Ui, Vec2,
};

/// A single icomoon icon entry
struct IconEntry {
    /// MapGenie category name (snake_case)
    name: &'static str,
    /// Unicode codepoint
    codepoint: char,
    /// Unicode hex string (e.g. "e900")
    unicode_hex: &'static str,
}

/// Display name: convert snake_case to Title Case
fn display_name(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    format!("{}{}", upper, chars.collect::<String>())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// All 96 icomoon icons from elden-ring-icons.css, ordered by unicode codepoint
const ICONS: &[IconEntry] = &[
    IconEntry {
        name: "whetblade",
        codepoint: '\u{e900}',
        unicode_hex: "e900",
    },
    IconEntry {
        name: "weapon",
        codepoint: '\u{e901}',
        unicode_hex: "e901",
    },
    IconEntry {
        name: "wandering_mausoleum",
        codepoint: '\u{e902}',
        unicode_hex: "e902",
    },
    IconEntry {
        name: "trinas_lilly",
        codepoint: '\u{e903}',
        unicode_hex: "e903",
    },
    IconEntry {
        name: "transition",
        codepoint: '\u{e904}',
        unicode_hex: "e904",
    },
    IconEntry {
        name: "trainer",
        codepoint: '\u{e905}',
        unicode_hex: "e905",
    },
    IconEntry {
        name: "tool",
        codepoint: '\u{e906}',
        unicode_hex: "e906",
    },
    IconEntry {
        name: "teardrop_scarab",
        codepoint: '\u{e907}',
        unicode_hex: "e907",
    },
    IconEntry {
        name: "talisman_pouch",
        codepoint: '\u{e908}',
        unicode_hex: "e908",
    },
    IconEntry {
        name: "talisman",
        codepoint: '\u{e909}',
        unicode_hex: "e909",
    },
    IconEntry {
        name: "summoning_sigil",
        codepoint: '\u{e90a}',
        unicode_hex: "e90a",
    },
    IconEntry {
        name: "stonesword_key",
        codepoint: '\u{e90b}',
        unicode_hex: "e90b",
    },
    IconEntry {
        name: "stone_cairn",
        codepoint: '\u{e90c}',
        unicode_hex: "e90c",
    },
    IconEntry {
        name: "stake_of_marika",
        codepoint: '\u{e90d}',
        unicode_hex: "e90d",
    },
    IconEntry {
        name: "spiritspring_jump",
        codepoint: '\u{e90e}',
        unicode_hex: "e90e",
    },
    IconEntry {
        name: "spirit_ashes",
        codepoint: '\u{e90f}',
        unicode_hex: "e90f",
    },
    IconEntry {
        name: "spellbook",
        codepoint: '\u{e910}',
        unicode_hex: "e910",
    },
    IconEntry {
        name: "sorcery",
        codepoint: '\u{e911}',
        unicode_hex: "e911",
    },
    IconEntry {
        name: "somber_smithing_stone",
        codepoint: '\u{e912}',
        unicode_hex: "e912",
    },
    IconEntry {
        name: "smithing_table",
        codepoint: '\u{e913}',
        unicode_hex: "e913",
    },
    IconEntry {
        name: "smithing_stone",
        codepoint: '\u{e914}',
        unicode_hex: "e914",
    },
    IconEntry {
        name: "slumbering_egg",
        codepoint: '\u{e915}',
        unicode_hex: "e915",
    },
    IconEntry {
        name: "site_of_grace",
        codepoint: '\u{e916}',
        unicode_hex: "e916",
    },
    IconEntry {
        name: "shield",
        codepoint: '\u{e917}',
        unicode_hex: "e917",
    },
    IconEntry {
        name: "scadutree_fragment",
        codepoint: '\u{e918}',
        unicode_hex: "e918",
    },
    IconEntry {
        name: "sacred_tear",
        codepoint: '\u{e919}',
        unicode_hex: "e919",
    },
    IconEntry {
        name: "sacramental_bud",
        codepoint: '\u{e91a}',
        unicode_hex: "e91a",
    },
    IconEntry {
        name: "rune_arc",
        codepoint: '\u{e91b}',
        unicode_hex: "e91b",
    },
    IconEntry {
        name: "ruin_fragment",
        codepoint: '\u{e91c}',
        unicode_hex: "e91c",
    },
    IconEntry {
        name: "ritual_pot",
        codepoint: '\u{e91d}',
        unicode_hex: "e91d",
    },
    IconEntry {
        name: "revered_spirit_ash",
        codepoint: '\u{e91e}',
        unicode_hex: "e91e",
    },
    IconEntry {
        name: "remembrance",
        codepoint: '\u{e91f}',
        unicode_hex: "e91f",
    },
    IconEntry {
        name: "rebirth_monument",
        codepoint: '\u{e920}',
        unicode_hex: "e920",
    },
    IconEntry {
        name: "quest",
        codepoint: '\u{e921}',
        unicode_hex: "e921",
    },
    IconEntry {
        name: "puzzle",
        codepoint: '\u{e922}',
        unicode_hex: "e922",
    },
    IconEntry {
        name: "portal",
        codepoint: '\u{e923}',
        unicode_hex: "e923",
    },
    IconEntry {
        name: "perfume_bottle",
        codepoint: '\u{e924}',
        unicode_hex: "e924",
    },
    IconEntry {
        name: "painting",
        codepoint: '\u{e925}',
        unicode_hex: "e925",
    },
    IconEntry {
        name: "nascient_butterfly",
        codepoint: '\u{e926}',
        unicode_hex: "e926",
    },
    IconEntry {
        name: "multiplayer_item",
        codepoint: '\u{e927}',
        unicode_hex: "e927",
    },
    IconEntry {
        name: "miscellaneous",
        codepoint: '\u{e928}',
        unicode_hex: "e928",
    },
    IconEntry {
        name: "miquellas_lilly",
        codepoint: '\u{e929}',
        unicode_hex: "e929",
    },
    IconEntry {
        name: "miquellas_cross",
        codepoint: '\u{e92a}',
        unicode_hex: "e92a",
    },
    IconEntry {
        name: "minor_erdtree",
        codepoint: '\u{e92b}',
        unicode_hex: "e92b",
    },
    IconEntry {
        name: "mimics_tear",
        codepoint: '\u{e92c}',
        unicode_hex: "e92c",
    },
    IconEntry {
        name: "merchant",
        codepoint: '\u{e92d}',
        unicode_hex: "e92d",
    },
    IconEntry {
        name: "memory_stone",
        codepoint: '\u{e92e}',
        unicode_hex: "e92e",
    },
    IconEntry {
        name: "martyr_effigy",
        codepoint: '\u{e92f}',
        unicode_hex: "e92f",
    },
    IconEntry {
        name: "map_fragment",
        codepoint: '\u{e930}',
        unicode_hex: "e930",
    },
    IconEntry {
        name: "lore",
        codepoint: '\u{e931}',
        unicode_hex: "e931",
    },
    IconEntry {
        name: "location",
        codepoint: '\u{e932}',
        unicode_hex: "e932",
    },
    IconEntry {
        name: "legacy_dungeon",
        codepoint: '\u{e933}',
        unicode_hex: "e933",
    },
    IconEntry {
        name: "landmark",
        codepoint: '\u{e934}',
        unicode_hex: "e934",
    },
    IconEntry {
        name: "key_item",
        codepoint: '\u{e935}',
        unicode_hex: "e935",
    },
    IconEntry {
        name: "item",
        codepoint: '\u{e936}',
        unicode_hex: "e936",
    },
    IconEntry {
        name: "invasion",
        codepoint: '\u{e937}',
        unicode_hex: "e937",
    },
    IconEntry {
        name: "incantation",
        codepoint: '\u{e938}',
        unicode_hex: "e938",
    },
    IconEntry {
        name: "imp_seal_statue",
        codepoint: '\u{e939}',
        unicode_hex: "e939",
    },
    IconEntry {
        name: "hidden_passage",
        codepoint: '\u{e93a}',
        unicode_hex: "e93a",
    },
    IconEntry {
        name: "guiding_statue",
        codepoint: '\u{e93b}',
        unicode_hex: "e93b",
    },
    IconEntry {
        name: "great_rune",
        codepoint: '\u{e93c}',
        unicode_hex: "e93c",
    },
    IconEntry {
        name: "great_glovewort",
        codepoint: '\u{e93d}',
        unicode_hex: "e93d",
    },
    IconEntry {
        name: "great_boss",
        codepoint: '\u{e93e}',
        unicode_hex: "e93e",
    },
    IconEntry {
        name: "golden_seed",
        codepoint: '\u{e93f}',
        unicode_hex: "e93f",
    },
    IconEntry {
        name: "golden_rune",
        codepoint: '\u{e940}',
        unicode_hex: "e940",
    },
    IconEntry {
        name: "glovewort",
        codepoint: '\u{e941}',
        unicode_hex: "e941",
    },
    IconEntry {
        name: "ghost_glovewort",
        codepoint: '\u{e942}',
        unicode_hex: "e942",
    },
    IconEntry {
        name: "ghost",
        codepoint: '\u{e943}',
        unicode_hex: "e943",
    },
    IconEntry {
        name: "gesture",
        codepoint: '\u{e944}',
        unicode_hex: "e944",
    },
    IconEntry {
        name: "evergaol",
        codepoint: '\u{e945}',
        unicode_hex: "e945",
    },
    IconEntry {
        name: "enemy",
        codepoint: '\u{e946}',
        unicode_hex: "e946",
    },
    IconEntry {
        name: "elite_enemy",
        codepoint: '\u{e947}',
        unicode_hex: "e947",
    },
    IconEntry {
        name: "elevator",
        codepoint: '\u{e948}',
        unicode_hex: "e948",
    },
    IconEntry {
        name: "easter_egg",
        codepoint: '\u{e949}',
        unicode_hex: "e949",
    },
    IconEntry {
        name: "dungeon",
        codepoint: '\u{e94a}',
        unicode_hex: "e94a",
    },
    IconEntry {
        name: "dragon_shrine",
        codepoint: '\u{e94b}',
        unicode_hex: "e94b",
    },
    IconEntry {
        name: "dragon_heart",
        codepoint: '\u{e94c}',
        unicode_hex: "e94c",
    },
    IconEntry {
        name: "divine_tower",
        codepoint: '\u{e94d}',
        unicode_hex: "e94d",
    },
    IconEntry {
        name: "demigod",
        codepoint: '\u{e94e}',
        unicode_hex: "e94e",
    },
    IconEntry {
        name: "deathroot",
        codepoint: '\u{e94f}',
        unicode_hex: "e94f",
    },
    IconEntry {
        name: "crystal_tear",
        codepoint: '\u{e950}',
        unicode_hex: "e950",
    },
    IconEntry {
        name: "creature",
        codepoint: '\u{e951}',
        unicode_hex: "e951",
    },
    IconEntry {
        name: "crafting_material",
        codepoint: '\u{e952}',
        unicode_hex: "e952",
    },
    IconEntry {
        name: "cracked_pot",
        codepoint: '\u{e953}',
        unicode_hex: "e953",
    },
    IconEntry {
        name: "cookbook",
        codepoint: '\u{e954}',
        unicode_hex: "e954",
    },
    IconEntry {
        name: "consumable",
        codepoint: '\u{e955}',
        unicode_hex: "e955",
    },
    IconEntry {
        name: "character",
        codepoint: '\u{e956}',
        unicode_hex: "e956",
    },
    IconEntry {
        name: "cerulean_scarab",
        codepoint: '\u{e957}',
        unicode_hex: "e957",
    },
    IconEntry {
        name: "boss",
        codepoint: '\u{e958}',
        unicode_hex: "e958",
    },
    IconEntry {
        name: "bolstering_material",
        codepoint: '\u{e959}',
        unicode_hex: "e959",
    },
    IconEntry {
        name: "birdseye_telescope",
        codepoint: '\u{e95a}',
        unicode_hex: "e95a",
    },
    IconEntry {
        name: "bell_bearing",
        codepoint: '\u{e95b}',
        unicode_hex: "e95b",
    },
    IconEntry {
        name: "ash_of_war",
        codepoint: '\u{e95c}',
        unicode_hex: "e95c",
    },
    IconEntry {
        name: "armor",
        codepoint: '\u{e95d}',
        unicode_hex: "e95d",
    },
    IconEntry {
        name: "ancient_smithing_stone",
        codepoint: '\u{e95e}',
        unicode_hex: "e95e",
    },
    IconEntry {
        name: "ammunition",
        codepoint: '\u{e95f}',
        unicode_hex: "e95f",
    },
];

const CELL_SIZE: f32 = 80.0;
const GLYPH_SIZE: f32 = 32.0;
const LABEL_SIZE: f32 = 8.0;
const COPIED_TOAST_DURATION: f64 = 1.5;

pub struct IconsViewState {
    /// Timestamp when last copy happened (for toast feedback)
    pub last_copied_time: f64,
    /// Name of the last copied icon
    pub last_copied_name: String,
    /// Search filter
    pub search: String,
}

impl Default for IconsViewState {
    fn default() -> Self {
        Self {
            last_copied_time: 0.0,
            last_copied_name: String::new(),
            search: String::new(),
        }
    }
}

pub fn icons_view(ui: &mut Ui, state: &mut IconsViewState) {
    let now = ui.input(|i| i.time);
    let icomoon_family = FontFamily::Name("Icomoon".into());

    // Header with search
    ui.horizontal(|ui| {
        ui.label(RichText::new("Elden Map Icon Font Reference").strong());
        ui.separator();
        ui.label(RichText::new(format!("{} glyphs", ICONS.len())).color(colors::TEXT_SECONDARY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Show copied toast
            if now - state.last_copied_time < COPIED_TOAST_DURATION
                && !state.last_copied_name.is_empty()
            {
                ui.label(
                    RichText::new(format!(
                        "{} Copied: {}",
                        egui_phosphor::regular::CHECK,
                        &state.last_copied_name
                    ))
                    .color(colors::STATUS_COLLECTED),
                );
            }
            let search_field = egui::TextEdit::singleline(&mut state.search)
                .hint_text(format!(
                    "{} Filter...",
                    egui_phosphor::regular::MAGNIFYING_GLASS
                ))
                .desired_width(150.0);
            ui.add(search_field);
        });
    });

    ui.separator();

    // Filter icons by search
    let filtered_icons: Vec<&IconEntry> = if state.search.is_empty() {
        ICONS.iter().collect()
    } else {
        let query = state.search.to_lowercase();
        ICONS
            .iter()
            .filter(|icon| {
                icon.name.contains(&query)
                    || display_name(icon.name).to_lowercase().contains(&query)
                    || icon.unicode_hex.contains(&query)
            })
            .collect()
    };

    // Scrollable grid
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let available_width = ui.available_width();
            let item_spacing = ui.spacing().item_spacing.x;
            let cols =
                (((available_width + item_spacing) / (CELL_SIZE + item_spacing)) as usize).max(1);

            ui.add_space(4.0);

            for row_icons in filtered_icons.chunks(cols) {
                ui.horizontal(|ui| {
                    for icon in row_icons {
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(CELL_SIZE, CELL_SIZE), Sense::click());

                        // Hover highlight
                        if response.hovered() {
                            ui.painter().rect_filled(
                                rect,
                                Rounding::same(4.0),
                                colors::CAT_SURFACE0,
                            );
                        }

                        // Glyph - center in the area above labels (~top 58px of 80px cell)
                        let glyph_pos = rect.center() - Vec2::new(0.0, 10.0);
                        ui.painter().text(
                            glyph_pos,
                            egui::Align2::CENTER_CENTER,
                            icon.codepoint.to_string(),
                            FontId::new(GLYPH_SIZE, icomoon_family.clone()),
                            Color32::WHITE,
                        );

                        // Label (category name)
                        let label_text = display_name(icon.name);
                        let label_pos = rect.center_bottom() - Vec2::new(0.0, 16.0);
                        ui.painter().text(
                            label_pos,
                            egui::Align2::CENTER_CENTER,
                            &label_text,
                            FontId::new(LABEL_SIZE, FontFamily::Proportional),
                            colors::TEXT_SECONDARY,
                        );

                        // Unicode hex below label
                        let hex_pos = rect.center_bottom() - Vec2::new(0.0, 6.0);
                        ui.painter().text(
                            hex_pos,
                            egui::Align2::CENTER_CENTER,
                            format!("\\u{{{}}}", icon.unicode_hex),
                            FontId::new(7.0, FontFamily::Monospace),
                            colors::TEXT_DISABLED,
                        );

                        // Click → copy to clipboard
                        if response.clicked() {
                            let clip = format!(
                                "{} | {} (\\u{{{}}})",
                                icon.codepoint, icon.name, icon.unicode_hex,
                            );
                            ui.ctx().copy_text(clip);
                            state.last_copied_time = now;
                            state.last_copied_name = icon.name.to_string();
                        }

                        // Tooltip on hover
                        if response.hovered() {
                            egui::show_tooltip(ui.ctx(), ui.layer_id(), response.id, |ui| {
                                ui.label(
                                    RichText::new(icon.codepoint.to_string())
                                        .font(FontId::new(48.0, icomoon_family.clone()))
                                        .color(Color32::WHITE),
                                );
                                ui.label(RichText::new(&label_text).strong());
                                ui.label(
                                    RichText::new(format!("Category: {}", icon.name))
                                        .color(colors::TEXT_SECONDARY),
                                );
                                ui.label(
                                    RichText::new(format!("Unicode: \\u{{{}}}", icon.unicode_hex))
                                        .font(FontId::new(11.0, FontFamily::Monospace))
                                        .color(colors::TEXT_SECONDARY),
                                );
                                ui.separator();
                                ui.label(
                                    RichText::new("Click to copy")
                                        .small()
                                        .color(colors::TEXT_DISABLED),
                                );
                            });
                        }

                        // Selected highlight (just copied this one)
                        if state.last_copied_name == icon.name
                            && now - state.last_copied_time < COPIED_TOAST_DURATION
                        {
                            ui.painter().rect_stroke(
                                rect.shrink(1.0),
                                Rounding::same(4.0),
                                Stroke::new(1.5, colors::STATUS_COLLECTED),
                            );
                        }
                    }
                });
            }
        });
}
