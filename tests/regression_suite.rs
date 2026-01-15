//! Regression Suite
//!
//! Basic validation tests that don't require crate imports.
//! More comprehensive tests are in the unit test modules within src/.

use std::fs;
use std::path::Path;

/// Test that ground_truth_offsets.json exists and is valid JSON
#[test]
fn test_ground_truth_exists() {
    let path = Path::new("ground_truth_offsets.json");
    assert!(path.exists(), "ground_truth_offsets.json should exist");

    let content = fs::read_to_string(path)
        .expect("Should be able to read ground_truth_offsets.json");

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .expect("ground_truth_offsets.json should be valid JSON");

    // Check required sections exist
    assert!(parsed.get("metadata").is_some(), "Should have metadata section");
    assert!(parsed.get("formulas").is_some(), "Should have formulas section");
    assert!(parsed.get("verified_flags").is_some(), "Should have verified_flags section");
}

/// Test that flag_relationships.json exists and is valid JSON
#[test]
#[ignore] // Only run when relationships file exists
fn test_relationships_exists() {
    let path = Path::new("scripts/flag_relationships.json");
    if !path.exists() {
        println!("flag_relationships.json not found, skipping test");
        return;
    }

    let content = fs::read_to_string(path)
        .expect("Should be able to read flag_relationships.json");

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .expect("flag_relationships.json should be valid JSON");

    // Check required sections exist
    assert!(parsed.get("nodes").is_some(), "Should have nodes section");
    assert!(parsed.get("edges").is_some(), "Should have edges section");
    assert!(parsed.get("statistics").is_some(), "Should have statistics section");

    // Check statistics values
    if let Some(stats) = parsed.get("statistics") {
        let total_flags = stats.get("total_flags")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        assert!(total_flags > 4000, "Expected 4000+ flags, got {}", total_flags);

        let total_rels = stats.get("total_relationships")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        assert!(total_rels > 2000, "Expected 2000+ relationships, got {}", total_rels);
    }
}

/// Test that extracted_event_flags.json exists and has expected flags
#[test]
#[ignore] // Only run when catalog file exists
fn test_catalog_exists() {
    let path = Path::new("scripts/extracted_event_flags.json");
    if !path.exists() {
        println!("extracted_event_flags.json not found, skipping test");
        return;
    }

    let content = fs::read_to_string(path)
        .expect("Should be able to read extracted_event_flags.json");

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .expect("extracted_event_flags.json should be valid JSON");

    // Should be an array
    let flags = parsed.as_array()
        .expect("extracted_event_flags.json should be an array");

    assert!(flags.len() > 5000, "Expected 5000+ flags, got {}", flags.len());
}

/// Test that block bases in ground_truth are reasonable
#[test]
fn test_block_bases_reasonable() {
    let path = Path::new("ground_truth_offsets.json");
    let content = fs::read_to_string(path)
        .expect("Should be able to read ground_truth_offsets.json");

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .expect("ground_truth_offsets.json should be valid JSON");

    let formulas = parsed.get("formulas")
        .expect("Should have formulas section")
        .as_object()
        .expect("formulas should be an object");

    let block_bases = formulas.get("block_bases")
        .expect("Should have block_bases in formulas")
        .as_object()
        .expect("block_bases should be an object");

    for (block_key, block_data) in block_bases {
        let base_offset = block_data.get("base_offset")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Skip unverified blocks with 0 offset
        if base_offset == 0 {
            continue;
        }

        // Block bases should be within reasonable range (< 5000 for 5-digit flags)
        assert!(
            base_offset < 5000,
            "Block {} has unreasonable base offset: {}",
            block_key, base_offset
        );

        // Block should have block_start matching key
        let block_start = block_data.get("block_start")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let expected_start: u64 = block_key.parse().unwrap_or(0);
        assert_eq!(
            block_start, expected_start,
            "Block {} has mismatched block_start: {}",
            block_key, block_start
        );
    }
}

/// Test that verified flags have required fields
#[test]
fn test_verified_flags_schema() {
    let path = Path::new("ground_truth_offsets.json");
    let content = fs::read_to_string(path)
        .expect("Should be able to read ground_truth_offsets.json");

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .expect("ground_truth_offsets.json should be valid JSON");

    let verified_flags = parsed.get("verified_flags")
        .expect("Should have verified_flags section")
        .as_object()
        .expect("verified_flags should be an object");

    for (flag_key, flag_data) in verified_flags {
        // Each flag should have offset and bit
        assert!(
            flag_data.get("offset").is_some(),
            "Flag {} missing 'offset' field",
            flag_key
        );
        assert!(
            flag_data.get("bit").is_some(),
            "Flag {} missing 'bit' field",
            flag_key
        );

        // Bit should be 0-7
        let bit = flag_data.get("bit")
            .and_then(|v| v.as_u64())
            .unwrap_or(8);
        assert!(
            bit < 8,
            "Flag {} has invalid bit position: {}",
            flag_key, bit
        );
    }
}

/// Test discoveries.json schema if it exists
#[test]
#[ignore] // Only run when discoveries file exists
fn test_discoveries_schema() {
    let path = Path::new("discoveries.json");
    if !path.exists() {
        println!("discoveries.json not found, skipping test");
        return;
    }

    let content = fs::read_to_string(path)
        .expect("Should be able to read discoveries.json");

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .expect("discoveries.json should be valid JSON");

    // Check required sections
    assert!(parsed.get("version").is_some(), "Should have version field");
    assert!(parsed.get("metadata").is_some(), "Should have metadata section");
    assert!(parsed.get("discoveries").is_some(), "Should have discoveries section");
}
