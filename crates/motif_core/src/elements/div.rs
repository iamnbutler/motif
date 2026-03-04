//! Container element with background, border, and children.

use crate::element::{
    AnyElement, Element, IntoElement, LayoutContext, PaintContext, ParentElement,
};
use crate::layout::{self, NodeId};
use crate::{Corners, DeviceRect, Edges, Quad, Rect, Size};
use palette::Srgba;
use smallvec::SmallVec;

/// A container element, analogous to an HTML div.
///
/// Supports background color, borders, rounded corners, and children.
/// Uses builder pattern for configuration.
///
/// ```ignore
/// div()
///     .size(Size::new(200.0, 100.0))
///     .background(Srgba::new(0.1, 0.1, 0.15, 1.0))
///     .corner_radius(8.0)
///     .child(text("Hello"))
/// ```
pub struct Div {
    // Layout style
    style: layout::Style,
    // Visual properties
    background: Option<Srgba>,
    border_color: Option<Srgba>,
    border_widths: Edges<f32>,
    corner_radii: Corners<f32>,
    // Children
    children: SmallVec<[AnyElement; 2]>,
}

impl Div {
    pub fn new() -> Self {
        Self {
            style: layout::Style::default(),
            background: None,
            border_color: None,
            border_widths: Edges::default(),
            corner_radii: Corners::default(),
            children: SmallVec::new(),
        }
    }

    /// Set fixed size in logical pixels.
    pub fn size(mut self, size: Size) -> Self {
        self.style.size = taffy::Size {
            width: taffy::style::Dimension::length(size.width),
            height: taffy::style::Dimension::length(size.height),
        };
        self
    }

    /// Set width in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.style.size.width = taffy::style::Dimension::length(width);
        self
    }

    /// Set height in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.style.size.height = taffy::style::Dimension::length(height);
        self
    }

    /// Set display mode (Flex, Block, None).
    pub fn flex(mut self) -> Self {
        self.style.display = taffy::style::Display::Flex;
        self
    }

    /// Set flex direction.
    pub fn flex_direction(mut self, direction: layout::FlexDirection) -> Self {
        self.style.flex_direction = direction;
        self
    }

    /// Shorthand for flex + column direction.
    pub fn flex_col(mut self) -> Self {
        self.style.display = taffy::style::Display::Flex;
        self.style.flex_direction = layout::FlexDirection::Column;
        self
    }

    /// Shorthand for flex + row direction.
    pub fn flex_row(mut self) -> Self {
        self.style.display = taffy::style::Display::Flex;
        self.style.flex_direction = layout::FlexDirection::Row;
        self
    }

    /// Set gap between children.
    pub fn gap(mut self, gap: f32) -> Self {
        self.style.gap = taffy::Size {
            width: taffy::style::LengthPercentage::length(gap),
            height: taffy::style::LengthPercentage::length(gap),
        };
        self
    }

    /// Set padding on all sides.
    pub fn padding(mut self, padding: f32) -> Self {
        self.style.padding = taffy::Rect {
            left: taffy::style::LengthPercentage::length(padding),
            right: taffy::style::LengthPercentage::length(padding),
            top: taffy::style::LengthPercentage::length(padding),
            bottom: taffy::style::LengthPercentage::length(padding),
        };
        self
    }

    /// Set justify content (main axis alignment).
    pub fn justify_content(mut self, justify: layout::JustifyContent) -> Self {
        self.style.justify_content = Some(justify);
        self
    }

    /// Set align items (cross axis alignment).
    pub fn align_items(mut self, align: layout::AlignItems) -> Self {
        self.style.align_items = Some(align);
        self
    }

    /// Set flex grow.
    pub fn flex_grow(mut self, grow: f32) -> Self {
        self.style.flex_grow = grow;
        self
    }

    /// Set flex shrink.
    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.style.flex_shrink = shrink;
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
        self.style.border = taffy::Rect {
            left: taffy::style::LengthPercentage::length(width),
            right: taffy::style::LengthPercentage::length(width),
            top: taffy::style::LengthPercentage::length(width),
            bottom: taffy::style::LengthPercentage::length(width),
        };
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

// Re-export taffy types for convenience
use taffy;

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
    fn request_layout(&mut self, cx: &mut LayoutContext) -> NodeId {
        // Request layout for all children first
        let child_ids: Vec<NodeId> = self
            .children
            .iter_mut()
            .map(|child| child.request_layout(cx))
            .collect();

        // Create our layout node with children
        cx.layout_engine()
            .new_with_children(self.style.clone(), &child_ids)
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        // Paint self as a quad if it has any visual properties
        if self.background.is_some() || self.border_color.is_some() {
            let scale = cx.scale_factor();
            let device_bounds = DeviceRect::new(
                scale.scale_point(bounds.origin),
                scale.scale_size(bounds.size),
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

        // Paint children (they get their bounds from layout engine)
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
    use crate::element::LayoutContext;
    use crate::{HitTree, LayoutEngine, ScaleFactor, Scene, TextContext};

    #[test]
    fn div_builder_sets_background() {
        let d = div().background(Srgba::new(1.0, 0.0, 0.0, 1.0));
        assert_eq!(d.background, Some(Srgba::new(1.0, 0.0, 0.0, 1.0)));
    }

    #[test]
    fn div_builder_sets_size() {
        let d = div().size(Size::new(100.0, 50.0));
        // Check that style has a defined length (not auto)
        assert_ne!(d.style.size.width, taffy::style::Dimension::auto());
        assert_eq!(d.style.size.width, taffy::style::Dimension::length(100.0));
    }

    #[test]
    fn div_paints_quad_when_has_background() {
        let mut d = div()
            .size(Size::new(100.0, 50.0))
            .background(Srgba::new(1.0, 0.0, 0.0, 1.0));

        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        // Request layout
        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let node_id = d.request_layout(&mut layout_cx);

        // Compute layout
        layout_engine.compute_layout(node_id, 800.0, 600.0, &mut text_ctx);

        // Paint
        let bounds = layout_engine.layout_bounds(node_id);
        let mut cx = PaintContext::new(
            &mut scene,
            &mut text_ctx,
            &mut hit_tree,
            &layout_engine,
            ScaleFactor(1.0),
        );
        d.paint(bounds, &mut cx);

        assert_eq!(scene.quad_count(), 1);
    }

    #[test]
    fn div_skips_paint_when_no_visual() {
        let mut d = div().size(Size::new(100.0, 50.0));

        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        // Request layout
        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let node_id = d.request_layout(&mut layout_cx);

        // Compute layout
        layout_engine.compute_layout(node_id, 800.0, 600.0, &mut text_ctx);

        // Paint
        let bounds = layout_engine.layout_bounds(node_id);
        let mut cx = PaintContext::new(
            &mut scene,
            &mut text_ctx,
            &mut hit_tree,
            &layout_engine,
            ScaleFactor(1.0),
        );
        d.paint(bounds, &mut cx);

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
