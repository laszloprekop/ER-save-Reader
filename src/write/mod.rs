//! Dormant save write-back path (ADR-0009): compiled under
//! `--features save-writeback`, reachable from nothing. Kept so the path stays
//! resurrectable; `#[allow(dead_code)]` records that the deadness is expected
//! rather than accidental.
#![allow(dead_code)]

pub mod write;