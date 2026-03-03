//! Core element traits for the motif UI framework.
//!
//! Two-tier component model:
//! - **Views** (`Render`): Stateful components that own data and persist across frames.
//! - **Elements** (`RenderOnce`): Stateless components consumed on render.

use crate::{Scene, ScaleFactor, TextContext};

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
    /// Paint this element to the scene.
    fn paint(&mut self, cx: &mut PaintContext);
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
pub struct AnyElement(Box<dyn Element>);

impl AnyElement {
    pub fn new(element: impl Element) -> Self {
        Self(Box::new(element))
    }

    pub fn paint(&mut self, cx: &mut PaintContext) {
        self.0.paint(cx);
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
    pub(crate) scale_factor: ScaleFactor,
}

impl<'a> PaintContext<'a> {
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

    /// Paint a child element.
    pub fn paint_child(&mut self, child: &mut AnyElement) {
        child.paint(self);
    }
}

/// Render a view and paint its element tree to the scene.
pub fn render_view<V: Render>(view: &mut V, cx: &mut WindowContext) {
    let mut view_cx = ViewContext::<V>::new(WindowContext {
        scene: cx.scene,
        text_ctx: cx.text_ctx,
        scale_factor: cx.scale_factor,
    });
    let element = view.render(&mut view_cx);
    let mut element = element.into_element();

    let mut paint_cx = PaintContext {
        scene: cx.scene,
        text_ctx: cx.text_ctx,
        scale_factor: cx.scale_factor,
    };
    element.paint(&mut paint_cx);
}

/// Empty element that renders nothing.
pub struct Empty;

impl Element for Empty {
    fn paint(&mut self, _cx: &mut PaintContext) {}
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

    #[test]
    fn empty_element_paints_nothing() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut cx = PaintContext::new(&mut scene, &mut text_ctx, ScaleFactor(1.0));
        let mut empty = Empty;
        empty.paint(&mut cx);
        assert_eq!(scene.quad_count(), 0);
    }

    #[test]
    fn any_element_wraps_element() {
        let mut any = AnyElement::new(Empty);
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut cx = PaintContext::new(&mut scene, &mut text_ctx, ScaleFactor(1.0));
        any.paint(&mut cx);
        assert_eq!(scene.quad_count(), 0);
    }
}
