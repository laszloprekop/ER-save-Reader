//! Entity relationships for cross-table navigation.
//!
//! Provides relationship lookups between different entity types:
//! - Items <-> Merchants (SoldBy)
//! - Items <-> Pickups (FoundAt)
//! - Items <-> Bosses (DroppedBy)
//! - Bosses <-> Items (Drops)
//! - Bosses <-> Graces (NearbyGrace)
//! - Graces <-> Bosses (NearbyBoss)

use std::collections::HashMap;
use once_cell::sync::Lazy;
use crate::db::merchants_data::MERCHANT_ITEMS;
use crate::db::pickup_data::WORLD_PICKUPS;
use crate::db::entity_relationships_data::{
    ITEM_DROPPED_BY, BOSS_DROP_INDEX, BOSS_DROPS,
    BOSS_NEARBY_GRACES, GRACE_NEARBY_BOSSES,
};
use crate::db::bosses_data::BOSSES_DATA;
use crate::db::graces_data::GRACES_DATA;

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
    /// Boss drops this item
    Drops,
    /// Boss has a nearby grace
    NearbyGrace,
    /// Grace has a nearby boss
    NearbyBoss,
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

/// (shop_id, merchant_name, price)
pub type SoldBy = (u32, &'static str, u32);

/// (event_flag, name, region, quantity)
pub type FoundAt = (u32, &'static str, &'static str, u32);

/// Merchants that sell a given item (by item_id).
/// Key: item_id, Value: Vec<(shop_id, merchant_name, price)>
pub static ITEM_SOLD_BY: Lazy<HashMap<u32, Vec<SoldBy>>> = Lazy::new(|| {
    let mut map: HashMap<u32, Vec<SoldBy>> = HashMap::new();

    for (shop_id, item) in MERCHANT_ITEMS.iter() {
        map.entry(item.item_id)
            .or_default()
            .push((*shop_id, item.merchant_name, item.price));
    }

    map
});

/// World pickup locations for a given item (by item_id).
/// Key: item_id, Value: Vec<(event_flag, name, region, quantity)>
pub static ITEM_FOUND_AT: Lazy<HashMap<u32, Vec<FoundAt>>> = Lazy::new(|| {
    let mut map: HashMap<u32, Vec<FoundAt>> = HashMap::new();

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

    // Check if dropped by any bosses (from generated data)
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

/// Get relationships for a grace (nearby bosses).
pub fn get_grace_relationships(event_flag: u32) -> Vec<Relationship> {
    let mut relationships = Vec::new();

    if let Some(nearby) = GRACE_NEARBY_BOSSES.get(&event_flag) {
        for (boss_flag, dist) in nearby {
            let label = BOSSES_DATA.get(boss_flag)
                .map(|b| b.name)
                .unwrap_or("Unknown Boss");
            relationships.push(Relationship {
                target_type: EntityType::Boss,
                target_id: *boss_flag,
                rel_type: RelationType::NearbyBoss,
                label,
                secondary: Some(format!("{:.0}m", dist)),
            });
        }
    }

    relationships
}

/// Get relationships for a boss (drops + nearby graces).
pub fn get_boss_relationships(defeat_flag: u32) -> Vec<Relationship> {
    let mut relationships = Vec::new();

    // Boss drops
    if let Some(indices) = BOSS_DROP_INDEX.get(&defeat_flag) {
        for &idx in indices {
            if let Some(drop) = BOSS_DROPS.get(idx) {
                relationships.push(Relationship {
                    target_type: EntityType::Item,
                    target_id: drop.item_id,
                    rel_type: RelationType::Drops,
                    label: drop.item_name,
                    secondary: Some(drop.category.display_name().to_string()),
                });
            }
        }
    }

    // Nearby graces
    if let Some(nearby) = BOSS_NEARBY_GRACES.get(&defeat_flag) {
        for (grace_flag, dist) in nearby {
            let label = GRACES_DATA.get(grace_flag)
                .map(|g| g.name)
                .unwrap_or("Unknown Grace");
            relationships.push(Relationship {
                target_type: EntityType::Grace,
                target_id: *grace_flag,
                rel_type: RelationType::NearbyGrace,
                label,
                secondary: Some(format!("{:.0}m", dist)),
            });
        }
    }

    relationships
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

    #[test]
    fn test_boss_relationships() {
        // Godrick the Grafted should have drops
        let rels = get_boss_relationships(10000800);
        let drops: Vec<_> = rels.iter().filter(|r| r.rel_type == RelationType::Drops).collect();
        assert!(!drops.is_empty(), "Godrick should have drops");
    }

    #[test]
    fn test_grace_relationships() {
        // Just verify no panic on a valid grace flag
        let _ = get_grace_relationships(71000);
    }
}
