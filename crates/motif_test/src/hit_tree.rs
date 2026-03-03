//! Hit testing data structures.
//!
//! Collects element bounds during paint and provides hit testing queries.

use motif_core::{Point, Rect};

/// Unique identifier for an element within a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementId(pub u64);

impl ElementId {
    /// Generate next ID from a counter.
    pub fn next(counter: &mut u64) -> Self {
        let id = *counter;
        *counter += 1;
        Self(id)
    }
}

/// Entry in the hit tree: element bounds with z-order.
#[derive(Debug, Clone)]
pub struct HitEntry {
    pub id: ElementId,
    pub bounds: Rect,
    pub z_index: u32,
}

/// Collects hit-testable regions during paint.
///
/// Elements register their bounds during paint. Hit testing walks
/// the list in reverse order (last painted = topmost = first hit).
#[derive(Debug, Default)]
pub struct HitTree {
    entries: Vec<HitEntry>,
    next_z: u32,
}

impl HitTree {
    /// Create an empty hit tree.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an element's bounds. Called during paint.
    pub fn push(&mut self, id: ElementId, bounds: Rect) {
        self.entries.push(HitEntry {
            id,
            bounds,
            z_index: self.next_z,
        });
        self.next_z += 1;
    }

    /// Hit test: returns topmost element containing point.
    pub fn hit_test(&self, point: Point) -> Option<ElementId> {
        self.entries
            .iter()
            .rev()
            .find(|e| rect_contains(&e.bounds, point))
            .map(|e| e.id)
    }

    /// Hit test: returns all elements containing point, topmost first.
    pub fn hit_test_all(&self, point: Point) -> Vec<ElementId> {
        self.entries
            .iter()
            .rev()
            .filter(|e| rect_contains(&e.bounds, point))
            .map(|e| e.id)
            .collect()
    }

    /// Number of registered elements.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear for next frame.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_z = 0;
    }

    /// Get all entries (for debugging/visualization).
    pub fn entries(&self) -> &[HitEntry] {
        &self.entries
    }
}

/// Check if a rect contains a point.
fn rect_contains(rect: &Rect, point: Point) -> bool {
    point.x >= rect.origin.x
        && point.x < rect.origin.x + rect.size.width
        && point.y >= rect.origin.y
        && point.y < rect.origin.y + rect.size.height
}

#[cfg(test)]
mod tests {
    use super::*;
    use motif_core::Size;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::new(Point::new(x, y), Size::new(w, h))
    }

    fn pt(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    // --- ElementId tests ---

    #[test]
    fn element_id_increments() {
        let mut counter = 0u64;
        let id1 = ElementId::next(&mut counter);
        let id2 = ElementId::next(&mut counter);
        let id3 = ElementId::next(&mut counter);

        assert_eq!(id1, ElementId(0));
        assert_eq!(id2, ElementId(1));
        assert_eq!(id3, ElementId(2));
        assert_eq!(counter, 3);
    }

    #[test]
    fn element_id_equality() {
        assert_eq!(ElementId(42), ElementId(42));
        assert_ne!(ElementId(1), ElementId(2));
    }

    // --- HitTree basic tests ---

    #[test]
    fn empty_hit_tree() {
        let tree = HitTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
        assert_eq!(tree.hit_test(pt(0.0, 0.0)), None);
    }

    #[test]
    fn push_registers_element() {
        let mut tree = HitTree::new();
        tree.push(ElementId(1), rect(0.0, 0.0, 100.0, 100.0));

        assert_eq!(tree.len(), 1);
        assert!(!tree.is_empty());
    }

    #[test]
    fn clear_removes_all() {
        let mut tree = HitTree::new();
        tree.push(ElementId(1), rect(0.0, 0.0, 100.0, 100.0));
        tree.push(ElementId(2), rect(50.0, 50.0, 100.0, 100.0));

        assert_eq!(tree.len(), 2);
        tree.clear();
        assert!(tree.is_empty());
    }

    // --- Single element hit tests ---

    #[test]
    fn hit_test_inside_single_element() {
        let mut tree = HitTree::new();
        tree.push(ElementId(1), rect(100.0, 100.0, 200.0, 50.0));

        // Inside
        assert_eq!(tree.hit_test(pt(150.0, 125.0)), Some(ElementId(1)));
        assert_eq!(tree.hit_test(pt(100.0, 100.0)), Some(ElementId(1))); // top-left corner
        assert_eq!(tree.hit_test(pt(299.0, 149.0)), Some(ElementId(1))); // near bottom-right
    }

    #[test]
    fn hit_test_outside_single_element() {
        let mut tree = HitTree::new();
        tree.push(ElementId(1), rect(100.0, 100.0, 200.0, 50.0));

        // Outside
        assert_eq!(tree.hit_test(pt(50.0, 50.0)), None); // above-left
        assert_eq!(tree.hit_test(pt(150.0, 50.0)), None); // above
        assert_eq!(tree.hit_test(pt(350.0, 125.0)), None); // right
        assert_eq!(tree.hit_test(pt(150.0, 200.0)), None); // below
    }

    #[test]
    fn hit_test_on_boundary() {
        let mut tree = HitTree::new();
        tree.push(ElementId(1), rect(100.0, 100.0, 100.0, 100.0));

        // On boundary: left/top edges are inclusive
        assert_eq!(tree.hit_test(pt(100.0, 100.0)), Some(ElementId(1)));

        // Right/bottom edges are exclusive
        assert_eq!(tree.hit_test(pt(200.0, 150.0)), None); // right edge
        assert_eq!(tree.hit_test(pt(150.0, 200.0)), None); // bottom edge
    }

    // --- Overlapping elements (z-order) ---

    #[test]
    fn hit_test_overlapping_returns_topmost() {
        let mut tree = HitTree::new();

        // Back element (painted first, z=0)
        tree.push(ElementId(1), rect(100.0, 100.0, 200.0, 200.0));

        // Front element (painted second, z=1, overlaps)
        tree.push(ElementId(2), rect(150.0, 150.0, 100.0, 100.0));

        // In overlap region: front (id=2) wins
        assert_eq!(tree.hit_test(pt(175.0, 175.0)), Some(ElementId(2)));

        // In back-only region: back (id=1) wins
        assert_eq!(tree.hit_test(pt(110.0, 110.0)), Some(ElementId(1)));

        // Outside both
        assert_eq!(tree.hit_test(pt(50.0, 50.0)), None);
    }

    #[test]
    fn hit_test_three_overlapping_layers() {
        let mut tree = HitTree::new();

        tree.push(ElementId(1), rect(0.0, 0.0, 300.0, 300.0)); // bottom
        tree.push(ElementId(2), rect(50.0, 50.0, 200.0, 200.0)); // middle
        tree.push(ElementId(3), rect(100.0, 100.0, 100.0, 100.0)); // top

        // Center: top layer
        assert_eq!(tree.hit_test(pt(150.0, 150.0)), Some(ElementId(3)));

        // Middle ring: middle layer
        assert_eq!(tree.hit_test(pt(75.0, 75.0)), Some(ElementId(2)));

        // Outer ring: bottom layer
        assert_eq!(tree.hit_test(pt(25.0, 25.0)), Some(ElementId(1)));
    }

    // --- hit_test_all ---

    #[test]
    fn hit_test_all_returns_all_in_z_order() {
        let mut tree = HitTree::new();

        tree.push(ElementId(1), rect(0.0, 0.0, 200.0, 200.0));
        tree.push(ElementId(2), rect(50.0, 50.0, 100.0, 100.0));

        // In overlap: both, topmost first
        let hits = tree.hit_test_all(pt(75.0, 75.0));
        assert_eq!(hits, vec![ElementId(2), ElementId(1)]);

        // In outer only: just bottom
        let hits = tree.hit_test_all(pt(25.0, 25.0));
        assert_eq!(hits, vec![ElementId(1)]);

        // Outside: empty
        let hits = tree.hit_test_all(pt(250.0, 250.0));
        assert!(hits.is_empty());
    }

    #[test]
    fn hit_test_all_nested_elements() {
        let mut tree = HitTree::new();

        // Parent
        tree.push(ElementId(100), rect(50.0, 50.0, 300.0, 200.0));

        // Child
        tree.push(ElementId(101), rect(100.0, 100.0, 100.0, 50.0));

        // In child: both, child first
        let hits = tree.hit_test_all(pt(125.0, 115.0));
        assert_eq!(hits, vec![ElementId(101), ElementId(100)]);
    }

    // --- Z-index tracking ---

    #[test]
    fn z_index_increments_on_push() {
        let mut tree = HitTree::new();

        tree.push(ElementId(1), rect(0.0, 0.0, 10.0, 10.0));
        tree.push(ElementId(2), rect(0.0, 0.0, 10.0, 10.0));
        tree.push(ElementId(3), rect(0.0, 0.0, 10.0, 10.0));

        let entries = tree.entries();
        assert_eq!(entries[0].z_index, 0);
        assert_eq!(entries[1].z_index, 1);
        assert_eq!(entries[2].z_index, 2);
    }

    #[test]
    fn z_index_resets_on_clear() {
        let mut tree = HitTree::new();

        tree.push(ElementId(1), rect(0.0, 0.0, 10.0, 10.0));
        tree.push(ElementId(2), rect(0.0, 0.0, 10.0, 10.0));
        tree.clear();

        tree.push(ElementId(3), rect(0.0, 0.0, 10.0, 10.0));

        let entries = tree.entries();
        assert_eq!(entries[0].z_index, 0); // reset
    }

    // --- Edge cases ---

    #[test]
    fn hit_test_zero_size_element() {
        let mut tree = HitTree::new();
        tree.push(ElementId(1), rect(100.0, 100.0, 0.0, 0.0));

        // Zero-size element can't be hit
        assert_eq!(tree.hit_test(pt(100.0, 100.0)), None);
    }

    #[test]
    fn hit_test_negative_coordinates() {
        let mut tree = HitTree::new();
        tree.push(ElementId(1), rect(-100.0, -100.0, 200.0, 200.0));

        // Inside (includes negative space)
        assert_eq!(tree.hit_test(pt(-50.0, -50.0)), Some(ElementId(1)));
        assert_eq!(tree.hit_test(pt(50.0, 50.0)), Some(ElementId(1)));

        // Outside
        assert_eq!(tree.hit_test(pt(-150.0, 0.0)), None);
    }

    #[test]
    fn hit_test_many_elements() {
        let mut tree = HitTree::new();

        // Add 1000 non-overlapping elements in a grid
        for i in 0..1000 {
            let x = (i % 100) as f32 * 10.0;
            let y = (i / 100) as f32 * 10.0;
            tree.push(ElementId(i as u64), rect(x, y, 10.0, 10.0));
        }

        assert_eq!(tree.len(), 1000);

        // Hit test specific element
        assert_eq!(tree.hit_test(pt(55.0, 35.0)), Some(ElementId(305)));

        // Miss
        assert_eq!(tree.hit_test(pt(1000.0, 1000.0)), None);
    }
}
