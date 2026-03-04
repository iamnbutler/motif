//! Debug server and devtools protocol for motif.
//!
//! Embeds a Unix domain socket server in any running motif app to enable
//! external tools to query and manipulate the scene.
//!
//! # Quick start
//!
//! ```no_run
//! use motif_debug::{DebugServer, SceneSnapshot};
//! use motif_core::Scene;
//!
//! // During app init:
//! let server = DebugServer::new().expect("failed to start debug server");
//!
//! // Each frame, after rendering:
//! let scene = Scene::new();
//! let snapshot = SceneSnapshot::from_scene(&scene, (800.0, 600.0), 2.0);
//! server.update_scene(snapshot);
//! ```

pub mod input_sim;
pub mod protocol;
pub mod screenshot;
pub mod server;
pub mod snapshot;

pub use input_sim::{SimResult, WindowPosition};
pub use protocol::{DebugError, DebugRequest, DebugResponse};
pub use screenshot::{capture_window_to_png, diff_screenshots, DiffResult};
pub use server::DebugServer;
pub use snapshot::{InputStateSnapshot, OverlayQuad, SceneSnapshot};
