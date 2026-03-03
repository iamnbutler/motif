//! Container element with background, border, and children.

use crate::element::{AnyElement, Element, IntoElement, PaintContext, ParentElement};
use crate::{Corners, DeviceRect, Edges, Point, Quad, Rect, Size};
use palette::Srgba;
use smallvec::SmallVec;

/// A container element, analogous to an HTML div.
///
/// Supports background color, borders, rounded corners, and children.
/// Uses builder pattern for configuration.
///
/// ```ignore
/// div()
///     .bounds(Rect::new(Point::ZERO, Size::new(200.0, 100.0)))
///     .background(Srgba::new(0.1, 0.1, 0.15, 1.0))
///     .corner_radius(8.0)
///     .child(text("Hello"))
/// ```
pub struct Div {
    bounds: Rect,
    background: Option<Srgba>,
    border_color: Option<Srgba>,
    border_widths: Edges<f32>,
    corner_radii: Corners<f32>,
    children: SmallVec<[AnyElement; 2]>,
}

impl Div {
    pub fn new() -> Self {
        Self {
            bounds: Rect::new(Point::new(0.0, 0.0), Size::new(0.0, 0.0)),
            background: None,
            border_color: None,
            border_widths: Edges::default(),
            corner_radii: Corners::default(),
            children: SmallVec::new(),
        }
    }

    pub fn bounds(mut self, bounds: Rect) -> Self {
        self.bounds = bounds;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.bounds = Rect::new(self.bounds.origin, size);
        self
    }

    pub fn position(mut self, position: Point) -> Self {
        self.bounds = Rect::new(position, self.bounds.size);
        self
    }

    pub fn background(mut self, color: impl Into<Srgba>) -> Self {
        self.background = Some(color.into());
        self
    }

    pub fn border_color(mut self, color: impl Into<Srgba>) -> Self {
        self.border_color = Some(color.into());
        self
    }

    pub fn border_width(mut self, width: f32) -> Self {
        self.border_widths = Edges::all(width);
        self
    }

    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radii = Corners::all(radius);
        self
    }

    pub fn corner_radii(mut self, radii: Corners<f32>) -> Self {
        self.corner_radii = radii;
        self
    }
}

impl Default for Div {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Div {
    fn children_mut(&mut self) -> &mut SmallVec<[AnyElement; 2]> {
        &mut self.children
    }
}

impl Element for Div {
    fn paint(&mut self, cx: &mut PaintContext) {
        // Paint self as a quad if it has any visual properties
        if self.background.is_some() || self.border_color.is_some() {
            let scale = cx.scale_factor();
            let device_bounds = DeviceRect::new(
                scale.scale_point(self.bounds.origin),
                scale.scale_size(self.bounds.size),
            );

            let mut quad = Quad::new(
                device_bounds,
                self.background.unwrap_or(Srgba::new(0.0, 0.0, 0.0, 0.0)),
            );

            if let Some(border_color) = self.border_color {
                quad.border_color = border_color;
                quad.border_widths = self.border_widths;
            }

            quad.corner_radii = self.corner_radii;
            cx.scene().push_quad(quad);
        }

        // Paint children
        for child in &mut self.children {
            cx.paint_child(child);
        }
    }
}

impl IntoElement for Div {
    type Element = Div;
    fn into_element(self) -> Self::Element {
        self
    }
}

/// Create a new Div element.
pub fn div() -> Div {
    Div::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ScaleFactor, Scene, TextContext};

    #[test]
    fn div_builder_sets_background() {
        let d = div().background(Srgba::new(1.0, 0.0, 0.0, 1.0));
        assert_eq!(d.background, Some(Srgba::new(1.0, 0.0, 0.0, 1.0)));
    }

    #[test]
    fn div_builder_sets_bounds() {
        let d = div().bounds(Rect::new(Point::new(10.0, 20.0), Size::new(100.0, 50.0)));
        assert_eq!(d.bounds.origin.x, 10.0);
        assert_eq!(d.bounds.size.width, 100.0);
    }

    #[test]
    fn div_paints_quad_when_has_background() {
        let mut d = div()
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(100.0, 50.0)))
            .background(Srgba::new(1.0, 0.0, 0.0, 1.0));

        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut cx = PaintContext::new(&mut scene, &mut text_ctx, ScaleFactor(1.0));
        d.paint(&mut cx);

        assert_eq!(scene.quad_count(), 1);
    }

    #[test]
    fn div_skips_paint_when_no_visual() {
        let mut d = div().bounds(Rect::new(Point::new(0.0, 0.0), Size::new(100.0, 50.0)));

        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut cx = PaintContext::new(&mut scene, &mut text_ctx, ScaleFactor(1.0));
        d.paint(&mut cx);

        assert_eq!(scene.quad_count(), 0);
    }

    #[test]
    fn div_accepts_children() {
        let d = div()
            .child(crate::element::Empty)
            .child(crate::element::Empty);
        assert_eq!(d.children.len(), 2);
    }
}
