# Verification Strategy for ER-save-Reader

## Overview

This document outlines verification processes to improve the accuracy of save file parsing and database coverage. The goal is to achieve 100% accuracy by cross-validating against multiple sources.

---

## Available Verification Assets

### 1. Granular Save Snapshots (Ground Truth)
Location: `/Elden Ring save files/Granular snapshots for debugging/`

Sequential snapshots capturing specific game events:
- **Pickup events**: Before/after collecting items
- **Grace discovery**: Before/after touching Sites of Grace
- **Boss defeats**: Pre/post boss kill states
- **Progression milestones**: Door opens, NPC interactions

**Example sequence**:
```
09 before picking up Smoldering Butterfly treasure_m60_43_50_00_1043500010
10 after picked up Smoldering Butterfly treasure_m60_43_50_00_1043500010
```

### 2. Controlled Test Characters
| Slot | Character | Purpose |
|------|-----------|---------|
| 0 | Confessor | Mid-game reference (544 pickups collected) |
| 1 | Wretch | Early game progression tracking |
| 2 | V1 | Pickup debugging (specific item collected) |
| 3 | V2 | Pickup debugging (same item, different path) |
| 4 | V3 | True negative (no pickup for diff control) |

### 3. Decompiled Game Params (Source of Truth)
Location: `/Elden Ring decompiled game files/regulation-bin/`

Primary params for verification:
- `ItemLotParam_map.param.xml` - World pickup definitions
- `ShopLineupParam.param.xml` - Shop item definitions
- `EquipParamWeapon.param.xml` - Weapon definitions
- `EquipParamGoods.param.xml` - Item definitions
- `Magic.param.xml` - Spell definitions

### 4. Existing Validator (src/util/validator.rs)
Current validations:
- Weapon-gem compatibility
- Armor category validation
- Duplicate item detection
- Physics tear validation
- Equipped item validation

---

## Proposed Verification Processes

### V1: Differential Snapshot Testing

**Purpose**: Verify flag changes match expected outcomes for known events

**Method**:
1. Load "before" snapshot
2. Load "after" snapshot
3. Diff event_flags byte arrays
4. Verify exactly expected flags changed

**Implementation**:
```rust
pub struct DiffTest {
    before_path: PathBuf,
    after_path: PathBuf,
    expected_flag_changes: Vec<(u32, bool)>, // (flag_id, new_value)
    description: &'static str,
}

pub fn run_diff_test(test: &DiffTest) -> Result<(), Vec<String>> {
    let before = load_save(&test.before_path)?;
    let after = load_save(&test.after_path)?;

    let before_flags = &before.slot.event_flags;
    let after_flags = &after.slot.event_flags;

    let mut errors = vec![];

    // Check expected changes occurred
    for (flag_id, expected_value) in &test.expected_flag_changes {
        let before_value = is_flag_set(before_flags, *flag_id);
        let after_value = is_flag_set(after_flags, *flag_id);

        if after_value != *expected_value {
            errors.push(format!(
                "Flag {} expected {} but got {}",
                flag_id, expected_value, after_value
            ));
        }
    }

    // Check no unexpected changes
    let changed_flags = find_all_changed_flags(before_flags, after_flags);
    for flag_id in changed_flags {
        if !test.expected_flag_changes.iter().any(|(f, _)| *f == flag_id) {
            errors.push(format!("Unexpected flag change: {}", flag_id));
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
```

**Test cases from snapshots**:
| Test | Before | After | Expected Flag |
|------|--------|-------|---------------|
| Smoldering Butterfly | 09 | 10 | 1043500010=true |
| Missionary Cookbook | 01 | 02 | 67310=true |
| Minor Eldtree Grace | 03 | 04 | grace_flag=true |
| Golden Order Seal | 05 | 06 | pickup_flag=true |

---

### V2: Database-to-Param Cross-Validation

**Purpose**: Ensure database entries match game params

**Method**:
1. Parse game param XML files
2. Compare against database module entries
3. Report discrepancies

**Implementation**:
```rust
pub fn validate_pickup_database() -> ValidationReport {
    let item_lot_param = parse_xml("ItemLotParam_map.param.xml");
    let mut report = ValidationReport::new();

    for pickup in WORLD_PICKUPS.iter() {
        // Verify flag exists in ItemLotParam
        let lot = item_lot_param.get(&pickup.item_lot_id);
        if lot.is_none() {
            report.add_error(format!(
                "Pickup {} not found in ItemLotParam", pickup.item_lot_id
            ));
            continue;
        }

        // Verify flag ID matches
        let lot = lot.unwrap();
        if lot.getItemFlagId != pickup.event_flag {
            report.add_error(format!(
                "Flag mismatch for {}: db={}, param={}",
                pickup.item_lot_id, pickup.event_flag, lot.getItemFlagId
            ));
        }

        // Verify item exists in appropriate param
        validate_item_exists(pickup.item_id, pickup.category, &mut report);
    }

    report
}
```

**Validations**:
| Database | Against Param | Checks |
|----------|---------------|--------|
| pickup_data.rs | ItemLotParam_map | flag_id, item_id, quantity |
| spells.rs | Magic.param | spell_id, fp_cost, slots |
| shop_items.rs | ShopLineupParam | item_id, stock_flag, release_flag |
| weapon_name.rs | EquipParamWeapon | weapon_id exists |
| armor_name.rs | EquipParamProtector | armor_id exists |

---

### V3: Flag Offset Formula Verification

**Purpose**: Verify formula-based offset calculation is correct

**Method**:
1. Use known flags with verified values in test saves
2. Calculate offset using formula
3. Read actual bit from save
4. Compare expected vs actual

**Implementation**:
```rust
pub fn verify_flag_formula(save: &SaveSlot) -> Vec<FlagVerification> {
    let known_flags = vec![
        // Graces (known to be collected from gameplay)
        KnownFlag { id: 76100, name: "First Step", expected: true },
        KnownFlag { id: 76101, name: "Church of Elleh", expected: true },
        // Bosses (known to be defeated)
        KnownFlag { id: 10000, name: "Margit", expected: true },
        // Cookbooks (known to be collected)
        KnownFlag { id: 67310, name: "Missionary's Cookbook [4]", expected: true },
    ];

    let mut results = vec![];
    for flag in known_flags {
        let calculated = is_flag_set(&save.event_flags, flag.id);
        results.push(FlagVerification {
            flag_id: flag.id,
            name: flag.name,
            expected: flag.expected,
            actual: calculated,
            passed: calculated == flag.expected,
        });
    }
    results
}
```

**Edge cases to test**:
- Tile boundary flags (first/last in tile)
- Dungeon flags (8-digit format)
- Simple flags (< 100000)
- DLC flags (20XXXXXXXX format)

---

### V4: Round-Trip Integrity Testing

**Purpose**: Ensure save modifications don't corrupt data

**Method**:
1. Load save
2. Modify specific flag
3. Write save
4. Reload save
5. Verify modification persisted
6. Verify no other data changed

**Implementation**:
```rust
pub fn test_round_trip_integrity(
    save_path: &Path,
    flag_id: u32,
    new_value: bool
) -> Result<(), String> {
    // Load original
    let original = load_save(save_path)?;
    let original_hash = hash_event_flags(&original.event_flags);

    // Modify
    let mut modified = original.clone();
    set_flag(&mut modified.event_flags, flag_id, new_value);

    // Write to temp file
    let temp_path = temp_file();
    write_save(&modified, &temp_path)?;

    // Reload
    let reloaded = load_save(&temp_path)?;

    // Verify modification
    let actual_value = is_flag_set(&reloaded.event_flags, flag_id);
    if actual_value != new_value {
        return Err(format!(
            "Flag {} not persisted: expected {}, got {}",
            flag_id, new_value, actual_value
        ));
    }

    // Verify no collateral damage (excluding the changed flag)
    let expected_changes = 1; // Only the one flag we changed
    let actual_changes = count_flag_differences(
        &original.event_flags,
        &reloaded.event_flags
    );

    if actual_changes != expected_changes {
        return Err(format!(
            "Unexpected changes: expected {} flag change, got {}",
            expected_changes, actual_changes
        ));
    }

    Ok(())
}
```

---

### V5: Coverage Gap Detection

**Purpose**: Identify flags in save that aren't in database

**Method**:
1. Scan all set flags in save
2. Check each against database
3. Report unmapped flags with context

**Implementation**:
```rust
pub fn detect_unmapped_flags(save: &SaveSlot) -> Vec<UnmappedFlag> {
    let mut unmapped = vec![];

    // Iterate all bytes in event_flags
    for (byte_idx, byte) in save.event_flags.iter().enumerate() {
        for bit in 0..8 {
            if (byte >> (7 - bit)) & 1 == 1 {
                let flag_id = reverse_calculate_flag_id(byte_idx, bit);

                if !is_flag_in_database(flag_id) {
                    unmapped.push(UnmappedFlag {
                        flag_id,
                        byte_offset: byte_idx,
                        bit_position: bit,
                        category: categorize_flag(flag_id),
                    });
                }
            }
        }
    }

    unmapped
}

fn categorize_flag(flag_id: u32) -> FlagCategory {
    match flag_id {
        0..=999 => FlagCategory::Core,
        9100..=9199 => FlagCategory::Remembrance,
        10_000_000..=19_999_999 => FlagCategory::TilePickup,
        20_000_000..=29_999_999 => FlagCategory::DlcPickup,
        60_000..=69_999 => FlagCategory::Progression,
        100_000..=199_999 => FlagCategory::ShopStock,
        _ => FlagCategory::Unknown,
    }
}
```

**Output**: Report showing coverage gaps by category:
```
Coverage Report for Slot 0 (Confessor):
- Total flags set: 2,847
- Mapped in database: 1,892 (66.5%)
- Unmapped: 955 (33.5%)

Unmapped by category:
- TilePickup: 412 (need to expand pickup_data.rs)
- ShopStock: 234 (need shop_items.rs integration)
- Progression: 189 (need event_flags.rs expansion)
- Unknown: 120 (need investigation)
```

---

### V6: Consistency Cross-Checks

**Purpose**: Verify related flags have consistent states

**Rules to verify**:
1. Boss defeated → Remembrance possession possible
2. Map fragment possessed → Discovery flag set
3. Grace touched → Valid spawn point
4. Shop item purchased → Stock flag set + release flag met

**Implementation**:
```rust
pub fn verify_consistency(save: &SaveSlot) -> Vec<ConsistencyError> {
    let mut errors = vec![];

    // Rule 1: Boss → Remembrance
    for (boss_flag, remembrance_flag) in BOSS_REMEMBRANCE_MAP {
        let boss_defeated = is_flag_set(&save.event_flags, *boss_flag);
        let has_remembrance = is_flag_set(&save.event_flags, *remembrance_flag);

        // Can have remembrance only if boss defeated (not vice versa - may have used it)
        if has_remembrance && !boss_defeated {
            errors.push(ConsistencyError {
                rule: "Boss-Remembrance",
                message: format!(
                    "Has remembrance {} but boss {} not defeated",
                    remembrance_flag, boss_flag
                ),
            });
        }
    }

    // Rule 2: Map fragment possession → discovery
    for (possession_flag, discovery_flag) in MAP_FRAGMENT_FLAGS {
        let possessed = is_flag_set(&save.event_flags, *possession_flag);
        let discovered = is_flag_set(&save.event_flags, *discovery_flag);

        if possessed && !discovered {
            errors.push(ConsistencyError {
                rule: "MapFragment",
                message: format!(
                    "Has map fragment {} but discovery {} not set",
                    possession_flag, discovery_flag
                ),
            });
        }
    }

    errors
}
```

---

## Implementation Priority

| Priority | Verification | Impact | Effort |
|----------|--------------|--------|--------|
| **1** | V3: Flag Formula | Critical - core accuracy | Low |
| **2** | V1: Differential Testing | High - proves correctness | Medium |
| **3** | V2: DB-Param Cross-Validation | High - catches data errors | Medium |
| **4** | V5: Coverage Gap Detection | Medium - guides expansion | Low |
| **5** | V4: Round-Trip Integrity | Medium - ensures safety | Medium |
| **6** | V6: Consistency Checks | Low - quality assurance | High |

---

## Automation Strategy

### Test Harness Structure
```
tests/
├── fixtures/
│   ├── snapshots/           # Symlink to granular snapshots
│   └── expected/            # Expected diff results
├── verification/
│   ├── flag_formula_test.rs
│   ├── differential_test.rs
│   ├── crossval_test.rs
│   ├── coverage_test.rs
│   └── integrity_test.rs
└── integration/
    └── full_validation.rs
```

### CI Integration
```yaml
# .github/workflows/verify.yml
verification:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v3
    - name: Run flag formula tests
      run: cargo test --test flag_formula_test
    - name: Run cross-validation
      run: cargo test --test crossval_test
    - name: Generate coverage report
      run: cargo run --bin coverage_report
```

---

## Expected Outcomes

After implementing these verification processes:

1. **Flag Formula**: 100% confidence in offset calculation
2. **Differential Tests**: Regression-proof pickup tracking
3. **Cross-Validation**: Database entries match game data
4. **Coverage Reports**: Clear roadmap for database expansion
5. **Round-Trip Tests**: Safe save modification
6. **Consistency Checks**: Logical state validation

---

## Appendix: Test Data Extraction

### Creating New Differential Tests
1. Play game to desired state
2. Backup save (before)
3. Perform specific action (pickup, grace touch, etc.)
4. Backup save (after)
5. Name with descriptive pattern: `XX description_FLAGID`
6. Add to test corpus with expected flag

### Param File Parsing
Use existing `scripts/extract_*.py` patterns to parse params for validation.
