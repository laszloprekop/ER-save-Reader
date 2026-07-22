mod profile_summary;
pub mod slot;
pub mod general;
pub mod stats;
pub mod events;
pub mod inventory;
pub mod regions;
// Character transplant: copies a slot from one save into another, so it only
// means anything if the result can be written back. Dormant with the rest of
// the write path (ADR-0009).
#[cfg(feature = "save-writeback")]
pub mod importer;
pub mod vm;
pub mod regulation;
pub mod equipment;
pub mod export;
pub mod verification_vm;