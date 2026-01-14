/// Event Flag Discovery Module
///
/// Implements sophisticated techniques for discovering event flag mappings
/// by analyzing raw byte changes rather than relying on formula assumptions.
///
/// ## Core Approaches:
/// 1. **Byte Differential Scanner** - Compare saves, find changed bytes, reverse-engineer flags
/// 2. **Segment Analysis** - Map 1.8MB region by density to find flag vs data areas
/// 3. **Formula Fitting** - Use verified anchors to fit formula parameters
///
/// ## Key Insight:
/// Instead of: flag_id → formula → (byte, bit) → check value
/// We use: (byte_change, bit_change) → reverse_lookup → possible flag_ids → validate
///
/// ## Usage:
/// ```rust,ignore
/// use crate::discovery::integration::{run_discovery_workflow, run_differential_discovery};
///
/// // Run full discovery on a save slot
/// let result = run_discovery_workflow(save_path, slot_index)?;
///
/// // Compare two saves to find what changed
/// let diff = run_differential_discovery(before_path, after_path, slot_index)?;
/// ```

pub mod byte_diff;
pub mod segment_analysis;
pub mod reverse_lookup;
pub mod discovery_report;
pub mod integration;
pub mod offset_probe;
pub mod ground_truth_probe;
pub mod flag_catalog;
pub mod discovery_store;
pub mod snapshot_batch;
pub mod consensus;
pub mod cross_validator;
pub mod ground_truth_updater;

pub use byte_diff::ByteDiffScanner;
pub use segment_analysis::SegmentAnalyzer;
pub use reverse_lookup::FlagReverser;
pub use discovery_report::DiscoveryReport;
pub use integration::{
    run_discovery_workflow, run_differential_discovery,
    run_differential_discovery_with_persistence, run_offset_probing_with_persistence,
    differential_discovery_and_save,
};
pub use offset_probe::{
    probe_failing_flags, OffsetProber, ProbeResult, ProbeConfig,
    probe_and_persist, probe_and_save, persist_probe_results,
};
pub use flag_catalog::{FlagCatalog, CatalogFlag, CatalogError};
pub use discovery_store::{
    DiscoveryStore, StoredDiscovery, DiscoveryStatus,
    OffsetObservation, ObservationSource, StoreError, StoreSummary,
};
pub use snapshot_batch::{
    run_batch_analysis, batch_analyze_and_save, list_snapshot_pairs,
    get_snapshot_summary, SnapshotPair, SnapshotMetadata, BatchAnalysisResult,
};
pub use consensus::{
    ConsensusBuilder, ConsensusConfig, ConsensusResult, ConsensusStatus,
    ConsensusReport, SourceWeights,
};
pub use cross_validator::{
    CrossValidator, CrossValidationConfig, CrossValidationResult,
    batch_validate, BatchValidationResult,
};
pub use ground_truth_updater::{
    GroundTruthUpdater, UpdateConfig, PendingUpdate, UpdateResult, UpdateError,
    list_backups,
};
