//! Example demonstrating the element/view system.
//!
//! Shows both a stateful Render view and stateless RenderOnce elements.

use motif_core::{
    div, element, metal::{MetalRenderer, MetalSurface},
    text, IntoElement, PaintContext, ParentElement, Point, Rect, Render, RenderOnce,
    Renderer, ScaleFactor, Scene, SharedString, Size, Srgba, TextContext,
    ViewContext, WindowContext, Element,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

// --- Stateful View: Counter ---

struct Counter {
    count: i32,
    label: SharedString,
}

impl Counter {
    fn new(label: impl Into<SharedString>) -> Self {
        Self {
            count: 0,
            label: label.into(),
        }
    }
}

impl Render for Counter {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        self.count += 1; // Increment each frame to show it's stateful

        div()
            .bounds(Rect::new(Point::new(50.0, 50.0), Size::new(700.0, 120.0)))
            .background(Srgba::new(0.12, 0.12, 0.18, 1.0))
            .corner_radius(12.0)
            .child(
                text(format!("{}: frame {}", self.label, self.count))
                    .position(Point::new(70.0, 120.0))
                    .font_size(32.0)
                    .color(Srgba::new(1.0, 1.0, 1.0, 1.0)),
            )
    }
}

// --- Stateless Element: InfoCard ---

struct InfoCard {
    title: SharedString,
    body: SharedString,
    position: Point,
    accent: Srgba,
}

impl RenderOnce for InfoCard {
    fn render(self, _cx: &mut WindowContext) -> impl IntoElement {
        div()
            .bounds(Rect::new(self.position, Size::new(330.0, 120.0)))
            .background(Srgba::new(0.15, 0.15, 0.22, 1.0))
            .corner_radius(8.0)
            .border_color(self.accent)
            .border_width(2.0)
            .child(
                text(self.title)
                    .position(Point::new(self.position.x + 20.0, self.position.y + 45.0))
                    .font_size(22.0)
                    .color(self.accent),
            )
            .child(
                text(self.body)
                    .position(Point::new(self.position.x + 20.0, self.position.y + 80.0))
                    .font_size(14.0)
                    .color(Srgba::new(0.7, 0.7, 0.7, 1.0)),
            )
    }
}

fn info_card(
    title: impl Into<SharedString>,
    body: impl Into<SharedString>,
    position: Point,
    accent: Srgba,
) -> InfoCard {
    InfoCard {
        title: title.into(),
        body: body.into(),
        position,
        accent,
    }
}

// --- App ---

struct App {
    window: Option<Window>,
    renderer: Option<MetalRenderer>,
    surface: Option<MetalSurface>,
    scene: Scene,
    text_ctx: TextContext,
    counter: Counter,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            surface: None,
            scene: Scene::new(),
            text_ctx: TextContext::new(),
            counter: Counter::new("Render count"),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("Motif - Element System")
                .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
            let window = event_loop.create_window(attrs).unwrap();

            let renderer = MetalRenderer::new();
            let surface = unsafe { MetalSurface::new(&window, renderer.device()) };

            window.request_redraw();
            self.window = Some(window);
            self.renderer = Some(renderer);
            self.surface = Some(surface);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface {
                    surface.resize(size.width as f32, size.height as f32);
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(surface), Some(window)) =
                    (&mut self.renderer, &mut self.surface, &self.window)
                {
                    self.scene.clear();

                    let scale = ScaleFactor(window.scale_factor() as f32);

                    // Render the stateful view
                    {
                        let mut cx = WindowContext::new(
                            &mut self.scene,
                            &mut self.text_ctx,
                            scale,
                        );
                        element::render_view(&mut self.counter, &mut cx);
                    }

                    // Render stateless RenderOnce elements
                    {
                        let card1 = info_card(
                            "Render trait",
                            "Views carry state, get &mut self",
                            Point::new(50.0, 200.0),
                            Srgba::new(0.4, 0.8, 1.0, 1.0),
                        );

                        let card2 = info_card(
                            "RenderOnce trait",
                            "Elements are stateless, consumed on render",
                            Point::new(420.0, 200.0),
                            Srgba::new(1.0, 0.6, 0.3, 1.0),
                        );

                        // Render card1
                        let mut cx = WindowContext::new(
                            &mut self.scene,
                            &mut self.text_ctx,
                            scale,
                        );
                        let mut el = card1.render(&mut cx).into_element();
                        let mut paint_cx = PaintContext::new(
                            &mut self.scene,
                            &mut self.text_ctx,
                            scale,
                        );
                        el.paint(&mut paint_cx);

                        // Render card2
                        let mut cx = WindowContext::new(
                            &mut self.scene,
                            &mut self.text_ctx,
                            scale,
                        );
                        let mut el = card2.render(&mut cx).into_element();
                        let mut paint_cx = PaintContext::new(
                            &mut self.scene,
                            &mut self.text_ctx,
                            scale,
                        );
                        el.paint(&mut paint_cx);
                    }

                    // Footer text
                    {
                        let mut footer = text("Built with motif element system")
                            .position(Point::new(50.0, 380.0))
                            .font_size(14.0)
                            .color(Srgba::new(0.5, 0.5, 0.5, 1.0))
                            .into_element();

                        let mut paint_cx = PaintContext::new(
                            &mut self.scene,
                            &mut self.text_ctx,
                            scale,
                        );
                        footer.paint(&mut paint_cx);
                    }

                    renderer.render(&self.scene, surface);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    eprintln!("=== Motif Element System Demo ===");
    eprintln!("Demonstrates Render (stateful views) and RenderOnce (stateless elements)");

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
