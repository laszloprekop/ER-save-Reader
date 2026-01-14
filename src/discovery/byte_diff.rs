/// Byte Differential Scanner
///
/// Compares raw event flag bytes between two saves without any formula assumptions.
/// This is the foundation for discovering unknown flag mappings.

use std::collections::HashMap;

/// A single bit change detected between two saves
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BitChange {
    /// Byte offset within event_flags array (0..1833374)
    pub byte_offset: usize,
    /// Bit position within the byte (0-7, where 7 is MSB)
    pub bit_position: u8,
    /// Value before the change
    pub before: bool,
    /// Value after the change
    pub after: bool,
}

impl BitChange {
    /// Returns true if this was a flag being set (false -> true)
    pub fn was_set(&self) -> bool {
        !self.before && self.after
    }

    /// Returns true if this was a flag being cleared (true -> false)
    pub fn was_cleared(&self) -> bool {
        self.before && !self.after
    }
}

/// Result of scanning two event flag arrays for differences
#[derive(Debug)]
pub struct ByteDiffResult {
    /// All individual bit changes detected
    pub bit_changes: Vec<BitChange>,
    /// Bytes that had any change (for quick reference)
    pub changed_bytes: Vec<usize>,
    /// Total bytes scanned
    pub bytes_scanned: usize,
    /// Statistics
    pub stats: DiffStats,
}

/// Statistics about the diff scan
#[derive(Debug, Default)]
pub struct DiffStats {
    /// Number of bits that were set (0 -> 1)
    pub bits_set: usize,
    /// Number of bits that were cleared (1 -> 0)
    pub bits_cleared: usize,
    /// Number of bytes that changed
    pub bytes_changed: usize,
    /// Byte offset of first change
    pub first_change_offset: Option<usize>,
    /// Byte offset of last change
    pub last_change_offset: Option<usize>,
}

/// Scanner for comparing event flag byte arrays
pub struct ByteDiffScanner {
    /// Optional: only scan specific byte ranges
    scan_ranges: Option<Vec<(usize, usize)>>,
    /// Optional: exclude certain byte ranges (known non-flag data)
    exclude_ranges: Vec<(usize, usize)>,
}

impl Default for ByteDiffScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteDiffScanner {
    pub fn new() -> Self {
        Self {
            scan_ranges: None,
            exclude_ranges: Vec::new(),
        }
    }

    /// Only scan specific byte ranges
    pub fn with_ranges(mut self, ranges: Vec<(usize, usize)>) -> Self {
        self.scan_ranges = Some(ranges);
        self
    }

    /// Exclude certain byte ranges from scanning
    pub fn exclude_ranges(mut self, ranges: Vec<(usize, usize)>) -> Self {
        self.exclude_ranges = ranges;
        self
    }

    /// Perform full byte-by-byte comparison of two event flag arrays
    pub fn scan(&self, before: &[u8], after: &[u8]) -> ByteDiffResult {
        let min_len = before.len().min(after.len());
        let mut bit_changes = Vec::new();
        let mut changed_bytes = Vec::new();
        let mut stats = DiffStats::default();

        // Determine which bytes to scan
        let ranges: Vec<(usize, usize)> = match &self.scan_ranges {
            Some(r) => r.clone(),
            None => vec![(0, min_len)],
        };

        let total_scanned: usize = ranges.iter().map(|(s, e)| e - s).sum();

        for (start, end) in ranges {
            let actual_end = end.min(min_len);

            for byte_offset in start..actual_end {
                // Skip excluded ranges
                if self.is_excluded(byte_offset) {
                    continue;
                }

                let before_byte = before[byte_offset];
                let after_byte = after[byte_offset];

                if before_byte != after_byte {
                    changed_bytes.push(byte_offset);
                    stats.bytes_changed += 1;

                    if stats.first_change_offset.is_none() {
                        stats.first_change_offset = Some(byte_offset);
                    }
                    stats.last_change_offset = Some(byte_offset);

                    // Find which bits changed
                    let xor = before_byte ^ after_byte;
                    for bit in 0..8 {
                        if (xor & (1 << bit)) != 0 {
                            let before_bit = (before_byte & (1 << bit)) != 0;
                            let after_bit = (after_byte & (1 << bit)) != 0;

                            // Convert to MSB-first bit position (7 = MSB, 0 = LSB)
                            let bit_position = 7 - bit;

                            if after_bit {
                                stats.bits_set += 1;
                            } else {
                                stats.bits_cleared += 1;
                            }

                            bit_changes.push(BitChange {
                                byte_offset,
                                bit_position: bit_position as u8,
                                before: before_bit,
                                after: after_bit,
                            });
                        }
                    }
                }
            }
        }

        ByteDiffResult {
            bit_changes,
            changed_bytes,
            bytes_scanned: total_scanned,
            stats,
        }
    }

    fn is_excluded(&self, offset: usize) -> bool {
        self.exclude_ranges.iter().any(|(start, end)| {
            offset >= *start && offset < *end
        })
    }
}

/// Group bit changes by contiguous byte regions for analysis
pub fn group_changes_by_region(changes: &[BitChange], gap_threshold: usize) -> Vec<ChangeRegion> {
    if changes.is_empty() {
        return Vec::new();
    }

    let mut sorted: Vec<_> = changes.iter().cloned().collect();
    sorted.sort_by_key(|c| c.byte_offset);

    let mut regions = Vec::new();
    let mut current_region = ChangeRegion {
        start_offset: sorted[0].byte_offset,
        end_offset: sorted[0].byte_offset,
        changes: vec![sorted[0].clone()],
    };

    for change in sorted.iter().skip(1) {
        if change.byte_offset <= current_region.end_offset + gap_threshold {
            // Extend current region
            current_region.end_offset = change.byte_offset;
            current_region.changes.push(change.clone());
        } else {
            // Start new region
            regions.push(current_region);
            current_region = ChangeRegion {
                start_offset: change.byte_offset,
                end_offset: change.byte_offset,
                changes: vec![change.clone()],
            };
        }
    }
    regions.push(current_region);

    regions
}

/// A contiguous region of changes
#[derive(Debug, Clone)]
pub struct ChangeRegion {
    pub start_offset: usize,
    pub end_offset: usize,
    pub changes: Vec<BitChange>,
}

impl ChangeRegion {
    pub fn size(&self) -> usize {
        self.end_offset - self.start_offset + 1
    }

    pub fn change_count(&self) -> usize {
        self.changes.len()
    }
}

/// Quick scan to find all regions that have any differences
pub fn find_changed_regions(before: &[u8], after: &[u8], block_size: usize) -> Vec<(usize, usize, usize)> {
    let min_len = before.len().min(after.len());
    let mut regions = Vec::new();

    let mut block_start = 0;
    while block_start < min_len {
        let block_end = (block_start + block_size).min(min_len);

        let mut diff_count = 0;
        for i in block_start..block_end {
            if before[i] != after[i] {
                diff_count += (before[i] ^ after[i]).count_ones() as usize;
            }
        }

        if diff_count > 0 {
            regions.push((block_start, block_end, diff_count));
        }

        block_start = block_end;
    }

    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_bit_change() {
        let before = vec![0b00000000, 0b00000000];
        let after = vec![0b00000000, 0b00000001];

        let scanner = ByteDiffScanner::new();
        let result = scanner.scan(&before, &after);

        assert_eq!(result.bit_changes.len(), 1);
        assert_eq!(result.bit_changes[0].byte_offset, 1);
        assert_eq!(result.bit_changes[0].bit_position, 7); // LSB = bit position 7 in MSB-first
        assert!(!result.bit_changes[0].before);
        assert!(result.bit_changes[0].after);
    }

    #[test]
    fn test_multiple_bit_changes() {
        let before = vec![0b11110000];
        let after = vec![0b00001111];

        let scanner = ByteDiffScanner::new();
        let result = scanner.scan(&before, &after);

        assert_eq!(result.bit_changes.len(), 8);
        assert_eq!(result.stats.bits_set, 4);
        assert_eq!(result.stats.bits_cleared, 4);
    }

    #[test]
    fn test_region_grouping() {
        let changes = vec![
            BitChange { byte_offset: 100, bit_position: 0, before: false, after: true },
            BitChange { byte_offset: 102, bit_position: 0, before: false, after: true },
            BitChange { byte_offset: 200, bit_position: 0, before: false, after: true },
        ];

        // With gap threshold of 10, first two should be in same region
        let regions = group_changes_by_region(&changes, 10);
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].changes.len(), 2);
        assert_eq!(regions[1].changes.len(), 1);
    }
}
