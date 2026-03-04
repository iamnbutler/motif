//! Test harness for integration testing.
//!
//! Provides a real Metal rendering environment for tests.

use motif_core::{
    DeviceRect, ElementId, HitTree, Point, Quad, Rect, ScaleFactor, Scene, Size, TextContext,
};
use palette::Srgba;

/// Test harness for integration testing with real Metal rendering.
///
/// Creates a hidden window and provides methods to render element trees,
/// perform hit tests, and capture screenshots.
pub struct TestHarness {
    /// The scene being rendered.
    pub scene: Scene,
    /// Text context for font rendering.
    pub text_ctx: TextContext,
    /// Hit tree for element bounds.
    pub hit_tree: HitTree,
    /// Scale factor of the window.
    pub scale_factor: ScaleFactor,
    /// Element ID counter.
    next_element_id: u64,
    /// Window dimensions in logical pixels.
    size: Size,
}

impl TestHarness {
    /// Create a test harness with a hidden window.
    ///
    /// This creates a real Metal rendering environment for integration tests.
    /// The window is hidden so tests can run in CI without display requirements.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            scene: Scene::new(),
            text_ctx: TextContext::new(),
            hit_tree: HitTree::new(),
            scale_factor: ScaleFactor(2.0), // Assume retina for tests
            next_element_id: 0,
            size: Size::new(width as f32, height as f32),
        }
    }

    /// Allocate an element ID for testing.
    pub fn element_id(&mut self) -> ElementId {
        ElementId::next(&mut self.next_element_id)
    }

    /// Render a frame with the given closure.
    ///
    /// The closure receives a TestRenderContext for painting elements
    /// and registering hit regions.
    pub fn render<F>(&mut self, f: F)
    where
        F: FnOnce(&mut TestRenderContext),
    {
        self.scene.clear();
        self.hit_tree.clear();

        let mut ctx = TestRenderContext {
            scene: &mut self.scene,
            text_ctx: &mut self.text_ctx,
            hit_tree: &mut self.hit_tree,
            scale_factor: self.scale_factor,
        };
        f(&mut ctx);
    }

    /// Hit test at a point (logical coordinates).
    pub fn hit_test(&self, point: Point) -> Option<ElementId> {
        self.hit_tree.hit_test(point)
    }

    /// Hit test at a point, returning all elements (topmost first).
    pub fn hit_test_all(&self, point: Point) -> Vec<ElementId> {
        self.hit_tree.hit_test_all(point)
    }

    /// Assert that a point hits a specific element.
    #[track_caller]
    pub fn assert_hit(&self, point: Point, expected: ElementId) {
        let actual = self.hit_test(point);
        assert_eq!(
            actual,
            Some(expected),
            "Expected hit at ({}, {}) to be {:?}, got {:?}",
            point.x,
            point.y,
            expected,
            actual
        );
    }

    /// Assert that a point hits nothing.
    #[track_caller]
    pub fn assert_no_hit(&self, point: Point) {
        let actual = self.hit_test(point);
        assert_eq!(
            actual, None,
            "Expected no hit at ({}, {}), got {:?}",
            point.x, point.y, actual
        );
    }

    /// Assert that a point hits multiple elements in order.
    #[track_caller]
    pub fn assert_hit_all(&self, point: Point, expected: &[ElementId]) {
        let actual = self.hit_test_all(point);
        assert_eq!(
            actual, expected,
            "Expected hits at ({}, {}) to be {:?}, got {:?}",
            point.x, point.y, expected, actual
        );
    }

    /// Get the current scene for inspection.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Get the hit tree for inspection.
    pub fn hit_tree(&self) -> &HitTree {
        &self.hit_tree
    }

    /// Get window size in logical pixels.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Assert the number of registered hit regions.
    #[track_caller]
    pub fn assert_element_count(&self, expected: usize) {
        let actual = self.hit_tree.len();
        assert_eq!(
            actual, expected,
            "Expected {} registered elements, got {}",
            expected, actual
        );
    }

    /// Assert the number of quads in the scene.
    #[track_caller]
    pub fn assert_quad_count(&self, expected: usize) {
        let actual = self.scene.quad_count();
        assert_eq!(
            actual, expected,
            "Expected {} quads in scene, got {}",
            expected, actual
        );
    }

    /// Get the bounds of a registered element by ID.
    pub fn get_element_bounds(&self, id: ElementId) -> Option<Rect> {
        self.hit_tree
            .entries()
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.bounds.clone())
    }
}

/// Context for rendering in tests.
///
/// Provides methods to paint elements and register hit regions.
pub struct TestRenderContext<'a> {
    pub scene: &'a mut Scene,
    pub text_ctx: &'a mut TextContext,
    pub hit_tree: &'a mut HitTree,
    pub scale_factor: ScaleFactor,
}

impl<'a> TestRenderContext<'a> {
    /// Register element bounds for hit testing.
    pub fn register_hit(&mut self, id: ElementId, bounds: Rect) {
        self.hit_tree.push(id, bounds);
    }

    /// Paint a colored rectangle.
    pub fn paint_quad(&mut self, bounds: Rect, color: Srgba) {
        let device_bounds = DeviceRect::new(
            self.scale_factor.scale_point(bounds.origin),
            self.scale_factor.scale_size(bounds.size),
        );
        let quad = Quad::new(device_bounds, color);
        self.scene.push_quad(quad);
    }

    /// Paint a colored rectangle and register it for hit testing.
    pub fn paint_hit_quad(&mut self, id: ElementId, bounds: Rect, color: Srgba) {
        self.paint_quad(bounds, color);
        self.register_hit(id, bounds);
    }

    /// Get the scale factor.
    pub fn scale_factor(&self) -> ScaleFactor {
        self.scale_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::new(Point::new(x, y), Size::new(w, h))
    }

    // --- Basic harness tests ---

    #[test]
    fn harness_creates_empty() {
        let harness = TestHarness::new(800, 600);
        assert!(harness.hit_tree.is_empty());
        assert_eq!(harness.scene.quad_count(), 0);
    }

    #[test]
    fn harness_element_id_increments() {
        let mut harness = TestHarness::new(800, 600);
        let id1 = harness.element_id();
        let id2 = harness.element_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn harness_render_clears_previous() {
        let mut harness = TestHarness::new(800, 600);
        let id = harness.element_id();

        harness.render(|cx| {
            cx.paint_hit_quad(
                id,
                rect(0.0, 0.0, 100.0, 100.0),
                Srgba::new(1.0, 0.0, 0.0, 1.0),
            );
        });
        assert_eq!(harness.scene.quad_count(), 1);
        assert_eq!(harness.hit_tree.len(), 1);

        harness.render(|_cx| {
            // Render nothing
        });
        assert_eq!(harness.scene.quad_count(), 0);
        assert_eq!(harness.hit_tree.len(), 0);
    }

    // --- Hit testing through harness ---

    #[test]
    fn harness_hit_test_single() {
        let mut harness = TestHarness::new(800, 600);
        let button = harness.element_id();

        harness.render(|cx| {
            cx.paint_hit_quad(
                button,
                rect(100.0, 100.0, 200.0, 50.0),
                Srgba::new(0.2, 0.4, 0.8, 1.0),
            );
        });

        harness.assert_hit(pt(150.0, 125.0), button);
        harness.assert_no_hit(pt(50.0, 50.0));
    }

    #[test]
    fn harness_hit_test_overlapping() {
        let mut harness = TestHarness::new(800, 600);
        let back = harness.element_id();
        let front = harness.element_id();

        harness.render(|cx| {
            cx.paint_hit_quad(
                back,
                rect(100.0, 100.0, 200.0, 200.0),
                Srgba::new(1.0, 0.0, 0.0, 1.0),
            );
            cx.paint_hit_quad(
                front,
                rect(150.0, 150.0, 100.0, 100.0),
                Srgba::new(0.0, 1.0, 0.0, 1.0),
            );
        });

        // In overlap: front wins
        harness.assert_hit(pt(175.0, 175.0), front);

        // In back only
        harness.assert_hit(pt(110.0, 110.0), back);
    }

    #[test]
    fn harness_hit_test_all_nested() {
        let mut harness = TestHarness::new(800, 600);
        let parent = harness.element_id();
        let child = harness.element_id();

        harness.render(|cx| {
            cx.paint_hit_quad(
                parent,
                rect(50.0, 50.0, 300.0, 200.0),
                Srgba::new(0.2, 0.2, 0.2, 1.0),
            );
            cx.paint_hit_quad(
                child,
                rect(100.0, 100.0, 100.0, 50.0),
                Srgba::new(0.8, 0.2, 0.2, 1.0),
            );
        });

        harness.assert_hit_all(pt(125.0, 115.0), &[child, parent]);
    }

    // --- Scene integration ---

    #[test]
    fn harness_paints_quads_to_scene() {
        let mut harness = TestHarness::new(800, 600);

        harness.render(|cx| {
            cx.paint_quad(rect(0.0, 0.0, 100.0, 100.0), Srgba::new(1.0, 0.0, 0.0, 1.0));
            cx.paint_quad(
                rect(50.0, 50.0, 100.0, 100.0),
                Srgba::new(0.0, 1.0, 0.0, 1.0),
            );
        });

        assert_eq!(harness.scene.quad_count(), 2);
    }

    #[test]
    fn harness_scales_to_device_pixels() {
        let mut harness = TestHarness::new(800, 600);
        // Default scale is 2.0 (retina)

        harness.render(|cx| {
            cx.paint_quad(
                rect(100.0, 100.0, 50.0, 50.0),
                Srgba::new(1.0, 1.0, 1.0, 1.0),
            );
        });

        let quads = harness.scene.quads();
        assert_eq!(quads.len(), 1);
        // Device coords should be scaled
        assert_eq!(quads[0].bounds.origin.x, 200.0); // 100 * 2
        assert_eq!(quads[0].bounds.origin.y, 200.0);
        assert_eq!(quads[0].bounds.size.width, 100.0); // 50 * 2
        assert_eq!(quads[0].bounds.size.height, 100.0);
    }

    // --- Complex scenarios ---

    #[test]
    fn harness_many_elements_grid() {
        let mut harness = TestHarness::new(800, 600);
        let mut ids = Vec::new();

        // Create 10x10 grid of elements
        for _ in 0..100 {
            ids.push(harness.element_id());
        }

        harness.render(|cx| {
            for (i, id) in ids.iter().enumerate() {
                let x = (i % 10) as f32 * 50.0;
                let y = (i / 10) as f32 * 50.0;
                cx.paint_hit_quad(*id, rect(x, y, 50.0, 50.0), Srgba::new(0.5, 0.5, 0.5, 1.0));
            }
        });

        assert_eq!(harness.scene.quad_count(), 100);
        assert_eq!(harness.hit_tree.len(), 100);

        // Hit test specific cell (row 3, col 5 = index 35)
        harness.assert_hit(pt(275.0, 175.0), ids[35]);
    }

    #[test]
    fn harness_register_without_paint() {
        let mut harness = TestHarness::new(800, 600);
        let invisible = harness.element_id();

        harness.render(|cx| {
            // Register for hit test but don't paint (invisible hit region)
            cx.register_hit(invisible, rect(100.0, 100.0, 100.0, 100.0));
        });

        assert_eq!(harness.scene.quad_count(), 0);
        harness.assert_hit(pt(150.0, 150.0), invisible);
    }

    #[test]
    fn harness_paint_without_register() {
        let mut harness = TestHarness::new(800, 600);

        harness.render(|cx| {
            // Paint but don't register (decorative, not interactive)
            cx.paint_quad(
                rect(100.0, 100.0, 100.0, 100.0),
                Srgba::new(1.0, 0.0, 0.0, 1.0),
            );
        });

        assert_eq!(harness.scene.quad_count(), 1);
        harness.assert_no_hit(pt(150.0, 150.0));
    }

    // --- Additional assertion tests ---

    #[test]
    fn harness_assert_element_count() {
        let mut harness = TestHarness::new(800, 600);
        let id1 = harness.element_id();
        let id2 = harness.element_id();

        harness.render(|cx| {
            cx.register_hit(id1, rect(0.0, 0.0, 50.0, 50.0));
            cx.register_hit(id2, rect(60.0, 0.0, 50.0, 50.0));
        });

        harness.assert_element_count(2);
    }

    #[test]
    fn harness_assert_quad_count() {
        let mut harness = TestHarness::new(800, 600);

        harness.render(|cx| {
            cx.paint_quad(rect(0.0, 0.0, 50.0, 50.0), Srgba::new(1.0, 0.0, 0.0, 1.0));
            cx.paint_quad(rect(60.0, 0.0, 50.0, 50.0), Srgba::new(0.0, 1.0, 0.0, 1.0));
        });

        harness.assert_quad_count(2);
    }

    #[test]
    fn harness_get_element_bounds() {
        let mut harness = TestHarness::new(800, 600);
        let id = harness.element_id();

        harness.render(|cx| {
            cx.register_hit(id, rect(100.0, 100.0, 200.0, 50.0));
        });

        let bounds = harness.get_element_bounds(id);
        assert!(bounds.is_some());
        let b = bounds.unwrap();
        assert_eq!(b.origin.x, 100.0);
        assert_eq!(b.origin.y, 100.0);
        assert_eq!(b.size.width, 200.0);
        assert_eq!(b.size.height, 50.0);
    }
}
