//! Integration test harness for motif.
//!
//! Provides a full testing environment with real Metal rendering,
//! hit testing, and debug server integration.

pub mod harness;

pub use harness::{TestHarness, TestRenderContext};
// Re-export hit testing types from motif_core
pub use motif_core::{ElementId, HitEntry, HitTree};
