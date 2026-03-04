//! Core element traits for the motif UI framework.
//!
//! Two-tier component model:
//! - **Views** (`Render`): Stateful components that own data and persist across frames.
//! - **Elements** (`RenderOnce`): Stateless components consumed on render.
//!
//! Element lifecycle:
//! 1. `request_layout()` - build layout tree, return NodeId
//! 2. Layout engine computes bounds
//! 3. `paint()` - draw at computed bounds

use crate::{ElementId, HitTree, LayoutEngine, NodeId, Point, Rect, ScaleFactor, Scene, TextContext};

/// Views are stateful components that persist across frames.
///
/// Implement this for types that carry data and need to be re-rendered.
/// Views get `&mut self` so they can read and mutate their state.
///
/// ```ignore
/// struct Counter { count: i32 }
///
/// impl Render for Counter {
///     fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
///         div().child(text(format!("Count: {}", self.count)))
///     }
/// }
/// ```
pub trait Render: 'static + Sized {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement;
}

/// Elements are stateless components consumed on render.
///
/// Implement this for reusable UI patterns that don't own state.
/// Takes `self` by value (consumes) because elements are ephemeral.
///
/// ```ignore
/// struct Button { label: SharedString }
///
/// impl RenderOnce for Button {
///     fn render(self, cx: &mut WindowContext) -> impl IntoElement {
///         div().background(BLUE).child(text(self.label))
///     }
/// }
/// ```
pub trait RenderOnce: 'static + Sized {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement;
}

/// Anything that can become an element.
///
/// Implemented for built-in element types (Div, Text) and automatically
/// for types implementing `RenderOnce`.
pub trait IntoElement: Sized {
    type Element: Element;
    fn into_element(self) -> Self::Element;
}

/// Low-level element trait with layout and paint phases.
///
/// Most users should implement `Render` or `RenderOnce` instead.
/// This is for primitive element types like Div and Text.
pub trait Element: 'static {
    /// Request layout for this element.
    ///
    /// Called during the layout phase. Elements should:
    /// 1. Request layout for any children
    /// 2. Create a layout node with their style and children's NodeIds
    /// 3. Return their NodeId
    fn request_layout(&mut self, cx: &mut LayoutContext) -> NodeId;

    /// Paint this element at the given bounds.
    ///
    /// Called after layout has been computed. The bounds are the
    /// computed position and size from the layout engine.
    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext);
}

/// Trait for elements that can accept children.
pub trait ParentElement: Sized {
    fn children_mut(&mut self) -> &mut smallvec::SmallVec<[AnyElement; 2]>;

    fn child(mut self, child: impl IntoElement) -> Self {
        let element = child.into_element();
        self.children_mut().push(AnyElement::new(element));
        self
    }

    fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        for child in children {
            let element = child.into_element();
            self.children_mut().push(AnyElement::new(element));
        }
        self
    }
}

/// Type-erased element wrapper.
///
/// Allows heterogeneous collections of elements (e.g. Div children).
pub struct AnyElement {
    element: Box<dyn Element>,
    node_id: Option<NodeId>,
}

impl AnyElement {
    pub fn new(element: impl Element) -> Self {
        Self {
            element: Box::new(element),
            node_id: None,
        }
    }

    /// Request layout for this element.
    pub fn request_layout(&mut self, cx: &mut LayoutContext) -> NodeId {
        let node_id = self.element.request_layout(cx);
        self.node_id = Some(node_id);
        node_id
    }

    /// Paint this element. Must call request_layout first.
    pub fn paint(&mut self, cx: &mut PaintContext) {
        let node_id = self.node_id.expect("must call request_layout before paint");
        let bounds = cx.layout_bounds(node_id);
        self.element.paint(bounds, cx);
    }
}

/// Context for rendering views (stateful).
pub struct ViewContext<'a, V: 'static> {
    pub(crate) window: WindowContext<'a>,
    _marker: std::marker::PhantomData<V>,
}

impl<'a, V: 'static> ViewContext<'a, V> {
    pub fn new(window: WindowContext<'a>) -> Self {
        Self {
            window,
            _marker: std::marker::PhantomData,
        }
    }

    /// Access the underlying window context.
    pub fn window_cx(&mut self) -> &mut WindowContext<'a> {
        &mut self.window
    }
}

impl<V: 'static> std::ops::Deref for ViewContext<'_, V> {
    type Target = WindowContext<'static>;

    fn deref(&self) -> &Self::Target {
        // Safety: ViewContext's lifetime is bounded by 'a,
        // but we erase it here for ergonomics. The borrow checker
        // still prevents misuse through the &self lifetime.
        unsafe { std::mem::transmute(&self.window) }
    }
}

impl<V: 'static> std::ops::DerefMut for ViewContext<'_, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::mem::transmute(&mut self.window) }
    }
}

/// Context for the layout phase.
pub struct LayoutContext<'a> {
    pub(crate) layout_engine: &'a mut LayoutEngine,
    pub(crate) text_ctx: &'a mut TextContext,
    pub(crate) scale_factor: ScaleFactor,
}

impl<'a> LayoutContext<'a> {
    pub fn new(
        layout_engine: &'a mut LayoutEngine,
        text_ctx: &'a mut TextContext,
        scale_factor: ScaleFactor,
    ) -> Self {
        Self {
            layout_engine,
            text_ctx,
            scale_factor,
        }
    }

    pub fn layout_engine(&mut self) -> &mut LayoutEngine {
        self.layout_engine
    }

    pub fn text_ctx(&mut self) -> &mut TextContext {
        self.text_ctx
    }

    pub fn scale_factor(&self) -> ScaleFactor {
        self.scale_factor
    }
}

/// Context for rendering elements and painting.
pub struct WindowContext<'a> {
    pub(crate) scene: &'a mut Scene,
    pub(crate) text_ctx: &'a mut TextContext,
    pub(crate) scale_factor: ScaleFactor,
}

impl<'a> WindowContext<'a> {
    pub fn new(
        scene: &'a mut Scene,
        text_ctx: &'a mut TextContext,
        scale_factor: ScaleFactor,
    ) -> Self {
        Self {
            scene,
            text_ctx,
            scale_factor,
        }
    }

    pub fn scene(&mut self) -> &mut Scene {
        self.scene
    }

    pub fn text_ctx(&mut self) -> &mut TextContext {
        self.text_ctx
    }

    pub fn scale_factor(&self) -> ScaleFactor {
        self.scale_factor
    }
}

/// Context for the paint phase.
pub struct PaintContext<'a> {
    pub(crate) scene: &'a mut Scene,
    pub(crate) text_ctx: &'a mut TextContext,
    pub(crate) hit_tree: &'a mut HitTree,
    pub(crate) layout_engine: &'a LayoutEngine,
    pub(crate) scale_factor: ScaleFactor,
    /// Offset between layout position and actual paint position.
    /// Applied to all layout_bounds results.
    pub(crate) offset: Point,
}

impl<'a> PaintContext<'a> {
    pub fn new(
        scene: &'a mut Scene,
        text_ctx: &'a mut TextContext,
        hit_tree: &'a mut HitTree,
        layout_engine: &'a LayoutEngine,
        scale_factor: ScaleFactor,
    ) -> Self {
        Self {
            scene,
            text_ctx,
            hit_tree,
            layout_engine,
            scale_factor,
            offset: Point::new(0.0, 0.0),
        }
    }

    pub fn scene(&mut self) -> &mut Scene {
        self.scene
    }

    pub fn text_ctx(&mut self) -> &mut TextContext {
        self.text_ctx
    }

    pub fn hit_tree(&mut self) -> &mut HitTree {
        self.hit_tree
    }

    pub fn scale_factor(&self) -> ScaleFactor {
        self.scale_factor
    }

    /// Get computed layout bounds for a node, adjusted by current offset.
    pub fn layout_bounds(&self, node_id: NodeId) -> Rect {
        let mut bounds = self.layout_engine.layout_bounds(node_id);
        bounds.origin.x += self.offset.x;
        bounds.origin.y += self.offset.y;
        bounds
    }

    /// Set the paint offset. This is the difference between where layout says
    /// an element should be and where it's actually being painted.
    pub fn set_offset(&mut self, offset: Point) {
        self.offset = offset;
    }

    /// Get the current paint offset.
    pub fn offset(&self) -> Point {
        self.offset
    }

    /// Register an element for hit testing.
    pub fn register_hit(&mut self, id: ElementId, bounds: Rect) {
        self.hit_tree.push(id, bounds);
    }

    /// Paint a child element.
    pub fn paint_child(&mut self, child: &mut AnyElement) {
        child.paint(self);
    }
}

/// Render a view and paint its element tree to the scene.
///
/// This performs the full element lifecycle:
/// 1. Call view.render() to get the element tree
/// 2. Request layout for all elements
/// 3. Compute layout
/// 4. Paint all elements at their computed bounds
pub fn render_view<V: Render>(
    view: &mut V,
    cx: &mut WindowContext,
    layout_engine: &mut LayoutEngine,
    hit_tree: &mut HitTree,
    window_size: crate::Size,
) {
    // Clear layout for fresh computation
    layout_engine.clear();

    // Render phase: build element tree
    let mut view_cx = ViewContext::<V>::new(WindowContext {
        scene: cx.scene,
        text_ctx: cx.text_ctx,
        scale_factor: cx.scale_factor,
    });
    let element = view.render(&mut view_cx);
    let mut element = element.into_element();

    // Layout phase: request layout for all elements
    {
        let mut layout_cx = LayoutContext {
            layout_engine: &mut *layout_engine,
            text_ctx: cx.text_ctx,
            scale_factor: cx.scale_factor,
        };
        let root_node = element.request_layout(&mut layout_cx);

        // Compute layout
        layout_engine.compute_layout(
            root_node,
            window_size.width * cx.scale_factor.0,
            window_size.height * cx.scale_factor.0,
            cx.text_ctx,
        );

        // Paint phase: paint at computed bounds
        let root_bounds = layout_engine.layout_bounds(root_node);
        let mut paint_cx = PaintContext {
            scene: cx.scene,
            text_ctx: cx.text_ctx,
            hit_tree,
            layout_engine,
            scale_factor: cx.scale_factor,
            offset: Point::new(0.0, 0.0),
        };
        element.paint(root_bounds, &mut paint_cx);
    }
}

/// Empty element that renders nothing.
pub struct Empty;

impl Element for Empty {
    fn request_layout(&mut self, cx: &mut LayoutContext) -> NodeId {
        // Empty element has zero size
        cx.layout_engine.new_leaf(crate::layout::Style::default())
    }

    fn paint(&mut self, _bounds: Rect, _cx: &mut PaintContext) {}
}

impl IntoElement for Empty {
    type Element = Empty;
    fn into_element(self) -> Self::Element {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ElementId, HitTree, LayoutEngine, Point, Rect, Size};

    #[test]
    fn empty_element_paints_nothing() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        // Request layout
        let mut empty = Empty;
        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let node_id = empty.request_layout(&mut layout_cx);

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
        empty.paint(bounds, &mut cx);
        assert_eq!(scene.quad_count(), 0);
    }

    #[test]
    fn any_element_wraps_element() {
        let mut any = AnyElement::new(Empty);
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        // Request layout
        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let _node_id = any.request_layout(&mut layout_cx);

        // Compute layout
        layout_engine.compute_layout(_node_id, 800.0, 600.0, &mut text_ctx);

        // Paint
        let mut cx = PaintContext::new(
            &mut scene,
            &mut text_ctx,
            &mut hit_tree,
            &layout_engine,
            ScaleFactor(1.0),
        );
        any.paint(&mut cx);
        assert_eq!(scene.quad_count(), 0);
    }

    #[test]
    fn paint_context_registers_hit() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let layout_engine = LayoutEngine::new();

        {
            let mut cx = PaintContext::new(
                &mut scene,
                &mut text_ctx,
                &mut hit_tree,
                &layout_engine,
                ScaleFactor(1.0),
            );
            let id = ElementId(42);
            let bounds = Rect::new(Point::new(100.0, 100.0), Size::new(200.0, 50.0));
            cx.register_hit(id, bounds);
        }

        // Hit tree should have the element
        assert_eq!(hit_tree.len(), 1);
        assert_eq!(
            hit_tree.hit_test(Point::new(150.0, 125.0)),
            Some(ElementId(42))
        );
        assert_eq!(hit_tree.hit_test(Point::new(50.0, 50.0)), None);
    }

    #[test]
    fn paint_context_multiple_hits() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let layout_engine = LayoutEngine::new();

        {
            let mut cx = PaintContext::new(
                &mut scene,
                &mut text_ctx,
                &mut hit_tree,
                &layout_engine,
                ScaleFactor(1.0),
            );

            // Back element
            cx.register_hit(
                ElementId(1),
                Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 200.0)),
            );

            // Front element (overlapping)
            cx.register_hit(
                ElementId(2),
                Rect::new(Point::new(50.0, 50.0), Size::new(100.0, 100.0)),
            );
        }

        // In overlap: front wins
        assert_eq!(
            hit_tree.hit_test(Point::new(75.0, 75.0)),
            Some(ElementId(2))
        );

        // In back only: back wins
        assert_eq!(
            hit_tree.hit_test(Point::new(25.0, 25.0)),
            Some(ElementId(1))
        );
    }
}
