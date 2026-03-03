//! Integration test harness for motif.
//!
//! Provides a full testing environment with real Metal rendering,
//! hit testing, and debug server integration.

pub mod harness;
pub mod hit_tree;

pub use harness::{TestHarness, TestRenderContext};
pub use hit_tree::{ElementId, HitEntry, HitTree};
