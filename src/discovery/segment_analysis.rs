/// Segment Analysis Module
///
/// Analyzes the raw event flags byte array to identify different segments
/// based on their bit density, entropy, and structural patterns.
///
/// Key insight: Flag regions have distinctive characteristics:
/// - Sparse bit patterns (1-15% density typical)
/// - Uniform distribution within active areas
/// - Clear boundaries between different flag categories

use std::collections::HashMap;

/// Statistics for a region of bytes
#[derive(Debug, Clone)]
pub struct RegionStats {
    pub start: usize,
    pub end: usize,
    pub total_bits: usize,
    pub set_bits: usize,
    pub density: f64,
    pub entropy: f64,
    pub pattern_score: f64,
}

impl RegionStats {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            total_bits: (end - start) * 8,
            set_bits: 0,
            density: 0.0,
            entropy: 0.0,
            pattern_score: 0.0,
        }
    }
}

/// Segment type based on analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SegmentType {
    /// Empty region (all zeros)
    Empty,
    /// Sparse flags (1-10% density)
    SparseFlags,
    /// Medium flags (10-30% density)
    MediumFlags,
    /// Dense flags (30-70% density)
    DenseFlags,
    /// Full region (all ones)
    Full,
    /// Non-flag data (high entropy, irregular patterns)
    Data,
    /// Unknown pattern
    Unknown,
}

impl SegmentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SegmentType::Empty => "empty",
            SegmentType::SparseFlags => "sparse_flags",
            SegmentType::MediumFlags => "medium_flags",
            SegmentType::DenseFlags => "dense_flags",
            SegmentType::Full => "full",
            SegmentType::Data => "data",
            SegmentType::Unknown => "unknown",
        }
    }
}

/// Analyzer for event flag byte arrays
pub struct SegmentAnalyzer {
    /// Block size for analysis (default 1024 bytes)
    block_size: usize,
}

impl Default for SegmentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SegmentAnalyzer {
    pub fn new() -> Self {
        Self { block_size: 1024 }
    }

    pub fn with_block_size(mut self, size: usize) -> Self {
        self.block_size = size;
        self
    }

    /// Analyze the entire event flags array and return segment statistics
    pub fn analyze(&self, data: &[u8]) -> SegmentAnalysisResult {
        let mut blocks = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let end = (offset + self.block_size).min(data.len());
            let block_data = &data[offset..end];

            let stats = self.analyze_block(offset, end, block_data);
            blocks.push(stats);

            offset = end;
        }

        let segments = self.classify_segments(&blocks);
        let density_map = self.build_density_map(&blocks);

        SegmentAnalysisResult {
            total_size: data.len(),
            block_size: self.block_size,
            blocks,
            segments,
            density_map,
        }
    }

    fn analyze_block(&self, start: usize, end: usize, data: &[u8]) -> RegionStats {
        let mut stats = RegionStats::new(start, end);

        // Count set bits
        for byte in data {
            stats.set_bits += byte.count_ones() as usize;
        }

        stats.density = stats.set_bits as f64 / stats.total_bits as f64;

        // Calculate entropy (Shannon entropy of byte values)
        let mut byte_counts: HashMap<u8, usize> = HashMap::new();
        for &byte in data {
            *byte_counts.entry(byte).or_insert(0) += 1;
        }

        let total_bytes = data.len() as f64;
        stats.entropy = byte_counts.values()
            .map(|&count| {
                let p = count as f64 / total_bytes;
                if p > 0.0 { -p * p.log2() } else { 0.0 }
            })
            .sum();

        // Calculate pattern score (how "flag-like" the region looks)
        // Flag regions typically have:
        // - Low to medium density
        // - Low entropy (many zeros, few unique byte values)
        // - No repeating structural patterns (unlike packed data)
        stats.pattern_score = self.calculate_pattern_score(data, stats.density, stats.entropy);

        stats
    }

    fn calculate_pattern_score(&self, data: &[u8], density: f64, entropy: f64) -> f64 {
        // Heuristic scoring:
        // - Ideal flag density is 1-20%
        // - Ideal entropy for sparse flags is 0-2 bits
        // - Penalize patterns that look like structured data

        let density_score = if density < 0.01 {
            0.3 // Too empty, might be unused
        } else if density < 0.05 {
            1.0 // Perfect for sparse flags
        } else if density < 0.15 {
            0.9 // Good for flags
        } else if density < 0.30 {
            0.7 // Medium density
        } else if density < 0.70 {
            0.5 // High density, might be data
        } else {
            0.2 // Very dense, likely not flags
        };

        let entropy_score = if entropy < 1.0 {
            1.0 // Very low entropy (sparse data)
        } else if entropy < 2.0 {
            0.9
        } else if entropy < 4.0 {
            0.6
        } else if entropy < 6.0 {
            0.3
        } else {
            0.1 // High entropy, likely random data
        };

        // Check for repeating patterns (non-flag data)
        let pattern_penalty = self.detect_repeating_patterns(data);

        (density_score * entropy_score) * (1.0 - pattern_penalty)
    }

    fn detect_repeating_patterns(&self, data: &[u8]) -> f64 {
        if data.len() < 8 {
            return 0.0;
        }

        // Check for 4-byte repeating patterns (common in structured data)
        let mut repeat_count = 0;
        for i in 4..data.len() {
            if data[i] == data[i - 4] {
                repeat_count += 1;
            }
        }

        let repeat_ratio = repeat_count as f64 / (data.len() - 4) as f64;

        // If more than 50% of bytes repeat at 4-byte intervals, it's likely structured data
        if repeat_ratio > 0.5 { repeat_ratio } else { 0.0 }
    }

    fn classify_segments(&self, blocks: &[RegionStats]) -> Vec<Segment> {
        let mut segments: Vec<Segment> = Vec::new();

        for (i, block) in blocks.iter().enumerate() {
            let seg_type = self.classify_block(block);

            // Merge with previous if same type
            if let Some(last) = segments.last_mut() {
                if last.segment_type == seg_type {
                    last.end = block.end;
                    last.stats.push(block.clone());
                    continue;
                }
            }

            segments.push(Segment {
                start: block.start,
                end: block.end,
                segment_type: seg_type,
                stats: vec![block.clone()],
            });
        }

        segments
    }

    fn classify_block(&self, stats: &RegionStats) -> SegmentType {
        if stats.density < 0.001 {
            SegmentType::Empty
        } else if stats.density > 0.99 {
            SegmentType::Full
        } else if stats.pattern_score < 0.3 {
            SegmentType::Data
        } else if stats.density < 0.10 {
            SegmentType::SparseFlags
        } else if stats.density < 0.30 {
            SegmentType::MediumFlags
        } else if stats.density < 0.70 {
            SegmentType::DenseFlags
        } else {
            SegmentType::Unknown
        }
    }

    fn build_density_map(&self, blocks: &[RegionStats]) -> Vec<(usize, f64)> {
        blocks.iter()
            .map(|b| (b.start, b.density))
            .collect()
    }
}

/// A detected segment in the event flags array
#[derive(Debug, Clone)]
pub struct Segment {
    pub start: usize,
    pub end: usize,
    pub segment_type: SegmentType,
    pub stats: Vec<RegionStats>,
}

impl Segment {
    pub fn size(&self) -> usize {
        self.end - self.start
    }

    pub fn avg_density(&self) -> f64 {
        if self.stats.is_empty() {
            return 0.0;
        }
        self.stats.iter().map(|s| s.density).sum::<f64>() / self.stats.len() as f64
    }
}

/// Result of segment analysis
#[derive(Debug)]
pub struct SegmentAnalysisResult {
    pub total_size: usize,
    pub block_size: usize,
    pub blocks: Vec<RegionStats>,
    pub segments: Vec<Segment>,
    pub density_map: Vec<(usize, f64)>,
}

impl SegmentAnalysisResult {
    /// Get all segments of a specific type
    pub fn segments_of_type(&self, seg_type: SegmentType) -> Vec<&Segment> {
        self.segments.iter()
            .filter(|s| s.segment_type == seg_type)
            .collect()
    }

    /// Get total bytes covered by flag-type segments
    pub fn flag_bytes(&self) -> usize {
        self.segments.iter()
            .filter(|s| matches!(
                s.segment_type,
                SegmentType::SparseFlags | SegmentType::MediumFlags | SegmentType::DenseFlags
            ))
            .map(|s| s.size())
            .sum()
    }

    /// Get total bytes covered by empty segments
    pub fn empty_bytes(&self) -> usize {
        self.segments_of_type(SegmentType::Empty)
            .iter()
            .map(|s| s.size())
            .sum()
    }

    /// Print a summary of the analysis
    pub fn print_summary(&self) {
        println!("\n=== Segment Analysis Summary ===");
        println!("Total size: {} bytes ({:.2} MB)", self.total_size, self.total_size as f64 / 1_048_576.0);
        println!("Block size: {} bytes", self.block_size);
        println!("Segments found: {}", self.segments.len());

        println!("\nSegment breakdown:");
        let mut type_counts: HashMap<SegmentType, (usize, usize)> = HashMap::new();
        for seg in &self.segments {
            let entry = type_counts.entry(seg.segment_type).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += seg.size();
        }

        for (seg_type, (count, size)) in &type_counts {
            let pct = (*size as f64 / self.total_size as f64) * 100.0;
            println!("  {:15} {:3} segments, {:8} bytes ({:5.1}%)",
                seg_type.as_str(), count, size, pct);
        }

        println!("\nFlag coverage: {:.1}%", (self.flag_bytes() as f64 / self.total_size as f64) * 100.0);
    }

    /// Find potential undiscovered flag regions
    pub fn find_potential_flag_regions(&self) -> Vec<(usize, usize, String)> {
        let mut regions = Vec::new();

        for seg in &self.segments {
            match seg.segment_type {
                SegmentType::Unknown => {
                    regions.push((seg.start, seg.end, "unknown - needs investigation".to_string()));
                }
                SegmentType::SparseFlags | SegmentType::MediumFlags | SegmentType::DenseFlags => {
                    // These are likely flags - check if they're in known formula ranges
                    regions.push((seg.start, seg.end, format!("{} - potential flags", seg.segment_type.as_str())));
                }
                _ => {}
            }
        }

        regions
    }
}

/// Quick scan to find "hot" regions (any non-zero bytes)
pub fn find_hot_regions(data: &[u8], min_gap: usize) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut in_region = false;
    let mut region_start = 0;
    let mut last_nonzero = 0;

    for (i, &byte) in data.iter().enumerate() {
        if byte != 0 {
            if !in_region {
                in_region = true;
                region_start = i;
            }
            last_nonzero = i;
        } else if in_region && i - last_nonzero > min_gap {
            // End current region
            regions.push((region_start, last_nonzero + 1));
            in_region = false;
        }
    }

    if in_region {
        regions.push((region_start, last_nonzero + 1));
    }

    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_region() {
        let data = vec![0u8; 1024];
        let analyzer = SegmentAnalyzer::new();
        let result = analyzer.analyze(&data);

        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].segment_type, SegmentType::Empty);
    }

    #[test]
    fn test_sparse_flags() {
        let mut data = vec![0u8; 1024];
        // Set ~5% of bits (8192 total bits, need ~410 set)
        // Setting every 2nd byte with 0xFF gives us 512*8 = 4096 bits (50%)
        // Let's set every 2nd byte with 0x0F (4 bits) = 512*4 = 2048 bits (~25%)
        // Actually, let's be more precise: set ~5% = 410 bits
        // We'll set every ~20th bit by setting bytes strategically
        for i in (0..1024).step_by(3) {
            data[i] = 0x11; // 2 bits per byte, 341 bytes = 682 bits = ~8%
        }

        let analyzer = SegmentAnalyzer::new();
        let result = analyzer.analyze(&data);

        // Should be classified as sparse/medium flags
        assert!(
            matches!(
                result.segments[0].segment_type,
                SegmentType::SparseFlags | SegmentType::MediumFlags
            ),
            "Expected SparseFlags or MediumFlags, got {:?} with density {:.2}%",
            result.segments[0].segment_type,
            result.segments[0].avg_density() * 100.0
        );
    }

    #[test]
    fn test_hot_regions() {
        let mut data = vec![0u8; 1000];
        data[100] = 0xFF;
        data[101] = 0xFF;
        data[500] = 0xFF;

        let regions = find_hot_regions(&data, 10);

        // Should find 2 regions (100-101 and 500)
        assert_eq!(regions.len(), 2);
    }
}
