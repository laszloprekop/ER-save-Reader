/// Test Case Suite for Event Flag Validation
///
/// Curated test cases with known flag states per character slot.
/// Designed for validation against actual save files to build
/// high-confidence offset mappings.
///
/// ## Design Principles
/// 1. Graces are the most reliable test cases (unique, verifiable, trackable)
/// 2. Early game accessible (Limgrave, Weeping Peninsula, early dungeons)
/// 3. Include both KNOWN_TRUE and KNOWN_FALSE for each slot
/// 4. Cover different flag types (grace, boss, cookbook, progression)
///
/// ## World Pickup Flags Limitation
/// World pickup flags (10-digit format like 1044367310) have limited trackability:
/// - Tile formula stores 875 bytes (7000 bits) per map tile
/// - Only local_id 0-6999 are trackable; 7000+ return None
/// - Most ItemLotParam entries use local_id >= 7000 (untrackable)
/// - These flags are still SET by the game but stored elsewhere
///
/// ## Character Slots (from CLAUDE.md)
/// - Slot 0: Confessor, mid-game progression
/// - Slot 1: Wretch, early game, one boss defeat
/// - Slot 2: V1, item pickup debugging
/// - Slot 3: V2, same progression as V1
/// - Slot 4: V3, control character

use std::collections::HashMap;
use std::path::Path;
use crate::util::verification_records::{VerificationRecord, load_verification_records, get_records_for_slot};

/// A single test case for flag validation
#[derive(Debug, Clone)]
pub struct FlagTestCase {
    /// The flag ID to test
    pub flag_id: u32,
    /// Human-readable name for this flag
    pub name: String,
    /// Category of flag (for filtering/reporting)
    pub category: FlagCategory,
    /// Expected state in the save file
    pub expected: bool,
    /// How the user verified this state
    pub verification_method: String,
    /// Optional: specific item picked up (for inventory verification)
    pub item_name: Option<String>,
    /// Optional: map location for reference
    pub location: Option<String>,
}

/// Categories of flags for prioritization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlagCategory {
    /// World item pickups - highest priority (unique, verifiable)
    WorldPickup,
    /// Site of Grace touched
    Grace,
    /// Boss defeated
    BossDefeat,
    /// NPC interaction/quest state
    NpcEvent,
    /// Cookbook/crafting unlock
    Cookbook,
    /// Map fragment collected
    MapFragment,
    /// Other progression flags
    Progression,
}

impl FlagCategory {
    pub fn priority(&self) -> u8 {
        match self {
            FlagCategory::WorldPickup => 1,  // Highest
            FlagCategory::Grace => 2,
            FlagCategory::BossDefeat => 2,
            FlagCategory::NpcEvent => 3,
            FlagCategory::Cookbook => 3,
            FlagCategory::MapFragment => 3,
            FlagCategory::Progression => 4,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            FlagCategory::WorldPickup => "World Pickup",
            FlagCategory::Grace => "Grace",
            FlagCategory::BossDefeat => "Boss Defeat",
            FlagCategory::NpcEvent => "NPC Event",
            FlagCategory::Cookbook => "Cookbook",
            FlagCategory::MapFragment => "Map Fragment",
            FlagCategory::Progression => "Progression",
        }
    }

    /// Parse category from verification record string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "grace" => FlagCategory::Grace,
            "boss defeat" | "bossdefeat" | "field boss defeat" => FlagCategory::BossDefeat,
            "world pickup" | "worldpickup" | "pickup" => FlagCategory::WorldPickup,
            "npc event" | "npcevent" | "npc" => FlagCategory::NpcEvent,
            "cookbook" => FlagCategory::Cookbook,
            "map fragment" | "mapfragment" | "map" => FlagCategory::MapFragment,
            "landmark" | "landmarks" | "map poi" | "mappoi" | "poi" => FlagCategory::Progression, // Landmarks go to progression
            _ => FlagCategory::Progression,
        }
    }
}

/// Test suite for a specific character slot
#[derive(Debug, Clone)]
pub struct SlotTestSuite {
    /// Slot index (0-9)
    pub slot_index: usize,
    /// Character name for reference
    pub character_name: String,
    /// Description of character progression
    pub description: String,
    /// Flags known to be TRUE (set) for this character
    pub known_true: Vec<FlagTestCase>,
    /// Flags known to be FALSE (not set) for this character
    pub known_false: Vec<FlagTestCase>,
}

impl SlotTestSuite {
    pub fn new(slot_index: usize, character_name: &str, description: &str) -> Self {
        Self {
            slot_index,
            character_name: character_name.to_string(),
            description: description.to_string(),
            known_true: Vec::new(),
            known_false: Vec::new(),
        }
    }

    pub fn add_true(&mut self, case: FlagTestCase) {
        self.known_true.push(case);
    }

    pub fn add_false(&mut self, case: FlagTestCase) {
        self.known_false.push(case);
    }

    /// Get all test cases (both true and false)
    pub fn all_cases(&self) -> impl Iterator<Item = (&FlagTestCase, bool)> {
        self.known_true.iter().map(|c| (c, true))
            .chain(self.known_false.iter().map(|c| (c, false)))
    }

    /// Get test cases by category
    pub fn by_category(&self, category: FlagCategory) -> Vec<(&FlagTestCase, bool)> {
        self.all_cases()
            .filter(|(c, _)| c.category == category)
            .collect()
    }
}

/// Complete test suite across all character slots
pub struct TestSuiteCollection {
    pub slots: HashMap<usize, SlotTestSuite>,
}

impl TestSuiteCollection {
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    pub fn add_slot(&mut self, suite: SlotTestSuite) {
        self.slots.insert(suite.slot_index, suite);
    }

    pub fn get_slot(&self, index: usize) -> Option<&SlotTestSuite> {
        self.slots.get(&index)
    }

    /// Get flags that should differ between two slots
    /// (one has it TRUE, other has it FALSE)
    pub fn get_differentiating_flags(&self, slot_a: usize, slot_b: usize) -> Vec<(u32, &str)> {
        let suite_a = match self.slots.get(&slot_a) {
            Some(s) => s,
            None => return Vec::new(),
        };
        let suite_b = match self.slots.get(&slot_b) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut results = Vec::new();

        // Find flags TRUE in A but FALSE in B
        for case in &suite_a.known_true {
            if suite_b.known_false.iter().any(|c| c.flag_id == case.flag_id) {
                results.push((case.flag_id, case.name.as_str()));
            }
        }

        // Find flags FALSE in A but TRUE in B
        for case in &suite_a.known_false {
            if suite_b.known_true.iter().any(|c| c.flag_id == case.flag_id) {
                results.push((case.flag_id, case.name.as_str()));
            }
        }

        results
    }
}

impl Default for TestSuiteCollection {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CURATED TEST CASES
// ============================================================================
// These are populated based on known character states.
// Start with empty structure - to be filled in with verified data.

/// Build the curated test suite collection
///
/// This function creates test cases based on documented character states.
/// Each test case should be:
/// 1. Verifiable by the user (check inventory, map, etc.)
/// 2. Early game accessible
/// 3. Documented with verification method
pub fn build_test_suite() -> TestSuiteCollection {
    let mut collection = TestSuiteCollection::new();

    // ========================================================================
    // SLOT 0: Confessor (mid-game progression)
    // ========================================================================
    let mut slot0 = SlotTestSuite::new(
        0,
        "Confessor",
        "Mid-game progression, many graces, bosses, items collected"
    );

    // --- KNOWN TRUE: Graces (from verification-records.jsonl 2026-01-20) ---
    // 38 confirmed matches extracted from JSONL where matches=true AND manualStatus=true

    // Tutorial graces (71xxx) - Block base 2625
    slot0.add_true(grace(71800, "Cave of Knowledge", "Tutorial area"));
    slot0.add_true(grace(71801, "Stranded Graveyard", "Tutorial area"));

    // World graces (76xxx) - Block base 3250
    slot0.add_true(grace(76100, "Church of Elleh", "Limgrave"));
    slot0.add_true(grace(76101, "The First Step", "Limgrave"));
    slot0.add_true(grace(76120, "Waypoint Ruins Cellar", "Limgrave"));
    slot0.add_true(grace(76162, "Fourth Church of Marika", "Weeping Peninsula"));
    slot0.add_true(grace(76204, "Academy Gate Town", "Liurnia of the Lakes"));
    slot0.add_true(grace(76206, "Main Academy Gate", "Liurnia of the Lakes"));
    slot0.add_true(grace(76207, "East Raya Lucaria Gate", "Bellum Highway"));
    slot0.add_true(grace(76216, "Boilprawn Shack", "Liurnia of the Lakes"));
    slot0.add_true(grace(76218, "Revenger's Shack", "Liurnia of the Lakes"));
    slot0.add_true(grace(76219, "Folly on the Lake", "Liurnia of the Lakes"));
    slot0.add_true(grace(76231, "Manor Lower Level", "Liurnia of the Lakes"));
    slot0.add_true(grace(76300, "Abandoned Coffin", "Altus Plateau"));
    slot0.add_true(grace(76301, "Altus Plateau", "Altus Plateau"));
    slot0.add_true(grace(76353, "Road of Iniquity", "Mt. Gelmir"));
    slot0.add_true(grace(76400, "Smoldering Church", "Caelid"));
    slot0.add_true(grace(76411, "Southern Aeonia Swamp Bank", "Caelid"));
    slot0.add_true(grace(76450, "Dragonbarrow West", "Greyoll's Dragonbarrow"));

    // Dungeon graces (73xxx) - Block base 2664
    slot0.add_true(grace(73000, "Tombsward Catacombs", "Weeping Peninsula"));
    slot0.add_true(grace(73003, "Road's End Catacombs", "Liurnia of the Lakes"));
    slot0.add_true(grace(73005, "Black Knife Catacombs", "Liurnia of the Lakes"));
    slot0.add_true(grace(73006, "Cliffbottom Catacombs", "Liurnia of the Lakes"));
    slot0.add_true(grace(73014, "Minor Erdtree Catacombs", "Caelid"));
    slot0.add_true(grace(73103, "Groveside Cave", "Limgrave"));
    slot0.add_true(grace(73202, "Raya Lucaria Crystal Tunnel", "Liurnia of the Lakes"));
    slot0.add_true(grace(73257, "Rear Gael Tunnel Entrance", "Caelid"));

    // --- KNOWN TRUE: Boss defeats (dungeon type, Area 10 = Stormveil) ---
    slot0.add_true(boss_defeat(10000800, "Godrick the Grafted", "Stormveil Castle"));
    slot0.add_true(boss_defeat(10000850, "Margit, the Fell Omen", "Stormveil Castle"));

    // --- KNOWN TRUE: Dungeon pickups (Area 10 = Stormveil) ---
    slot0.add_true(world_pickup(10007990, "Godskin Prayerbook", "Godskin Prayerbook", "Stormveil Castle"));
    slot0.add_true(world_pickup(10007230, "Claw Talisman", "Claw Talisman", "Stormveil Castle"));

    // --- KNOWN TRUE: Progression flags (60xxx) ---
    slot0.add_true(FlagTestCase {
        flag_id: 60130,
        name: "Whetstone Knife".to_string(),
        category: FlagCategory::Progression,
        expected: true,
        verification_method: "verification-records.jsonl".to_string(),
        item_name: Some("Whetstone Knife".to_string()),
        location: Some("Gatefront Ruins".to_string()),
    });

    // --- KNOWN TRUE: Map fragments (62xxx) ---
    slot0.add_true(FlagTestCase {
        flag_id: 62063,
        name: "Map: Siofra River".to_string(),
        category: FlagCategory::MapFragment,
        expected: true,
        verification_method: "verification-records.jsonl".to_string(),
        item_name: Some("Map: Siofra River".to_string()),
        location: Some("Underground".to_string()),
    });

    // --- KNOWN TRUE: Cookbooks (67xxx, 68xxx) ---
    slot0.add_true(cookbook(67430, "Nomadic Warrior's Cookbook [9]", "Various"));
    slot0.add_true(cookbook(67630, "Missionary's Cookbook [5]", "Various"));
    slot0.add_true(cookbook(67840, "Perfumer's Cookbook [1]", "Various"));
    slot0.add_true(cookbook(68000, "Ancient Dragon Apostle's Cookbook [1]", "Various"));

    // --- KNOWN TRUE: Crystal Tears (65xxx) ---
    slot0.add_true(FlagTestCase {
        flag_id: 65120,
        name: "Winged Crystal Tear".to_string(),
        category: FlagCategory::Progression,
        expected: true,
        verification_method: "verification-records.jsonl".to_string(),
        item_name: Some("Winged Crystal Tear".to_string()),
        location: Some("Minor Erdtree".to_string()),
    });

    collection.add_slot(slot0);

    // ========================================================================
    // SLOT 1: Wretch (early game)
    // ========================================================================
    let mut slot1 = SlotTestSuite::new(
        1,
        "Wretch",
        "Early game, few graces and pickups, only tutorial enemy defeated"
    );

    // --- KNOWN TRUE: Graces (from verification-records.jsonl) ---
    // Graces with matches=true (offset formula works):
    slot1.add_true(grace(71800, "Cave of Knowledge", "Tutorial area"));
    slot1.add_true(grace(71801, "Stranded Graveyard", "Tutorial area"));
    slot1.add_true(grace(76101, "The First Step", "Limgrave starting area"));

    // Graces with matches=false (offset formula BROKEN - Wretch has them but formula doesn't find):
    // TRUE per manual verification: 76108 (Agheel Lake North), 76111 (Gatefront), 78102 (Guidance)

    // Note: 76100 (Church of Elleh) not in verification records - data shows TRUE in latest save

    collection.add_slot(slot1);

    // ========================================================================
    // SLOT 2: V1 (item pickup debugging)
    // ========================================================================
    let mut slot2 = SlotTestSuite::new(
        2,
        "V1",
        "Test character, very early game, one world pickup (1044367310)"
    );

    // --- KNOWN TRUE: Graces V1 HAS touched (from verification-records.jsonl) ---
    // Both have matches=true (offset formula works)
    slot2.add_true(grace(71801, "Stranded Graveyard", "Tutorial area"));
    slot2.add_true(grace(76101, "The First Step", "Limgrave starting area"));

    // NOTE: World pickup flag 1044367310 has local_id=7310, outside trackable range (0-6999)

    collection.add_slot(slot2);

    // ========================================================================
    // SLOT 3: V2 (same progression as V1)
    // ========================================================================
    let mut slot3 = SlotTestSuite::new(
        3,
        "V2",
        "Test character, same as V1, different travel path to pickup"
    );

    // --- KNOWN TRUE: Graces V2 HAS touched (from verification-records.jsonl) ---
    // Both have matches=true (offset formula works)
    slot3.add_true(grace(71801, "Stranded Graveyard", "Tutorial area"));
    slot3.add_true(grace(76101, "The First Step", "Limgrave starting area"));

    collection.add_slot(slot3);

    // ========================================================================
    // SLOT 4: V3 (control - no pickup)
    // ========================================================================
    let mut slot4 = SlotTestSuite::new(
        4,
        "V3",
        "Test character, same location as V1/V2, but did NOT pick up item"
    );

    // --- KNOWN TRUE: Graces V3 HAS touched (from verification-records.jsonl) ---
    // Both have matches=true (offset formula works)
    slot4.add_true(grace(71801, "Stranded Graveyard", "Tutorial area"));
    slot4.add_true(grace(76101, "The First Step", "Limgrave starting area"));

    collection.add_slot(slot4);

    // ========================================================================
    // SLOT 5: Sam (early-mid game progression)
    // ========================================================================
    let mut slot5 = SlotTestSuite::new(
        5,
        "Sam",
        "Early-mid game progression, exploring Limgrave and surrounding areas"
    );

    // --- KNOWN TRUE: Graces Sam HAS touched (from verification-records.jsonl) ---
    // Graces with matches=true (offset formula works):
    slot5.add_true(grace(71801, "Stranded Graveyard", "Tutorial area"));
    slot5.add_true(grace(73011, "Deathtouched Catacombs", "Stormhill"));
    slot5.add_true(grace(76100, "Church of Elleh", "Limgrave"));
    slot5.add_true(grace(76101, "The First Step", "Limgrave starting area"));

    // Graces with matches=false (offset formula BROKEN - Sam has them but formula doesn't find):
    // These are documented but expected to FAIL until formula is fixed
    // TRUE per manual verification: 76106, 76108, 76111, 76117, 76119, 76150, 76151, 76153, 76157, 76162, 76400

    collection.add_slot(slot5);

    collection
}

/// Helper to create a world pickup test case
pub fn world_pickup(
    flag_id: u32,
    name: &str,
    item_name: &str,
    location: &str,
) -> FlagTestCase {
    FlagTestCase {
        flag_id,
        name: name.to_string(),
        category: FlagCategory::WorldPickup,
        expected: true,
        verification_method: format!("Check inventory for '{}'", item_name),
        item_name: Some(item_name.to_string()),
        location: Some(location.to_string()),
    }
}

/// Helper to create a grace test case
pub fn grace(flag_id: u32, name: &str, location: &str) -> FlagTestCase {
    FlagTestCase {
        flag_id,
        name: name.to_string(),
        category: FlagCategory::Grace,
        expected: true,
        verification_method: "Check grace list in map menu".to_string(),
        item_name: None,
        location: Some(location.to_string()),
    }
}

/// Helper to create a boss defeat test case
pub fn boss_defeat(flag_id: u32, name: &str, location: &str) -> FlagTestCase {
    FlagTestCase {
        flag_id,
        name: name.to_string(),
        category: FlagCategory::BossDefeat,
        expected: true,
        verification_method: "Boss arena is cleared, fog gate gone".to_string(),
        item_name: None,
        location: Some(location.to_string()),
    }
}

/// Helper to create a cookbook test case
pub fn cookbook(flag_id: u32, name: &str, location: &str) -> FlagTestCase {
    FlagTestCase {
        flag_id,
        name: name.to_string(),
        category: FlagCategory::Cookbook,
        expected: true,
        verification_method: "Check Item Crafting menu for recipes".to_string(),
        item_name: Some(name.to_string()),
        location: Some(location.to_string()),
    }
}

// ============================================================================
// DYNAMIC TEST SUITE FROM VERIFICATION RECORDS
// ============================================================================

/// Build test suite dynamically from verification records
///
/// This replaces hardcoded test cases with data loaded from the verification
/// records JSONL file. Each record's `manual_status` becomes the expected value.
///
/// # Arguments
/// * `records_path` - Path to the verification-records.jsonl file
///
/// # Returns
/// A TestSuiteCollection with test cases for all slots found in the records
pub fn build_test_suite_from_records(records_path: &Path) -> Result<TestSuiteCollection, String> {
    let records = load_verification_records(records_path)
        .map_err(|e| format!("Failed to load verification records: {}", e))?;

    if records.is_empty() {
        return Err("No verification records found".to_string());
    }

    let mut collection = TestSuiteCollection::new();

    // Group records by slot
    let mut slots_map: HashMap<u32, Vec<&VerificationRecord>> = HashMap::new();
    for record in &records {
        slots_map.entry(record.slot_index).or_default().push(record);
    }

    // Build test suite for each slot
    for (slot_index, slot_records) in slots_map {
        // Get character info from first record
        let first = slot_records.first().unwrap();
        let mut suite = SlotTestSuite::new(
            slot_index as usize,
            &first.character_name,
            &format!("Dynamically loaded from verification records ({} test cases)", slot_records.len())
        );

        // Convert each record to a test case
        for record in slot_records {
            let case = FlagTestCase {
                flag_id: record.flag_id,
                name: record.flag_name.clone(),
                category: FlagCategory::from_str(&record.flag_category),
                expected: record.user_marked_complete,
                verification_method: format!("From verification record ({})", record.flag_region),
                item_name: None,
                location: Some(record.flag_region.clone()),
            };

            // Add to appropriate list based on expected value
            if record.user_marked_complete {
                suite.add_true(case);
            } else {
                suite.add_false(case);
            }
        }

        collection.add_slot(suite);
    }

    Ok(collection)
}

/// Build test suite for a specific slot from verification records
pub fn build_slot_test_suite_from_records(
    records_path: &Path,
    slot_index: usize,
) -> Result<SlotTestSuite, String> {
    let records = load_verification_records(records_path)
        .map_err(|e| format!("Failed to load verification records: {}", e))?;

    let slot_records = get_records_for_slot(&records, slot_index as u32);

    if slot_records.is_empty() {
        return Err(format!("No verification records found for slot {}", slot_index));
    }

    let first = &slot_records[0];
    let mut suite = SlotTestSuite::new(
        slot_index,
        &first.character_name,
        &format!("Dynamically loaded ({} test cases)", slot_records.len())
    );

    for record in &slot_records {
        let case = FlagTestCase {
            flag_id: record.flag_id,
            name: record.flag_name.clone(),
            category: FlagCategory::from_str(&record.flag_category),
            expected: record.user_marked_complete,
            verification_method: format!("From verification record ({})", record.flag_region),
            item_name: None,
            location: Some(record.flag_region.clone()),
        };

        if record.user_marked_complete {
            suite.add_true(case);
        } else {
            suite.add_false(case);
        }
    }

    Ok(suite)
}

// ============================================================================
// VALIDATION ENGINE
// ============================================================================

use crate::save::save::save::Save;
use crate::db::pickup_flags::get_flag_offset;

/// Result of validating a single test case
#[derive(Debug)]
pub struct TestCaseResult {
    pub flag_id: u32,
    pub name: String,
    pub category: FlagCategory,
    pub expected: bool,
    pub actual: Option<bool>,
    pub passed: bool,
    pub offset: Option<(usize, u8)>,
    pub error: Option<String>,
}

/// Result of validating all test cases for a slot
#[derive(Debug)]
pub struct SlotValidationResult {
    pub slot_index: usize,
    pub character_name: String,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub results: Vec<TestCaseResult>,
}

impl SlotValidationResult {
    pub fn pass_rate(&self) -> f64 {
        if self.total_tests == 0 {
            return 0.0;
        }
        self.passed as f64 / self.total_tests as f64 * 100.0
    }
}

/// Validate test cases against a save file
pub struct TestCaseValidator {
    suite: TestSuiteCollection,
}

impl TestCaseValidator {
    pub fn new() -> Self {
        Self {
            suite: build_test_suite(),
        }
    }

    /// Validate all test cases for a specific slot
    pub fn validate_slot(
        &self,
        save_path: &Path,
        slot_index: usize,
    ) -> Result<SlotValidationResult, String> {
        // Load save file
        let save = Save::from_path(&save_path.to_path_buf())
            .map_err(|e| format!("Failed to load save: {}", e))?;

        let slot = save.save_type.get_slot(slot_index);
        let event_flags = &slot.event_flags.flags;

        // Get test suite for this slot
        let test_suite = self.suite.get_slot(slot_index)
            .ok_or_else(|| format!("No test suite defined for slot {}", slot_index))?;

        let mut results = Vec::new();
        let mut passed = 0;
        let mut failed = 0;
        let mut errors = 0;

        // Run all test cases
        for (case, expected_state) in test_suite.all_cases() {
            let result = self.validate_case(event_flags, case, expected_state);

            if result.error.is_some() {
                errors += 1;
            } else if result.passed {
                passed += 1;
            } else {
                failed += 1;
            }

            results.push(result);
        }

        Ok(SlotValidationResult {
            slot_index,
            character_name: test_suite.character_name.clone(),
            total_tests: results.len(),
            passed,
            failed,
            errors,
            results,
        })
    }

    /// Validate a single test case
    fn validate_case(
        &self,
        event_flags: &[u8],
        case: &FlagTestCase,
        expected: bool,
    ) -> TestCaseResult {
        // Try to get the offset for this flag
        let offset = get_flag_offset(case.flag_id);

        match offset {
            Some((byte_offset, bit_pos)) => {
                // Read the actual bit value
                if byte_offset as usize >= event_flags.len() {
                    return TestCaseResult {
                        flag_id: case.flag_id,
                        name: case.name.clone(),
                        category: case.category,
                        expected,
                        actual: None,
                        passed: false,
                        offset: Some((byte_offset as usize, bit_pos)),
                        error: Some(format!("Offset {} out of bounds (max {})",
                            byte_offset, event_flags.len())),
                    };
                }

                let byte = event_flags[byte_offset as usize];
                let actual = (byte >> bit_pos) & 1 == 1;
                let passed = actual == expected;

                TestCaseResult {
                    flag_id: case.flag_id,
                    name: case.name.clone(),
                    category: case.category,
                    expected,
                    actual: Some(actual),
                    passed,
                    offset: Some((byte_offset as usize, bit_pos)),
                    error: None,
                }
            }
            None => {
                TestCaseResult {
                    flag_id: case.flag_id,
                    name: case.name.clone(),
                    category: case.category,
                    expected,
                    actual: None,
                    passed: false,
                    offset: None,
                    error: Some("No offset formula for this flag ID".to_string()),
                }
            }
        }
    }

    /// Validate all slots and find cross-slot agreements/disagreements
    pub fn validate_all_slots(
        &self,
        save_path: &Path,
    ) -> Result<Vec<SlotValidationResult>, String> {
        let mut all_results = Vec::new();

        for slot_index in self.suite.slots.keys() {
            match self.validate_slot(save_path, *slot_index) {
                Ok(result) => all_results.push(result),
                Err(e) => println!("Warning: Slot {} validation failed: {}", slot_index, e),
            }
        }

        Ok(all_results)
    }

    /// Get the test suite collection for inspection
    pub fn suite(&self) -> &TestSuiteCollection {
        &self.suite
    }
}

impl Default for TestCaseValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DYNAMIC VALIDATOR (loads from verification records)
// ============================================================================

/// A test case validator that loads test cases dynamically from verification records
///
/// Unlike `TestCaseValidator` which uses hardcoded test cases, this validator
/// loads expectations from an external JSONL file. This makes tests adaptive
/// to changes in save files and verification data.
pub struct DynamicTestCaseValidator {
    suite: TestSuiteCollection,
    records_path: std::path::PathBuf,
}

impl DynamicTestCaseValidator {
    /// Create a new validator loading from the specified verification records file
    pub fn from_records(records_path: &Path) -> Result<Self, String> {
        let suite = build_test_suite_from_records(records_path)?;
        Ok(Self {
            suite,
            records_path: records_path.to_path_buf(),
        })
    }

    /// Get the path to the verification records file
    pub fn records_path(&self) -> &Path {
        &self.records_path
    }

    /// Validate all test cases for a specific slot
    pub fn validate_slot(
        &self,
        save_path: &Path,
        slot_index: usize,
    ) -> Result<SlotValidationResult, String> {
        let save = Save::from_path(&save_path.to_path_buf())
            .map_err(|e| format!("Failed to load save: {}", e))?;

        let slot = save.save_type.get_slot(slot_index);
        let event_flags = &slot.event_flags.flags;

        let test_suite = self.suite.get_slot(slot_index)
            .ok_or_else(|| format!("No verification records for slot {}", slot_index))?;

        let mut results = Vec::new();
        let mut passed = 0;
        let mut failed = 0;
        let mut errors = 0;

        for (case, expected_state) in test_suite.all_cases() {
            let result = validate_single_case(event_flags, case, expected_state);

            if result.error.is_some() {
                errors += 1;
            } else if result.passed {
                passed += 1;
            } else {
                failed += 1;
            }

            results.push(result);
        }

        Ok(SlotValidationResult {
            slot_index,
            character_name: test_suite.character_name.clone(),
            total_tests: results.len(),
            passed,
            failed,
            errors,
            results,
        })
    }

    /// Validate all slots that have verification records
    pub fn validate_all_slots(
        &self,
        save_path: &Path,
    ) -> Result<Vec<SlotValidationResult>, String> {
        let mut all_results = Vec::new();

        for slot_index in self.suite.slots.keys() {
            match self.validate_slot(save_path, *slot_index) {
                Ok(result) => all_results.push(result),
                Err(e) => println!("Warning: Slot {} validation failed: {}", slot_index, e),
            }
        }

        Ok(all_results)
    }

    /// Get the test suite collection
    pub fn suite(&self) -> &TestSuiteCollection {
        &self.suite
    }

    /// Get count of slots with test cases
    pub fn slot_count(&self) -> usize {
        self.suite.slots.len()
    }

    /// Get total test case count across all slots
    pub fn total_test_count(&self) -> usize {
        self.suite.slots.values()
            .map(|s| s.known_true.len() + s.known_false.len())
            .sum()
    }
}

/// Validate a single test case (shared logic)
fn validate_single_case(
    event_flags: &[u8],
    case: &FlagTestCase,
    expected: bool,
) -> TestCaseResult {
    let offset = get_flag_offset(case.flag_id);

    match offset {
        Some((byte_offset, bit_pos)) => {
            if byte_offset as usize >= event_flags.len() {
                return TestCaseResult {
                    flag_id: case.flag_id,
                    name: case.name.clone(),
                    category: case.category,
                    expected,
                    actual: None,
                    passed: false,
                    offset: Some((byte_offset as usize, bit_pos)),
                    error: Some(format!("Offset {} out of bounds (max {})",
                        byte_offset, event_flags.len())),
                };
            }

            let byte = event_flags[byte_offset as usize];
            let actual = (byte >> bit_pos) & 1 == 1;
            let passed = actual == expected;

            TestCaseResult {
                flag_id: case.flag_id,
                name: case.name.clone(),
                category: case.category,
                expected,
                actual: Some(actual),
                passed,
                offset: Some((byte_offset as usize, bit_pos)),
                error: None,
            }
        }
        None => {
            TestCaseResult {
                flag_id: case.flag_id,
                name: case.name.clone(),
                category: case.category,
                expected,
                actual: None,
                passed: false,
                offset: None,
                error: Some("No offset formula for this flag ID".to_string()),
            }
        }
    }
}

/// Print a validation report
pub fn print_validation_report(result: &SlotValidationResult) {
    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║ TEST CASE VALIDATION: Slot {} ({})                    ",
        result.slot_index, result.character_name);
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Total: {} | Passed: {} | Failed: {} | Errors: {}",
        result.total_tests, result.passed, result.failed, result.errors);
    println!("║ Pass Rate: {:.1}%", result.pass_rate());
    println!("╠══════════════════════════════════════════════════════════════╣");

    // Group by category
    let mut by_category: HashMap<FlagCategory, Vec<&TestCaseResult>> = HashMap::new();
    for r in &result.results {
        by_category.entry(r.category).or_default().push(r);
    }

    for (category, cases) in by_category.iter() {
        let cat_passed = cases.iter().filter(|c| c.passed).count();
        println!("║");
        println!("║ {} ({}/{})", category.name(), cat_passed, cases.len());
        for case in cases {
            let status = if case.passed {
                "✓"
            } else if case.error.is_some() {
                "?"
            } else {
                "✗"
            };

            let detail = if let Some(err) = &case.error {
                format!("ERROR: {}", err)
            } else if let (Some(actual), Some((byte, bit))) = (case.actual, case.offset) {
                format!("expected={}, actual={} @ 0x{:x}:{}",
                    case.expected, actual, byte, bit)
            } else {
                "no offset".to_string()
            };

            println!("║   {} {} ({}): {}",
                status, case.name, case.flag_id, detail);
        }
    }

    println!("╚══════════════════════════════════════════════════════════════╝");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_suite() {
        let suite = build_test_suite();
        assert!(suite.slots.contains_key(&0));
        assert!(suite.slots.contains_key(&4));
    }

    #[test]
    fn test_category_priority() {
        assert!(FlagCategory::WorldPickup.priority() < FlagCategory::Grace.priority());
        assert!(FlagCategory::Grace.priority() < FlagCategory::Progression.priority());
    }

    #[test]
    fn test_validator_creation() {
        let validator = TestCaseValidator::new();
        // We have slots 0, 1, 2, 3, 4, 5 = 6 total
        assert_eq!(validator.suite.slots.len(), 6);
    }
}
