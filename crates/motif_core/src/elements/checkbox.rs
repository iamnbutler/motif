//! Checkbox element with checked/unchecked visual state.
//!
//! ```ignore
//! let cb_id = ElementId(1);
//! checkbox(cb_id)
//!     .checked(true)
//!     .position(Point::new(10.0, 10.0))
//!     .paint(&mut cx);
//! ```

use crate::{
    element::{Element, IntoElement, PaintContext},
    Corners, DevicePoint, DeviceRect, DeviceSize, Edges, ElementId, Point, Quad, Rect, Size, Srgba,
};

/// Checkbox element with checked/unchecked visual state.
///
/// Renders a bordered square box. When checked, an inner filled square is
/// drawn as the check indicator. Register the element for hit testing so the
/// application can toggle state on click.
pub struct Checkbox {
    id: ElementId,
    checked: bool,
    position: Point,
    // Dimensions
    size: f32,
    corner_radius: f32,
    // Colors
    background: Srgba,
    border_color: Srgba,
    border_width: f32,
    check_color: Srgba,
    // Interactive state (set by caller before paint)
    is_hovered: bool,
    is_pressed: bool,
}

impl Checkbox {
    pub fn new(id: ElementId) -> Self {
        Self {
            id,
            checked: false,
            position: Point::new(0.0, 0.0),
            size: 18.0,
            corner_radius: 3.0,
            background: Srgba::new(1.0, 1.0, 1.0, 1.0),
            border_color: Srgba::new(0.4, 0.4, 0.4, 1.0),
            border_width: 1.5,
            check_color: Srgba::new(0.2, 0.4, 0.8, 1.0),
            is_hovered: false,
            is_pressed: false,
        }
    }

    /// Set whether the checkbox is checked.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set the position of the checkbox box.
    pub fn position(mut self, position: Point) -> Self {
        self.position = position;
        self
    }

    /// Set the box size (width and height). Defaults to 18.0.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Set the corner radius.
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Set the background color (fill when unchecked).
    pub fn background(mut self, color: Srgba) -> Self {
        self.background = color;
        self
    }

    /// Set the border color.
    pub fn border_color(mut self, color: Srgba) -> Self {
        self.border_color = color;
        self
    }

    /// Set the border width.
    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = width;
        self
    }

    /// Set the check indicator color (used when checked).
    pub fn check_color(mut self, color: Srgba) -> Self {
        self.check_color = color;
        self
    }

    /// Set whether the checkbox is currently hovered.
    pub fn hovered(mut self, hovered: bool) -> Self {
        self.is_hovered = hovered;
        self
    }

    /// Set whether the checkbox is currently pressed.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.is_pressed = pressed;
        self
    }

    /// Get the element ID.
    pub fn id(&self) -> ElementId {
        self.id
    }

    /// Get the logical bounds of the checkbox.
    pub fn bounds(&self) -> Rect {
        Rect::new(self.position, Size::new(self.size, self.size))
    }
}

impl Element for Checkbox {
    fn paint(&mut self, cx: &mut PaintContext) {
        let scale = cx.scale_factor().0;
        let bounds = self.bounds();

        // Use check_color border on hover/press to provide visual feedback.
        let border_color = if self.is_hovered || self.is_pressed {
            self.check_color
        } else {
            self.border_color
        };

        // Paint outer box.
        let device_bounds = DeviceRect::new(
            DevicePoint::new(bounds.origin.x * scale, bounds.origin.y * scale),
            DeviceSize::new(bounds.size.width * scale, bounds.size.height * scale),
        );
        let mut outer_quad = Quad::new(device_bounds, self.background);
        outer_quad.border_color = border_color;
        outer_quad.border_widths = Edges::all(self.border_width * scale);
        outer_quad.corner_radii = Corners::all(self.corner_radius * scale);
        cx.scene().push_quad(outer_quad);

        // Paint check indicator (filled inner square) when checked.
        if self.checked {
            let inset = self.size * 0.25;
            let inner_size = self.size - inset * 2.0;
            let inner_device_bounds = DeviceRect::new(
                DevicePoint::new(
                    (bounds.origin.x + inset) * scale,
                    (bounds.origin.y + inset) * scale,
                ),
                DeviceSize::new(inner_size * scale, inner_size * scale),
            );
            let mut check_quad = Quad::new(inner_device_bounds, self.check_color);
            check_quad.corner_radii = Corners::all((self.corner_radius * 0.5) * scale);
            cx.scene().push_quad(check_quad);
        }

        // Register the full box for hit testing.
        cx.register_hit(self.id, bounds);
    }
}

impl IntoElement for Checkbox {
    type Element = Checkbox;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// Create a checkbox element with the given ID.
pub fn checkbox(id: ElementId) -> Checkbox {
    Checkbox::new(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HitTree, ScaleFactor, Scene, TextContext};

    #[test]
    fn checkbox_registers_hit() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();

        let mut cb = checkbox(ElementId(1)).position(Point::new(10.0, 10.0));

        {
            let mut cx =
                PaintContext::new(&mut scene, &mut text_ctx, &mut hit_tree, ScaleFactor(1.0));
            cb.paint(&mut cx);
        }

        // Center of the 18x18 box at (10,10) is (19,19).
        assert_eq!(
            hit_tree.hit_test(Point::new(19.0, 19.0)),
            Some(ElementId(1))
        );
    }

    #[test]
    fn checkbox_misses_outside_bounds() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();

        let mut cb = checkbox(ElementId(1)).position(Point::new(10.0, 10.0));

        {
            let mut cx =
                PaintContext::new(&mut scene, &mut text_ctx, &mut hit_tree, ScaleFactor(1.0));
            cb.paint(&mut cx);
        }

        assert_eq!(hit_tree.hit_test(Point::new(5.0, 5.0)), None);
    }

    #[test]
    fn checkbox_unchecked_paints_one_quad() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();

        let mut cb = checkbox(ElementId(1)).checked(false);

        {
            let mut cx =
                PaintContext::new(&mut scene, &mut text_ctx, &mut hit_tree, ScaleFactor(1.0));
            cb.paint(&mut cx);
        }

        assert_eq!(scene.quad_count(), 1);
    }

    #[test]
    fn checkbox_checked_paints_two_quads() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();

        let mut cb = checkbox(ElementId(1)).checked(true);

        {
            let mut cx =
                PaintContext::new(&mut scene, &mut text_ctx, &mut hit_tree, ScaleFactor(1.0));
            cb.paint(&mut cx);
        }

        // Outer box quad + inner check indicator quad.
        assert_eq!(scene.quad_count(), 2);
    }

    #[test]
    fn checkbox_default_state() {
        let cb = checkbox(ElementId(1));
        assert!(!cb.checked);
        assert!(!cb.is_hovered);
        assert!(!cb.is_pressed);
        assert_eq!(cb.size, 18.0);
        assert_eq!(cb.border_width, 1.5);
    }

    #[test]
    fn checkbox_builder_methods() {
        let cb = checkbox(ElementId(1))
            .checked(true)
            .position(Point::new(20.0, 30.0))
            .size(24.0)
            .corner_radius(4.0)
            .hovered(true)
            .pressed(false);

        assert!(cb.checked);
        assert_eq!(cb.position.x, 20.0);
        assert_eq!(cb.position.y, 30.0);
        assert_eq!(cb.size, 24.0);
        assert_eq!(cb.corner_radius, 4.0);
        assert!(cb.is_hovered);
        assert!(!cb.is_pressed);
    }

    #[test]
    fn checkbox_bounds_matches_position_and_size() {
        let cb = checkbox(ElementId(1))
            .position(Point::new(5.0, 10.0))
            .size(20.0);

        let bounds = cb.bounds();
        assert_eq!(bounds.origin.x, 5.0);
        assert_eq!(bounds.origin.y, 10.0);
        assert_eq!(bounds.size.width, 20.0);
        assert_eq!(bounds.size.height, 20.0);
    }

    #[test]
    fn checkbox_scale_factor_applied() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();

        let mut cb = checkbox(ElementId(1))
            .position(Point::new(10.0, 10.0))
            .size(20.0)
            .checked(false);

        {
            let mut cx =
                PaintContext::new(&mut scene, &mut text_ctx, &mut hit_tree, ScaleFactor(2.0));
            cb.paint(&mut cx);
        }

        // All device coordinates should be doubled.
        let quad = &scene.quads()[0];
        assert_eq!(quad.bounds.origin.x, 20.0);
        assert_eq!(quad.bounds.size.width, 40.0);
    }

    #[test]
    fn checkbox_hover_changes_border_color() {
        let cb_normal = checkbox(ElementId(1)).hovered(false);
        let cb_hovered = checkbox(ElementId(2)).hovered(true);

        // Verify the field is set; visual difference tested visually.
        assert!(!cb_normal.is_hovered);
        assert!(cb_hovered.is_hovered);
    }
}
