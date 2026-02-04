/// Reverse Flag ID Lookup
///
/// Given a (byte_offset, bit_position), reverse-engineer which flag_id(s)
/// could possibly map to that location.
///
/// This is the inverse of the forward formula: flag_id → (byte, bit)
/// Instead we compute: (byte, bit) → possible flag_ids

use std::collections::HashSet;

use crate::db::pickup_flags::{
    TILE_BASE_OFFSET, TILE_BYTES_PER_SLOT, TILE_SLOTS_PER_ROW,
    TILE_ROW_BASE, TILE_COL_BASE, MAX_TILE_LOCAL_ID, DUNGEON_SECTION_SIZE,
    DUNGEON_BASE_OFFSETS,
};
use crate::generated::ground_truth::VERIFIED_BLOCK_BASES;

/// Possible flag types that could match a (byte, bit) position
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PossibleFlagType {
    /// Simple flag (0-59999)
    Simple(u32),
    /// Block flag (60000-99999)
    Block { flag_id: u32, block_start: u32 },
    /// Tile flag (10-digit: 1XXYYZZZZ)
    Tile { flag_id: u32, row: u32, col: u32, local_id: u32 },
    /// Dungeon flag (8-digit: AASSZZZZ)
    Dungeon { flag_id: u32, area: u32, section: u32, local_id: u32 },
    /// Unknown - byte is in unmapped region
    Unknown { byte_offset: usize, bit_position: u8 },
}

impl PossibleFlagType {
    pub fn flag_id(&self) -> Option<u32> {
        match self {
            PossibleFlagType::Simple(id) => Some(*id),
            PossibleFlagType::Block { flag_id, .. } => Some(*flag_id),
            PossibleFlagType::Tile { flag_id, .. } => Some(*flag_id),
            PossibleFlagType::Dungeon { flag_id, .. } => Some(*flag_id),
            PossibleFlagType::Unknown { .. } => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            PossibleFlagType::Simple(_) => "simple",
            PossibleFlagType::Block { .. } => "block",
            PossibleFlagType::Tile { .. } => "tile",
            PossibleFlagType::Dungeon { .. } => "dungeon",
            PossibleFlagType::Unknown { .. } => "unknown",
        }
    }
}

/// Reverse lookup engine for finding possible flag IDs
pub struct FlagReverser {
    /// Cached reverse mapping: byte_offset → block_start (for block flags)
    block_byte_ranges: Vec<(usize, usize, u32)>, // (start_byte, end_byte, block_start)
    /// Cached dungeon mapping: byte_offset → (area, section)
    dungeon_byte_ranges: Vec<(usize, usize, u32, u32)>, // (start_byte, end_byte, area, section)
}

impl FlagReverser {
    pub fn new() -> Self {
        let mut reverser = Self {
            block_byte_ranges: Vec::new(),
            dungeon_byte_ranges: Vec::new(),
        };
        reverser.build_caches();
        reverser
    }

    fn build_caches(&mut self) {
        // Build block ranges from VERIFIED_BLOCK_BASES
        for (block_start, block_info) in VERIFIED_BLOCK_BASES.iter() {
            let start_byte = block_info.base_offset as usize;
            let end_byte = start_byte + 125; // 1000 flags / 8 bits = 125 bytes per block
            self.block_byte_ranges.push((start_byte, end_byte, *block_start));
        }
        self.block_byte_ranges.sort_by_key(|(s, _, _)| *s);

        // Build dungeon ranges from DUNGEON_BASE_OFFSETS
        for (key, &base) in DUNGEON_BASE_OFFSETS.iter() {
            let parts: Vec<&str> = key.split('_').collect();
            if parts.len() == 2 {
                if let (Ok(area), Ok(section)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    let start_byte = base as usize;
                    let end_byte = start_byte + DUNGEON_SECTION_SIZE as usize;
                    self.dungeon_byte_ranges.push((start_byte, end_byte, area, section));
                }
            }
        }
        self.dungeon_byte_ranges.sort_by_key(|(s, _, _, _)| *s);
    }

    /// Given (byte_offset, bit_position), find all possible flag IDs that could map there
    pub fn reverse_lookup(&self, byte_offset: usize, bit_position: u8) -> Vec<PossibleFlagType> {
        let mut results = Vec::new();

        // IMPORTANT: Check block flags FIRST - they are verified and more specific
        // than simple flags. Block 71000+ can have base_offsets < 7500, which would
        // otherwise incorrectly match simple flag calculation.
        // NOTE: Multiple blocks can overlap at the same byte offset, so collect ALL matches
        let block_flags = self.try_reverse_blocks(byte_offset, bit_position);
        results.extend(block_flags);

        // Try simple flag (0-59999): byte = flag_id / 8, bit = 7 - (flag_id % 8)
        // Only if no block flag matched at this offset
        if results.is_empty() && byte_offset < 7500 { // 60000 / 8 = 7500
            let flag_id = (byte_offset * 8 + (7 - bit_position as usize)) as u32;
            if flag_id < 60000 {
                results.push(PossibleFlagType::Simple(flag_id));
            }
        }

        // Try tile flag (10-digit)
        if let Some(tile_flag) = self.try_reverse_tile(byte_offset, bit_position) {
            results.push(tile_flag);
        }

        // Try dungeon flag (8-digit)
        if let Some(dungeon_flag) = self.try_reverse_dungeon(byte_offset, bit_position) {
            results.push(dungeon_flag);
        }

        // If nothing matched, mark as unknown
        if results.is_empty() {
            results.push(PossibleFlagType::Unknown { byte_offset, bit_position });
        }

        results
    }

    /// Find ALL blocks that could map to this byte offset (blocks can overlap)
    fn try_reverse_blocks(&self, byte_offset: usize, bit_position: u8) -> Vec<PossibleFlagType> {
        let mut results = Vec::new();

        for (start, end, block_start) in &self.block_byte_ranges {
            if byte_offset >= *start && byte_offset < *end {
                // Calculate flag_id within block
                let relative_byte = byte_offset - *start;
                let local_offset = relative_byte * 8 + (7 - bit_position as usize);
                let flag_id = *block_start + local_offset as u32;

                if flag_id < *block_start + 1000 {
                    results.push(PossibleFlagType::Block {
                        flag_id,
                        block_start: *block_start,
                    });
                }
            }
        }
        results
    }

    fn try_reverse_tile(&self, byte_offset: usize, bit_position: u8) -> Option<PossibleFlagType> {
        let tile_base = TILE_BASE_OFFSET as usize;
        let bytes_per_slot = TILE_BYTES_PER_SLOT as usize;

        if byte_offset < tile_base {
            return None;
        }

        let relative = byte_offset - tile_base;
        let slot = relative / bytes_per_slot;
        let local_byte = relative % bytes_per_slot;
        let local_id = (local_byte * 8 + (7 - bit_position as usize)) as u32;

        // Only valid if local_id <= MAX_TILE_LOCAL_ID
        if local_id > MAX_TILE_LOCAL_ID {
            return None;
        }

        // Reverse slot to (row, col)
        let row = (slot / TILE_SLOTS_PER_ROW as usize) as u32 + TILE_ROW_BASE;
        let col = (slot % TILE_SLOTS_PER_ROW as usize) as u32 + TILE_COL_BASE;

        // Construct 10-digit flag ID: 1XXYYZZZZ
        let flag_id = 1_000_000_000 + row * 1_000_000 + col * 10_000 + local_id;

        Some(PossibleFlagType::Tile {
            flag_id,
            row,
            col,
            local_id,
        })
    }

    fn try_reverse_dungeon(&self, byte_offset: usize, bit_position: u8) -> Option<PossibleFlagType> {
        // Find which dungeon section this byte falls into
        for (start, end, area, section) in &self.dungeon_byte_ranges {
            if byte_offset >= *start && byte_offset < *end {
                let relative_byte = byte_offset - *start;
                let local_id = (relative_byte * 8 + (7 - bit_position as usize)) as u32;

                if local_id < 10000 {
                    // Construct 8-digit flag ID: AASSZZZZ
                    let flag_id = *area * 1_000_000 + *section * 10_000 + local_id;
                    return Some(PossibleFlagType::Dungeon {
                        flag_id,
                        area: *area,
                        section: *section,
                        local_id,
                    });
                }
            }
        }
        None
    }

    /// Analyze a region of bytes to find all possible flag interpretations
    pub fn analyze_region(&self, start: usize, end: usize) -> RegionAnalysis {
        let mut analysis = RegionAnalysis {
            start_offset: start,
            end_offset: end,
            possible_types: HashSet::new(),
            sample_flags: Vec::new(),
        };

        // Sample a few positions to determine what types are possible
        let step = ((end - start) / 10).max(1);
        for byte in (start..end).step_by(step) {
            for bit in 0..8 {
                for result in self.reverse_lookup(byte, bit) {
                    analysis.possible_types.insert(result.type_name().to_string());
                    if analysis.sample_flags.len() < 20 {
                        analysis.sample_flags.push(result);
                    }
                }
            }
        }

        analysis
    }
}

impl Default for FlagReverser {
    fn default() -> Self {
        Self::new()
    }
}

/// Analysis of a byte region
#[derive(Debug)]
pub struct RegionAnalysis {
    pub start_offset: usize,
    pub end_offset: usize,
    pub possible_types: HashSet<String>,
    pub sample_flags: Vec<PossibleFlagType>,
}

/// Map the entire event flags array to identify which regions contain which flag types
pub fn map_flag_regions(event_flags_size: usize) -> Vec<FlagRegion> {
    let reverser = FlagReverser::new();
    let mut regions = Vec::new();

    // Scan in 1KB blocks
    let block_size = 1024;
    let mut offset = 0;

    while offset < event_flags_size {
        let end = (offset + block_size).min(event_flags_size);
        let analysis = reverser.analyze_region(offset, end);

        // Determine primary type
        let primary_type = if analysis.possible_types.contains("tile") {
            "tile"
        } else if analysis.possible_types.contains("dungeon") {
            "dungeon"
        } else if analysis.possible_types.contains("block") {
            "block"
        } else if analysis.possible_types.contains("simple") {
            "simple"
        } else {
            "unknown"
        };

        regions.push(FlagRegion {
            start: offset,
            end,
            primary_type: primary_type.to_string(),
            possible_types: analysis.possible_types,
        });

        offset = end;
    }

    // Merge adjacent regions with same type
    merge_regions(regions)
}

fn merge_regions(regions: Vec<FlagRegion>) -> Vec<FlagRegion> {
    if regions.is_empty() {
        return regions;
    }

    let mut merged = Vec::new();
    let mut current = regions[0].clone();

    for region in regions.into_iter().skip(1) {
        if region.primary_type == current.primary_type {
            current.end = region.end;
            current.possible_types.extend(region.possible_types);
        } else {
            merged.push(current);
            current = region;
        }
    }
    merged.push(current);

    merged
}

/// A mapped region of the event flags array
#[derive(Debug, Clone)]
pub struct FlagRegion {
    pub start: usize,
    pub end: usize,
    pub primary_type: String,
    pub possible_types: HashSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pickup_flags::get_flag_offset;

    #[test]
    fn test_simple_reverse() {
        let reverser = FlagReverser::new();

        // Flag 300 should be at byte 37, bit 3
        // Reverse: byte 37, bit 3 should give flag 300
        let results = reverser.reverse_lookup(37, 3);
        assert!(results.iter().any(|r| matches!(r, PossibleFlagType::Simple(300))));
    }

    #[test]
    fn test_round_trip_simple() {
        let reverser = FlagReverser::new();

        // Test round-trip for several simple flags
        for flag_id in [100, 500, 1000, 5000, 50000] {
            if let Some((byte, bit)) = get_flag_offset(flag_id) {
                let results = reverser.reverse_lookup(byte as usize, bit);
                assert!(
                    results.iter().any(|r| r.flag_id() == Some(flag_id)),
                    "Flag {} at ({}, {}) did not reverse correctly",
                    flag_id, byte, bit
                );
            }
        }
    }

    #[test]
    fn test_round_trip_block() {
        let reverser = FlagReverser::new();

        // Test grace flags (block 76000)
        for flag_id in [76100, 76101, 76102, 76150] {
            if let Some((byte, bit)) = get_flag_offset(flag_id) {
                let results = reverser.reverse_lookup(byte as usize, bit);
                assert!(
                    results.iter().any(|r| r.flag_id() == Some(flag_id)),
                    "Block flag {} at ({}, {}) did not reverse correctly. Results: {:?}",
                    flag_id, byte, bit, results
                );
            }
        }
    }

    #[test]
    fn test_round_trip_tile() {
        let reverser = FlagReverser::new();

        // Test tile flag
        let flag_id = 1042370100u32;
        if let Some((byte, bit)) = get_flag_offset(flag_id) {
            let results = reverser.reverse_lookup(byte as usize, bit);
            assert!(
                results.iter().any(|r| matches!(r, PossibleFlagType::Tile { .. })),
                "Tile flag {} at ({}, {}) did not reverse to tile type",
                flag_id, byte, bit
            );
        }
    }
}
