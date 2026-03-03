//! Integration test harness for motif.
//!
//! Provides a full testing environment with real Metal rendering,
//! hit testing, and debug server integration.

pub mod hit_tree;

pub use hit_tree::{ElementId, HitEntry, HitTree};
