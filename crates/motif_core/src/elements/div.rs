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

    /// Set flex wrapping behavior.
    pub fn flex_wrap(mut self, wrap: layout::FlexWrap) -> Self {
        self.style.flex_wrap = wrap;
        self
    }

    /// Allow flex items to wrap onto multiple lines.
    pub fn wrap(mut self) -> Self {
        self.style.flex_wrap = layout::FlexWrap::Wrap;
        self
    }

    /// Prevent flex items from wrapping (default behavior).
    pub fn no_wrap(mut self) -> Self {
        self.style.flex_wrap = layout::FlexWrap::NoWrap;
        self
    }

    /// Self-alignment within the parent flex container, overriding `align_items`.
    pub fn align_self(mut self, align: layout::AlignSelf) -> Self {
        self.style.align_self = Some(align);
        self
    }

    /// Alignment of wrapped flex lines across the cross axis (requires `wrap()`).
    pub fn align_content(mut self, align: layout::AlignContent) -> Self {
        self.style.align_content = Some(align);
        self
    }

    /// Flex basis — initial main-axis size before grow/shrink is applied.
    pub fn flex_basis(mut self, basis: f32) -> Self {
        self.style.flex_basis = taffy::style::Dimension::length(basis);
        self
    }

    /// Set positioning mode (default is `Relative`).
    ///
    /// `Absolute` removes the element from flow; its position is determined
    /// by `inset_top`, `inset_right`, `inset_bottom`, `inset_left` relative to
    /// the nearest non-static ancestor.
    pub fn position(mut self, position: layout::Position) -> Self {
        self.style.position = position;
        self
    }

    /// Shorthand for `position(Position::Absolute)`.
    ///
    /// Absolutely positioned elements are removed from the normal flow and
    /// their size/position is set via `inset_*` and explicit `width`/`height`.
    pub fn absolute(mut self) -> Self {
        self.style.position = layout::Position::Absolute;
        self
    }

    /// Set uniform inset on all sides (for absolutely positioned elements).
    pub fn inset(mut self, value: f32) -> Self {
        let v = taffy::style::LengthPercentageAuto::length(value);
        self.style.inset = taffy::Rect {
            left: v,
            right: v,
            top: v,
            bottom: v,
        };
        self
    }

    /// Set top inset (for absolutely positioned elements).
    pub fn inset_top(mut self, value: f32) -> Self {
        self.style.inset.top = taffy::style::LengthPercentageAuto::length(value);
        self
    }

    /// Set right inset (for absolutely positioned elements).
    pub fn inset_right(mut self, value: f32) -> Self {
        self.style.inset.right = taffy::style::LengthPercentageAuto::length(value);
        self
    }

    /// Set bottom inset (for absolutely positioned elements).
    pub fn inset_bottom(mut self, value: f32) -> Self {
        self.style.inset.bottom = taffy::style::LengthPercentageAuto::length(value);
        self
    }

    /// Set left inset (for absolutely positioned elements).
    pub fn inset_left(mut self, value: f32) -> Self {
        self.style.inset.left = taffy::style::LengthPercentageAuto::length(value);
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

    #[test]
    fn div_flex_wrap_sets_style() {
        let d = div().flex_wrap(layout::FlexWrap::Wrap);
        assert_eq!(d.style.flex_wrap, layout::FlexWrap::Wrap);
    }

    #[test]
    fn div_wrap_shorthand() {
        let d = div().wrap();
        assert_eq!(d.style.flex_wrap, layout::FlexWrap::Wrap);
    }

    #[test]
    fn div_no_wrap_shorthand() {
        let d = div().no_wrap();
        assert_eq!(d.style.flex_wrap, layout::FlexWrap::NoWrap);
    }

    #[test]
    fn div_align_self_sets_style() {
        let d = div().align_self(layout::AlignSelf::Center);
        assert_eq!(d.style.align_self, Some(layout::AlignSelf::Center));
    }

    #[test]
    fn div_align_content_sets_style() {
        let d = div().align_content(layout::AlignContent::SpaceBetween);
        assert_eq!(
            d.style.align_content,
            Some(layout::AlignContent::SpaceBetween)
        );
    }

    #[test]
    fn div_flex_basis_sets_style() {
        let d = div().flex_basis(50.0);
        assert_eq!(d.style.flex_basis, taffy::style::Dimension::length(50.0));
    }

    #[test]
    fn div_position_absolute_sets_style() {
        let d = div().absolute();
        assert_eq!(d.style.position, layout::Position::Absolute);
    }

    #[test]
    fn div_position_method_sets_style() {
        let d = div().position(layout::Position::Relative);
        assert_eq!(d.style.position, layout::Position::Relative);
    }

    #[test]
    fn div_inset_sets_all_sides() {
        let d = div().inset(10.0);
        let expected = taffy::style::LengthPercentageAuto::length(10.0);
        assert_eq!(d.style.inset.top, expected);
        assert_eq!(d.style.inset.right, expected);
        assert_eq!(d.style.inset.bottom, expected);
        assert_eq!(d.style.inset.left, expected);
    }

    #[test]
    fn div_inset_per_side() {
        let d = div()
            .inset_top(5.0)
            .inset_right(10.0)
            .inset_bottom(15.0)
            .inset_left(20.0);
        assert_eq!(
            d.style.inset.top,
            taffy::style::LengthPercentageAuto::length(5.0)
        );
        assert_eq!(
            d.style.inset.right,
            taffy::style::LengthPercentageAuto::length(10.0)
        );
        assert_eq!(
            d.style.inset.bottom,
            taffy::style::LengthPercentageAuto::length(15.0)
        );
        assert_eq!(
            d.style.inset.left,
            taffy::style::LengthPercentageAuto::length(20.0)
        );
    }

    #[test]
    fn absolutely_positioned_div_does_not_affect_parent_size() {
        let mut engine = LayoutEngine::new();
        let mut text_ctx = TextContext::new();

        // A fixed 50x50 child in normal flow
        let child_flow = engine.new_leaf(taffy::Style {
            size: taffy::Size {
                width: taffy::style::Dimension::length(50.0),
                height: taffy::style::Dimension::length(50.0),
            },
            ..Default::default()
        });

        // An absolutely positioned overlay (100x100) — should not expand parent
        let child_abs = engine.new_leaf(taffy::Style {
            position: layout::Position::Absolute,
            size: taffy::Size {
                width: taffy::style::Dimension::length(100.0),
                height: taffy::style::Dimension::length(100.0),
            },
            ..Default::default()
        });

        let parent = engine.new_with_children(
            taffy::Style {
                display: taffy::style::Display::Flex,
                ..Default::default()
            },
            &[child_flow, child_abs],
        );

        engine.compute_layout(parent, 800.0, 600.0, &mut text_ctx);

        let parent_bounds = engine.layout_bounds(parent);
        // Parent should size to the in-flow child (50x50), not the absolute one (100x100)
        assert_eq!(parent_bounds.size.width, 50.0);
        assert_eq!(parent_bounds.size.height, 50.0);
    }

    #[test]
    fn wrapping_flex_row_overflows_onto_next_line() {
        let mut engine = LayoutEngine::new();
        let mut text_ctx = TextContext::new();

        // Two 60px children in a 100px-wide wrapping row — second should wrap
        let child1 = engine.new_leaf(taffy::Style {
            size: taffy::Size {
                width: taffy::style::Dimension::length(60.0),
                height: taffy::style::Dimension::length(20.0),
            },
            ..Default::default()
        });
        let child2 = engine.new_leaf(taffy::Style {
            size: taffy::Size {
                width: taffy::style::Dimension::length(60.0),
                height: taffy::style::Dimension::length(20.0),
            },
            ..Default::default()
        });

        let parent = engine.new_with_children(
            taffy::Style {
                display: taffy::style::Display::Flex,
                flex_direction: layout::FlexDirection::Row,
                flex_wrap: layout::FlexWrap::Wrap,
                size: taffy::Size {
                    width: taffy::style::Dimension::length(100.0),
                    height: taffy::style::Dimension::auto(),
                },
                ..Default::default()
            },
            &[child1, child2],
        );

        engine.compute_layout(parent, 800.0, 600.0, &mut text_ctx);

        let b1 = engine.layout_bounds(child1);
        let b2 = engine.layout_bounds(child2);

        // First child on first row at y=0
        assert_eq!(b1.origin.y, 0.0);
        // Second child wrapped to second row at y=20
        assert_eq!(b2.origin.y, 20.0);
    }
}
