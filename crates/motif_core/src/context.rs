//! DrawContext provides a painter's stack for building scenes.

use crate::{
    AccessId, AccessNode, AccessRole, AccessTree, DevicePoint, DeviceRect, FocusHandle, FocusId,
    FocusState, Point, Quad, Rect, ScaleFactor, Scene, Size, TextContext, TextRun,
};
use palette::Srgba;

/// Painter's stack for hierarchical drawing.
pub struct DrawContext<'a> {
    scene: &'a mut Scene,
    access_tree: Option<&'a mut AccessTree>,
    focus_state: Option<&'a mut FocusState>,
    scale_factor: ScaleFactor,
    offset_stack: Vec<Point>,
    clip_stack: Vec<Rect>,
    next_access_id: u64,
}

impl<'a> DrawContext<'a> {
    pub fn new(scene: &'a mut Scene, scale_factor: ScaleFactor) -> Self {
        Self {
            scene,
            access_tree: None,
            focus_state: None,
            scale_factor,
            offset_stack: vec![Point::new(0.0, 0.0)],
            clip_stack: Vec::new(),
            next_access_id: 1,
        }
    }

    /// Create a DrawContext with accessibility support.
    ///
    /// When enabled, `paint_text()` will also create AccessNodes in the AccessTree.
    pub fn with_accessibility(
        scene: &'a mut Scene,
        access_tree: &'a mut AccessTree,
        scale_factor: ScaleFactor,
    ) -> Self {
        Self {
            scene,
            access_tree: Some(access_tree),
            focus_state: None,
            scale_factor,
            offset_stack: vec![Point::new(0.0, 0.0)],
            clip_stack: Vec::new(),
            next_access_id: 1,
        }
    }

    /// Create a DrawContext with focus management support.
    ///
    /// When enabled, `request_focus()`, `is_focused()`, and `blur()` will
    /// read and write the provided [`FocusState`], allowing elements to
    /// claim keyboard focus during their render pass.
    pub fn with_focus(
        scene: &'a mut Scene,
        focus_state: &'a mut FocusState,
        scale_factor: ScaleFactor,
    ) -> Self {
        Self {
            scene,
            access_tree: None,
            focus_state: Some(focus_state),
            scale_factor,
            offset_stack: vec![Point::new(0.0, 0.0)],
            clip_stack: Vec::new(),
            next_access_id: 1,
        }
    }

    /// Attach a [`FocusState`] to an existing context.
    ///
    /// This is a builder-style method so you can combine focus with an
    /// already-constructed context (e.g. one created by `with_accessibility`).
    ///
    /// ```ignore
    /// let mut cx = DrawContext::with_accessibility(&mut scene, &mut tree, scale)
    ///     .with_focus_state(&mut focus_state);
    /// ```
    pub fn with_focus_state(mut self, focus_state: &'a mut FocusState) -> Self {
        self.focus_state = Some(focus_state);
        self
    }

    // ── Focus management ──────────────────────────────────────────────────────

    /// Create a new [`FocusHandle`] for a focusable element.
    ///
    /// Elements should call this once (e.g. during construction or the first
    /// render pass) and store the returned handle. The handle can later be
    /// passed to [`is_focused`](Self::is_focused) and
    /// [`request_focus`](Self::request_focus).
    ///
    /// The handle is self-contained — its ID is globally unique regardless of
    /// whether a `FocusState` is attached to this context.
    pub fn focus_handle(&self) -> FocusHandle {
        FocusHandle::new()
    }

    /// Check whether the given handle currently holds keyboard focus.
    ///
    /// Returns `false` when no [`FocusState`] was attached to this context.
    pub fn is_focused(&self, handle: &FocusHandle) -> bool {
        self.focus_state
            .as_ref()
            .map(|s| handle.is_focused(s))
            .unwrap_or(false)
    }

    /// Request keyboard focus for the given handle.
    ///
    /// Emits a [`FocusEvent::Focus`](crate::FocusEvent::Focus) (and a
    /// [`FocusEvent::Blur`](crate::FocusEvent::Blur) for the previous holder)
    /// into the attached [`FocusState`]. Does nothing if no `FocusState` was
    /// attached.
    ///
    /// Returns the previously focused [`FocusId`], if any.
    pub fn request_focus(&mut self, handle: &FocusHandle) -> Option<FocusId> {
        self.focus_state.as_mut().and_then(|s| handle.focus(s))
    }

    /// Remove keyboard focus from the currently focused element.
    ///
    /// Emits a [`FocusEvent::Blur`](crate::FocusEvent::Blur) into the attached
    /// [`FocusState`]. Does nothing if no `FocusState` was attached.
    ///
    /// Returns the previously focused [`FocusId`], if any.
    pub fn blur(&mut self) -> Option<FocusId> {
        self.focus_state.as_mut().and_then(|s| s.blur())
    }

    // ── Accessibility ─────────────────────────────────────────────────────────

    /// Generate a unique AccessId for accessibility nodes.
    fn next_access_id(&mut self) -> AccessId {
        let id = AccessId(self.next_access_id);
        self.next_access_id += 1;
        id
    }

    /// Current offset (sum of all pushed offsets).
    fn current_offset(&self) -> Point {
        self.offset_stack.last().copied().unwrap_or_default()
    }

    /// Execute closure with additional offset applied.
    pub fn with_offset<R>(&mut self, offset: Point, f: impl FnOnce(&mut Self) -> R) -> R {
        let current = self.current_offset();
        let new_offset = Point::new(current.x + offset.x, current.y + offset.y);
        self.offset_stack.push(new_offset);
        let result = f(self);
        self.offset_stack.pop();
        result
    }

    /// Execute closure with clip bounds applied.
    pub fn with_clip<R>(&mut self, bounds: Rect, f: impl FnOnce(&mut Self) -> R) -> R {
        // Transform clip bounds by current offset
        let offset = self.current_offset();
        let clipped = Rect::new(
            Point::new(bounds.origin.x + offset.x, bounds.origin.y + offset.y),
            bounds.size,
        );
        self.clip_stack.push(clipped);
        let result = f(self);
        self.clip_stack.pop();
        result
    }

    /// Paint a simple filled quad.
    pub fn paint_quad(&mut self, bounds: Rect, fill: impl Into<Srgba>) {
        let mut quad = Quad::new(self.to_device_rect(bounds), fill);
        self.apply_clip(&mut quad);
        self.scene.push_quad(quad);
    }

    /// Paint a quad with full control.
    pub fn paint(&mut self, mut quad: Quad) {
        self.apply_clip(&mut quad);
        self.scene.push_quad(quad);
    }

    /// Apply current clip stack to quad.
    fn apply_clip(&self, quad: &mut Quad) {
        if let Some(clip) = self.clip_stack.last() {
            quad.clip_bounds = Some(self.scale_factor.scale_rect(*clip));
        }
    }

    /// Convert logical rect to device rect, applying current offset and scale.
    fn to_device_rect(&self, rect: Rect) -> DeviceRect {
        let offset = self.current_offset();
        let origin = Point::new(rect.origin.x + offset.x, rect.origin.y + offset.y);
        let scaled_origin = self.scale_factor.scale_point(origin);
        let scaled_size = self.scale_factor.scale_size(rect.size);
        DeviceRect::new(scaled_origin, scaled_size)
    }

    /// Convert logical point to device point, applying current offset and scale.
    fn to_device_point(&self, point: Point) -> DevicePoint {
        let offset = self.current_offset();
        let origin = Point::new(point.x + offset.x, point.y + offset.y);
        self.scale_factor.scale_point(origin)
    }

    /// Paint text at the given position.
    ///
    /// The position is the baseline origin (left side of first glyph baseline).
    /// If accessibility is enabled, also creates an AccessNode for screen readers.
    pub fn paint_text(
        &mut self,
        text: &str,
        position: Point,
        font_size: f32,
        color: impl Into<Srgba>,
        text_ctx: &mut TextContext,
    ) {
        let layout = text_ctx.layout_text(text, font_size * self.scale_factor.0);
        let device_position = self.to_device_point(position);
        let color = color.into();

        // Get the baseline offset from the first line so we can position correctly.
        // positioned_glyphs() returns y values relative to layout top, with baseline added.
        // We need to subtract baseline so the text baseline lands at the specified position.
        let line_metrics = layout.line_metrics();
        let baseline_offset = line_metrics.first().map(|m| m.baseline).unwrap_or(0.0);

        let device_origin =
            DevicePoint::new(device_position.x, device_position.y - baseline_offset);

        // Create accessibility node if enabled
        if self.access_tree.is_some() {
            let offset = self.current_offset();

            // Calculate text bounds in physical pixel coordinates for AccessKit
            // AccessKit (via winit) expects window-relative physical pixels
            let ascent = line_metrics.first().map(|m| m.ascent).unwrap_or(0.0);
            let descent = line_metrics.first().map(|m| m.descent).unwrap_or(0.0);

            let scale = self.scale_factor.0;

            // Physical pixel position and size
            let bounds = Rect::new(
                Point::new(
                    (position.x + offset.x) * scale,
                    (position.y + offset.y) * scale - ascent,
                ),
                Size::new(layout.width(), ascent + descent),
            );

            let access_id = self.next_access_id();
            let node =
                AccessNode::new(access_id, AccessRole::Label, text.to_string()).with_bounds(bounds);

            // We need to use the access_tree, but it's behind Option<&mut>
            // Take it temporarily to satisfy borrow checker
            if let Some(tree) = self.access_tree.as_mut() {
                tree.push(node);
            }
        }

        for run in layout.glyph_runs_with_font() {
            if let Some(font) = run.font_data {
                let mut text_run = TextRun::new(device_origin, color, run.font_size, font);
                text_run.normalized_coords = run.normalized_coords;

                for glyph in run.glyphs {
                    text_run.push_glyph(glyph.id, glyph.x, glyph.y);
                }

                self.scene.push_text_run(text_run);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FocusState, Size, TextContext};

    #[test]
    fn offset_stacking() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::new(&mut scene, scale);

        // Paint at origin
        cx.paint_quad(
            Rect::new(Point::new(0.0, 0.0), Size::new(10.0, 10.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        );

        // Paint with offset
        cx.with_offset(Point::new(100.0, 50.0), |cx| {
            cx.paint_quad(
                Rect::new(Point::new(0.0, 0.0), Size::new(10.0, 10.0)),
                Srgba::new(0.0, 1.0, 0.0, 1.0),
            );
        });

        assert_eq!(scene.quad_count(), 2);

        let quads = scene.quads();
        // First quad at origin
        assert_eq!(quads[0].bounds.origin.x, 0.0);
        assert_eq!(quads[0].bounds.origin.y, 0.0);
        // Second quad offset by (100, 50)
        assert_eq!(quads[1].bounds.origin.x, 100.0);
        assert_eq!(quads[1].bounds.origin.y, 50.0);
    }

    #[test]
    fn nested_offsets() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::new(&mut scene, scale);

        cx.with_offset(Point::new(10.0, 10.0), |cx| {
            cx.with_offset(Point::new(5.0, 5.0), |cx| {
                cx.paint_quad(
                    Rect::new(Point::new(0.0, 0.0), Size::new(10.0, 10.0)),
                    Srgba::new(1.0, 0.0, 0.0, 1.0),
                );
            });
        });

        let quads = scene.quads();
        // Nested offsets should accumulate: 10+5 = 15
        assert_eq!(quads[0].bounds.origin.x, 15.0);
        assert_eq!(quads[0].bounds.origin.y, 15.0);
    }

    #[test]
    fn scale_factor_applied() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(2.0); // 2x HiDPI
        let mut cx = DrawContext::new(&mut scene, scale);

        cx.paint_quad(
            Rect::new(Point::new(10.0, 20.0), Size::new(100.0, 50.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        );

        let quads = scene.quads();
        // Everything should be scaled by 2
        assert_eq!(quads[0].bounds.origin.x, 20.0);
        assert_eq!(quads[0].bounds.origin.y, 40.0);
        assert_eq!(quads[0].bounds.size.width, 200.0);
        assert_eq!(quads[0].bounds.size.height, 100.0);
    }

    #[test]
    fn clip_bounds_applied() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::new(&mut scene, scale);

        // Paint without clip - should have no clip bounds
        cx.paint_quad(
            Rect::new(Point::new(0.0, 0.0), Size::new(100.0, 100.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        );

        // Paint with clip
        cx.with_clip(
            Rect::new(Point::new(10.0, 10.0), Size::new(50.0, 50.0)),
            |cx| {
                cx.paint_quad(
                    Rect::new(Point::new(0.0, 0.0), Size::new(100.0, 100.0)),
                    Srgba::new(0.0, 1.0, 0.0, 1.0),
                );
            },
        );

        let quads = scene.quads();
        // First quad has no clip
        assert!(quads[0].clip_bounds.is_none());
        // Second quad should have clip bounds
        let clip = quads[1].clip_bounds.expect("should have clip bounds");
        assert_eq!(clip.origin.x, 10.0);
        assert_eq!(clip.origin.y, 10.0);
        assert_eq!(clip.size.width, 50.0);
        assert_eq!(clip.size.height, 50.0);
    }

    #[test]
    fn paint_text_creates_text_runs() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::new(&mut scene, scale);
        let mut text_ctx = TextContext::new();

        cx.paint_text(
            "Hello",
            Point::new(10.0, 50.0),
            16.0,
            Srgba::new(0.0, 0.0, 0.0, 1.0),
            &mut text_ctx,
        );

        assert!(scene.text_run_count() > 0, "should create text runs");
        let text_run = &scene.text_runs()[0];
        assert!(!text_run.glyphs.is_empty(), "should have glyphs");
        // X position should be exact
        assert_eq!(text_run.origin.x, 10.0);
        // Y position is adjusted for baseline - origin is above baseline
        // so the text baseline lands at the specified position
        assert!(
            text_run.origin.y < 50.0,
            "origin should be above baseline position"
        );
    }

    #[test]
    fn paint_text_respects_offset() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::new(&mut scene, scale);
        let mut text_ctx = TextContext::new();

        cx.with_offset(Point::new(100.0, 200.0), |cx| {
            cx.paint_text(
                "Hi",
                Point::new(10.0, 20.0),
                16.0,
                Srgba::new(0.0, 0.0, 0.0, 1.0),
                &mut text_ctx,
            );
        });

        let text_run = &scene.text_runs()[0];
        // X position should be offset: 100+10=110
        assert_eq!(text_run.origin.x, 110.0);
        // Y position is offset (200+20=220) minus baseline offset
        // so origin is above 220 but offset is correctly applied
        assert!(
            text_run.origin.y < 220.0,
            "origin should be above baseline position"
        );
        assert!(
            text_run.origin.y > 200.0,
            "origin should be below the offset y"
        );
    }

    #[test]
    fn paint_text_creates_access_node_when_enabled() {
        use crate::{AccessId, AccessRole, AccessTree};

        let mut scene = Scene::new();
        let mut access_tree = AccessTree::new(AccessId(0));
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::with_accessibility(&mut scene, &mut access_tree, scale);
        let mut text_ctx = TextContext::new();

        cx.paint_text(
            "Hello World",
            Point::new(50.0, 100.0),
            16.0,
            Srgba::new(0.0, 0.0, 0.0, 1.0),
            &mut text_ctx,
        );

        // Should have created an accessibility node
        assert!(access_tree.node_count() > 0, "should create access node");

        // Find the text node (it will have a generated ID starting from 1)
        let node = access_tree
            .get(AccessId(1))
            .expect("should have node with ID 1");
        assert_eq!(node.role, AccessRole::Label);
        assert_eq!(node.name, "Hello World");
        assert!(node.bounds.is_some(), "should have bounds");
    }

    #[test]
    fn paint_text_without_accessibility_works() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::new(&mut scene, scale);
        let mut text_ctx = TextContext::new();

        // Should work fine without accessibility
        cx.paint_text(
            "Hello",
            Point::new(10.0, 50.0),
            16.0,
            Srgba::new(0.0, 0.0, 0.0, 1.0),
            &mut text_ctx,
        );

        assert!(scene.text_run_count() > 0);
    }

    // ── Focus tests ────────────────────────────────────────────────────────────

    #[test]
    fn focus_handle_creates_unique_handle() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let cx = DrawContext::new(&mut scene, scale);

        let h1 = cx.focus_handle();
        let h2 = cx.focus_handle();
        assert_ne!(h1.id(), h2.id());
    }

    #[test]
    fn is_focused_returns_false_without_focus_state() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let cx = DrawContext::new(&mut scene, scale);

        let handle = cx.focus_handle();
        assert!(!cx.is_focused(&handle));
    }

    #[test]
    fn request_focus_does_nothing_without_focus_state() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::new(&mut scene, scale);

        let handle = cx.focus_handle();
        let prev = cx.request_focus(&handle);
        assert!(prev.is_none());
    }

    #[test]
    fn with_focus_constructor_attaches_state() {
        let mut scene = Scene::new();
        let mut focus = FocusState::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::with_focus(&mut scene, &mut focus, scale);

        let handle = cx.focus_handle();
        assert!(!cx.is_focused(&handle));

        cx.request_focus(&handle);
        assert!(cx.is_focused(&handle));
    }

    #[test]
    fn request_focus_emits_focus_event() {
        let mut scene = Scene::new();
        let mut focus = FocusState::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::with_focus(&mut scene, &mut focus, scale);

        let handle = cx.focus_handle();
        cx.request_focus(&handle);

        // Verify the event landed in the FocusState
        let events = focus.take_events();
        assert_eq!(events.len(), 1);
        use crate::FocusEvent;
        assert_eq!(events[0], FocusEvent::Focus { id: handle.id() });
    }

    #[test]
    fn request_focus_moves_focus_and_emits_blur() {
        let mut scene = Scene::new();
        let mut focus = FocusState::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::with_focus(&mut scene, &mut focus, scale);

        let h1 = cx.focus_handle();
        let h2 = cx.focus_handle();

        cx.request_focus(&h1);
        focus.take_events(); // clear first Focus event

        cx.request_focus(&h2);

        assert!(!cx.is_focused(&h1));
        assert!(cx.is_focused(&h2));

        use crate::FocusEvent;
        let events = focus.take_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], FocusEvent::Focus { id: h2.id() });
        assert_eq!(events[1], FocusEvent::Blur { id: h1.id() });
    }

    #[test]
    fn blur_clears_focus() {
        let mut scene = Scene::new();
        let mut focus = FocusState::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::with_focus(&mut scene, &mut focus, scale);

        let handle = cx.focus_handle();
        cx.request_focus(&handle);
        assert!(cx.is_focused(&handle));

        let prev = cx.blur();
        assert_eq!(prev, Some(handle.id()));
        assert!(!cx.is_focused(&handle));
    }

    #[test]
    fn with_focus_state_builder_combines_with_accessibility() {
        use crate::{AccessId, AccessTree};

        let mut scene = Scene::new();
        let mut access_tree = AccessTree::new(AccessId(0));
        let mut focus = FocusState::new();
        let scale = ScaleFactor(1.0);

        let mut cx = DrawContext::with_accessibility(&mut scene, &mut access_tree, scale)
            .with_focus_state(&mut focus);

        let handle = cx.focus_handle();
        cx.request_focus(&handle);
        assert!(cx.is_focused(&handle));
    }
}
