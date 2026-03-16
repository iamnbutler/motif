//! Hot-reloadable render function boundary.
//!
//! This module defines the stable API surface for live code patching via
//! [`cargo-hot`](https://github.com/DioxusLabs/dioxus/tree/main/packages/hot-reload)
//! and the [`subsecond`](https://crates.io/crates/subsecond) crate.
//!
//! # How it works
//!
//! Motif's hot reload works by patching function implementations at runtime
//! using a jump-table approach (`subsecond`). On every frame, `call` checks
//! whether a newer version of the render function has been compiled and, if so,
//! jumps to it transparently.
//!
//! # Stable boundary contract
//!
//! For hot patching to work, the types that cross the reload boundary must have
//! a **stable memory layout** between patches:
//!
//! - Function **signatures** (parameter types, return type) must not change.
//! - The **layout** of types that are borrowed across the boundary must not
//!   change (no adding/removing/reordering fields in those structs).
//!
//! **Safe to change between hot reloads** (no restart required):
//! - Function body: drawing commands, colors, text content, layout arithmetic.
//! - Local variables, control flow, helper function calls.
//! - New helper functions called from inside the render function.
//!
//! **Requires a full restart** (layout would change):
//! - Adding or removing fields in structs referenced across the boundary
//!   (e.g. adding a field to your `App` struct, then borrowing it in `render`).
//! - Changing the parameter list or return type of [`RenderFn`].
//!
//! # Example
//!
//! ```rust,no_run
//! use motif_core::{DrawContext, Point, Rect, ScaleFactor, Scene, Size, Srgba, TextContext};
//! use motif::hot;
//!
//! fn render(scene: &mut Scene, text_ctx: &mut TextContext, scale: ScaleFactor, size: (f32, f32)) {
//!     let mut cx = DrawContext::new(scene, scale);
//!     cx.paint_quad(
//!         Rect::new(Point::new(10.0, 10.0), Size::new(200.0, 100.0)),
//!         Srgba::new(0.2, 0.5, 1.0, 1.0),
//!     );
//! }
//!
//! fn run(scene: &mut Scene, text_ctx: &mut TextContext, scale: ScaleFactor, size: (f32, f32)) {
//!     // Connect once at startup (no-op without `hot` feature):
//!     hot::connect();
//!
//!     // Each frame:
//!     scene.clear();
//!     hot::call(render, scene, text_ctx, scale, size);
//! }
//! ```

use motif_core::{ScaleFactor, Scene, TextContext};

/// The canonical hot-reloadable render function signature.
///
/// All arguments are borrowed from outer application state that persists
/// across hot patches. This signature is **stable** — changing it requires a
/// full application restart.
///
/// | Parameter   | Description |
/// |-------------|-------------|
/// | `scene`     | Scene to paint into (caller clears it before each call) |
/// | `text_ctx`  | Text shaping / layout context |
/// | `scale`     | Current window DPI scale factor |
/// | `viewport`  | Logical viewport size `(width_pts, height_pts)` |
pub type RenderFn = fn(&mut Scene, &mut TextContext, ScaleFactor, (f32, f32));

/// Connect to the cargo-hot server for live patching.
///
/// Call this **once at application startup**, before the event loop runs.
/// When the `hot` feature is disabled this is a no-op, so the call is always
/// safe to leave in your code.
///
/// ```rust,no_run
/// fn main() {
///     motif::hot::connect(); // no-op without `--features hot`
///     // create event loop, etc.
/// }
/// ```
pub fn connect() {
    #[cfg(feature = "hot")]
    cargo_hot::connect();
}

/// Call a render function, routing through the hot-reload mechanism when the
/// `hot` feature is enabled.
///
/// | Feature state | Behaviour |
/// |---------------|-----------|
/// | `hot` **disabled** | Zero-overhead direct call — `f(scene, text_ctx, scale, size)`. |
/// | `hot` **enabled**  | Wraps the call in `subsecond::HotFn` so a live patch is applied on every frame. |
///
/// # Example
///
/// ```rust,no_run
/// use motif_core::{ScaleFactor, Scene, TextContext};
/// use motif::hot;
///
/// fn my_render(
///     scene: &mut Scene,
///     text_ctx: &mut TextContext,
///     scale: ScaleFactor,
///     size: (f32, f32),
/// ) {
///     // paint here
/// }
///
/// fn on_frame(scene: &mut Scene, text_ctx: &mut TextContext, scale: ScaleFactor, size: (f32, f32)) {
///     scene.clear();
///     hot::call(my_render, scene, text_ctx, scale, size);
/// }
/// ```
pub fn call(
    f: RenderFn,
    scene: &mut Scene,
    text_ctx: &mut TextContext,
    scale: ScaleFactor,
    size: (f32, f32),
) {
    #[cfg(feature = "hot")]
    {
        // Wrap in an Option so the outer closure is FnOnce, then move it into
        // the HotFn closure. The move is load-bearing for subsecond patching.
        let mut render_fn = Some(|| f(scene, text_ctx, scale, size));
        let mut hot_fn = cargo_hot::subsecond::HotFn::current(move || render_fn.take().unwrap()());
        hot_fn.call(());
    }
    #[cfg(not(feature = "hot"))]
    f(scene, text_ctx, scale, size);
}
