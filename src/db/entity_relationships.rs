//! Entity relationships for cross-table navigation.
//!
//! Provides relationship lookups between different entity types:
//! - Items <-> Merchants (SoldBy)
//! - Items <-> Pickups (FoundAt)
//! - Items <-> Bosses (DroppedBy)
//! - Graces <-> Bosses (Unlocks)

use std::collections::HashMap;
use once_cell::sync::Lazy;
use crate::db::merchants_data::MERCHANT_ITEMS;
use crate::db::pickup_data::WORLD_PICKUPS;

/// Entity type for relationship endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    Item,
    Grace,
    Boss,
    Pickup,
    Merchant,
}

/// Relationship type between entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    /// Item is sold by merchant
    SoldBy,
    /// Item is found at pickup location
    FoundAt,
    /// Item is dropped by boss/enemy
    DroppedBy,
    /// Grace/area is unlocked by defeating boss
    Unlocks,
}

/// A relationship entry.
#[derive(Debug, Clone)]
pub struct Relationship {
    pub target_type: EntityType,
    pub target_id: u32,
    pub rel_type: RelationType,
    pub label: &'static str,
    pub secondary: Option<String>,
}

/// Merchants that sell a given item (by item_id).
/// Key: item_id, Value: Vec<(shop_id, merchant_name, price)>
pub static ITEM_SOLD_BY: Lazy<HashMap<u32, Vec<(u32, &'static str, u32)>>> = Lazy::new(|| {
    let mut map: HashMap<u32, Vec<(u32, &'static str, u32)>> = HashMap::new();

    for (shop_id, item) in MERCHANT_ITEMS.iter() {
        map.entry(item.item_id)
            .or_default()
            .push((*shop_id, item.merchant_name, item.price));
    }

    map
});

/// World pickup locations for a given item (by item_id).
/// Key: item_id, Value: Vec<(event_flag, name, region, quantity)>
pub static ITEM_FOUND_AT: Lazy<HashMap<u32, Vec<(u32, &'static str, &'static str, u32)>>> = Lazy::new(|| {
    let mut map: HashMap<u32, Vec<(u32, &'static str, &'static str, u32)>> = HashMap::new();

    for pickup in WORLD_PICKUPS.iter() {
        // Only include pickups with valid event flags
        if pickup.event_flag > 0 {
            map.entry(pickup.item_id)
                .or_default()
                .push((pickup.event_flag, pickup.name, pickup.region, pickup.quantity));
        }
    }

    map
});

/// Boss drops - maps item_id to boss defeat flags
/// Key: item_id, Value: Vec<(defeat_flag, boss_name)>
/// Includes remembrances, great runes, weapons, ashes of war, talismans, and spirit ashes
pub static ITEM_DROPPED_BY: Lazy<HashMap<u32, Vec<(u32, &'static str)>>> = Lazy::new(|| {
    let mut map: HashMap<u32, Vec<(u32, &'static str)>> = HashMap::new();

    // ===== REMEMBRANCES (EquipParamGoods) =====
    map.insert(8150, vec![(10000800, "Godrick the Grafted")]);
    map.insert(8151, vec![(14000800, "Rennala, Queen of the Full Moon")]);
    map.insert(8152, vec![(16000800, "Rykard, Lord of Blasphemy")]);
    map.insert(8153, vec![(12010800, "Starscourge Radahn")]);
    map.insert(8154, vec![(15000800, "Malenia, Blade of Miquella")]);
    map.insert(8155, vec![(12050800, "Mohg, Lord of Blood")]);
    map.insert(8156, vec![(11000800, "Morgott, the Omen King")]);
    map.insert(8157, vec![(13000800, "Maliketh, the Black Blade")]);
    map.insert(8158, vec![(19000800, "Radagon / Elden Beast")]);
    map.insert(8159, vec![(11050800, "Hoarah Loux, Warrior")]);
    map.insert(8161, vec![(13000830, "Dragonlord Placidusax")]);
    map.insert(8162, vec![(12030850, "Lichdragon Fortissax")]);
    map.insert(8163, vec![(12020800, "Regal Ancestor Spirit")]);
    map.insert(8164, vec![(1052520800, "Fire Giant")]);
    map.insert(8165, vec![(12040800, "Astel, Naturalborn of the Void")]);

    // ===== GREAT RUNES (EquipParamGoods) =====
    map.insert(8101, vec![(10000800, "Godrick the Grafted")]);
    map.insert(8102, vec![(11000800, "Morgott, the Omen King")]);
    map.insert(8103, vec![(14000800, "Rennala, Queen of the Full Moon")]);
    map.insert(8104, vec![(16000800, "Rykard, Lord of Blasphemy")]);
    map.insert(8105, vec![(12010800, "Starscourge Radahn")]);
    map.insert(8106, vec![(15000800, "Malenia, Blade of Miquella")]);
    map.insert(8107, vec![(12050800, "Mohg, Lord of Blood")]);

    // ===== WEAPONS (EquipParamWeapon) =====
    // Margit - Margit's Shackle (key item from Patches, but Margit drops nothing)
    // Godrick - drops remembrance only (weapons from remembrance exchange)

    // Leonine Misbegotten (Castle Morne) - Grafted Blade Greatsword
    map.insert(21100000, vec![(1043300800, "Leonine Misbegotten")]);

    // Bloodhound Knight Darriwil - Bloodhound's Fang
    map.insert(17020000, vec![(1044360800, "Bloodhound Knight Darriwil")]);

    // Cemetery Shade (Tombsward Catacombs) - Lhutel the Headless (spirit ash)
    map.insert(410000, vec![(30030800, "Cemetery Shade (Tombsward)")]);

    // Grave Warden Duelist (Murkwater Catacombs) - Battle Hammer
    map.insert(9000000, vec![(30000800, "Grave Warden Duelist")]);

    // Scaly Misbegotten (Morne Tunnel) - Rusted Anchor
    map.insert(17170000, vec![(32010800, "Scaly Misbegotten")]);

    // Crucible Knight (Stormhill Evergaol) - Aspects of the Crucible: Tail
    map.insert(4040, vec![(1042380800, "Crucible Knight (Stormhill)")]);

    // Crucible Knight Ordovis + Crucible Knight - Ordovis's Greatsword + Crucible Axe Set
    map.insert(3060000, vec![(30070800, "Crucible Knight Ordovis")]);
    map.insert(310200, vec![(30070800, "Crucible Knight Ordovis")]);  // Ordovis's Vortex (incantation)

    // Tree Sentinel - Golden Halberd
    map.insert(15110000, vec![(1042380800, "Tree Sentinel")]);

    // Night's Cavalry (Limgrave) - Ash of War: Repeating Thrust
    map.insert(22000200, vec![(1042370800, "Night's Cavalry (Agheel Lake)")]);

    // Flying Dragon Agheel - Dragon Heart
    map.insert(8000, vec![(1044350800, "Flying Dragon Agheel")]);

    // Erdtree Avatar (Minor Erdtree) - various rewards
    map.insert(11057, vec![(1045350800, "Erdtree Avatar (Weeping Peninsula)")]);  // Opaline Hardtear

    // Ancestor Spirit - Ancestral Follower Ashes
    map.insert(419000, vec![(12080800, "Ancestor Spirit")]);

    // Regal Ancestor Spirit - Remembrance + Winged Greathorn (from remembrance)

    // Mimic Tear - Larval Tear + Silver Tear Mask
    map.insert(1980, vec![(12070800, "Mimic Tear")]);

    // Valiant Gargoyles - Gargoyle's Greatsword + Twinblade
    map.insert(12040000, vec![(12020800, "Valiant Gargoyles")]);
    map.insert(12060000, vec![(12020800, "Valiant Gargoyles")]);

    // Magma Wyrm Makar - Magma Wyrm's Scalesword + Dragon Heart
    map.insert(21040000, vec![(39200800, "Magma Wyrm Makar")]);

    // Red Wolf of Radagon - Memory Stone
    map.insert(8010, vec![(14000850, "Red Wolf of Radagon")]);

    // Glintstone Dragon Smarag - Dragon Heart
    map.insert(8000, vec![(1034450800, "Glintstone Dragon Smarag")]);

    // Royal Knight Loretta (Caria Manor) - Loretta's Greatbow (sorcery)
    map.insert(4003, vec![(1035500800, "Royal Knight Loretta (Caria)")]);

    // Full-Grown Fallingstar Beast - Somber Smithing Stone [6] + Fallingstar Beast Jaw
    map.insert(21150000, vec![(1037530800, "Full-Grown Fallingstar Beast")]);

    // Commander O'Neil - Commander's Standard + Unalloyed Gold Needle
    map.insert(15140000, vec![(1049380800, "Commander O'Neil")]);

    // Decaying Ekzykes - Dragon Heart
    map.insert(8000, vec![(1048370800, "Decaying Ekzykes")]);

    // Commander Niall - Veteran's Prosthesis
    map.insert(1350, vec![(1051560800, "Commander Niall")]);

    // Tibia Mariner (multiple locations) - Deathroot + Skeletal Militiaman Ashes
    map.insert(3700, vec![(1034500800, "Tibia Mariner (Summonwater)")]);
    map.insert(3700, vec![(1038410800, "Tibia Mariner (Liurnia)")]);
    map.insert(3700, vec![(1040530800, "Tibia Mariner (Wyndham Ruins)")]);
    map.insert(3700, vec![(1051570800, "Tibia Mariner (Mountaintops)")]);

    // Godfrey, First Elden Lord (Golden Shade) - Talisman Pouch
    map.insert(8011, vec![(11000850, "Godfrey, First Elden Lord (Shade)")]);

    // Morgott drops - Morgott's Cursed Sword (from remembrance)

    // Mohg, the Omen (Subterranean Shunning-Grounds) - Bloodflame Talons (incantation)
    map.insert(4370, vec![(35000800, "Mohg, the Omen")]);

    // Godskin Duo - Smithing-Stone Miner's Bell Bearing [4]
    map.insert(8700, vec![(13000850, "Godskin Duo")]);

    // ===== TALISMANS (EquipParamAccessory) =====
    // Ancestor Spirit - Ancestral Spirit's Horn
    map.insert(1080, vec![(12080800, "Ancestor Spirit")]);

    // Mimic Tear - Silver Tear Mask
    map.insert(450000, vec![(12070800, "Mimic Tear")]);

    // Omenkiller + Miranda the Blighted Bloom - Omensmirk Mask
    map.insert(350100, vec![(32050800, "Omenkiller (Perfumer's Grotto)")]);

    // Spirit-Caller Snail (Spiritcaller Cave) - Godskin Swaddling Cloth
    map.insert(1200, vec![(31190800, "Spirit-Caller Snail")]);

    // Beast Clergyman / Maliketh - drops remembrance

    // ===== ASHES OF WAR (EquipParamGem) =====
    // Ancient Hero of Zamor (Weeping Evergaol) - Radagon's Scarseal (talisman)
    map.insert(1020, vec![(1042330800, "Ancient Hero of Zamor")]);

    // Night's Cavalry drops various Ashes
    map.insert(22000200, vec![(1042370800, "Night's Cavalry (Agheel Lake)")]);  // Repeating Thrust
    map.insert(22000400, vec![(1048380800, "Night's Cavalry (Caelid)")]);  // Poison Moth Flight
    map.insert(22000800, vec![(1037500800, "Night's Cavalry (Liurnia)")]);  // Ice Spear
    map.insert(22001400, vec![(1040510800, "Night's Cavalry (Altus)")]);  // Shared Order

    // Bell Bearing Hunter (various) - Bone Peddler's/Meat Peddler's/Medicine Peddler's Bell Bearing
    map.insert(8601, vec![(1035500800, "Bell Bearing Hunter (Warmaster's Shack)")]);

    // ===== SPIRIT ASHES (EquipParamGoods) =====
    // Spirit ashes are usually found in catacombs, not dropped by specific bosses
    // But some special ones come from quest rewards or specific enemies

    // Mimic Tear - Mimic Tear Ashes (chest, not drop)
    // Lhutel the Headless - from Cemetery Shade
    map.insert(410000, vec![(30030800, "Cemetery Shade (Tombsward)")]);

    // Black Knife Tiche - from Alecto, Black Knife Ringleader
    map.insert(424000, vec![(1050570800, "Alecto, Black Knife Ringleader")]);

    // Ancient Dragon Knight Kristoff - from Ancient Hero of Zamor (Sainted Hero's Grave)
    map.insert(417000, vec![(30190800, "Ancient Hero of Zamor (Sainted)")]);

    // Cleanrot Knight Finlay - from Cleanrot Knight (Elphael)
    map.insert(429000, vec![(15010800, "Cleanrot Knight Duo (Elphael)")]);

    map
});

/// Get relationships for an item.
pub fn get_item_relationships(item_id: u32) -> Vec<Relationship> {
    let mut relationships = Vec::new();

    // Check if sold by any merchants
    if let Some(merchants) = ITEM_SOLD_BY.get(&item_id) {
        for (shop_id, merchant_name, price) in merchants {
            relationships.push(Relationship {
                target_type: EntityType::Merchant,
                target_id: *shop_id,
                rel_type: RelationType::SoldBy,
                label: merchant_name,
                secondary: Some(format!("{} runes", price)),
            });
        }
    }

    // Check if found at any world pickup locations
    if let Some(pickups) = ITEM_FOUND_AT.get(&item_id) {
        for (event_flag, name, region, quantity) in pickups {
            let secondary = if *quantity > 1 {
                format!("{} (x{})", region, quantity)
            } else {
                region.to_string()
            };
            relationships.push(Relationship {
                target_type: EntityType::Pickup,
                target_id: *event_flag,
                rel_type: RelationType::FoundAt,
                label: name,
                secondary: Some(secondary),
            });
        }
    }

    // Check if dropped by any bosses
    if let Some(bosses) = ITEM_DROPPED_BY.get(&item_id) {
        for (defeat_flag, boss_name) in bosses {
            relationships.push(Relationship {
                target_type: EntityType::Boss,
                target_id: *defeat_flag,
                rel_type: RelationType::DroppedBy,
                label: boss_name,
                secondary: None,
            });
        }
    }

    relationships
}

/// Get relationships for a grace.
pub fn get_grace_relationships(_event_flag: u32) -> Vec<Relationship> {
    // TODO: Find nearby pickups, related bosses
    Vec::new()
}

/// Get relationships for a boss.
pub fn get_boss_relationships(_defeat_flag: u32) -> Vec<Relationship> {
    // TODO: Find drops, unlocked graces
    Vec::new()
}

/// Get relationships for a merchant item.
pub fn get_merchant_relationships(shop_id: u32) -> Vec<Relationship> {
    let mut relationships = Vec::new();

    // Find the item this shop entry sells
    if let Some(item) = MERCHANT_ITEMS.get(&shop_id) {
        relationships.push(Relationship {
            target_type: EntityType::Item,
            target_id: item.item_id,
            rel_type: RelationType::SoldBy,
            label: item.item_name,
            secondary: Some(item.equip_type.as_str().to_string()),
        });
    }

    relationships
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_sold_by_lookup() {
        // Smithing Stone [1] (ID: 10100) should be sold by multiple merchants
        let merchants = ITEM_SOLD_BY.get(&10100);
        assert!(merchants.is_some(), "Smithing Stone [1] should be sold by merchants");
    }

    #[test]
    fn test_item_found_at_lookup() {
        // Verify the ITEM_FOUND_AT map was built
        assert!(!ITEM_FOUND_AT.is_empty(), "ITEM_FOUND_AT should have entries");

        // Check that we have some items with pickup locations
        let total_items_with_pickups = ITEM_FOUND_AT.len();
        assert!(total_items_with_pickups > 100, "Should have many items with pickup locations");
    }

    #[test]
    fn test_get_item_relationships() {
        // Test getting relationships for an item that's sold by merchants
        let rels = get_item_relationships(10100); // Smithing Stone [1]
        // Verify it returns a valid (possibly empty) vec
        let _ = rels; // Just verify no panic
    }
}
