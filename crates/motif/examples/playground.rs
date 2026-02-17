//! Visual playground for debugging and testing motif features.
//!
//! A screen-filling canvas with sections for typography, rendering,
//! elements, and anything else that needs visual verification.
//!
//! Run with: cargo run --example playground

use motif_core::{
    div,
    element::{self, Element, PaintContext},
    metal::{MetalRenderer, MetalSurface},
    text, ArcStr, DrawContext, IntoElement, ParentElement, Point, Rect, Render,
    RenderOnce, Renderer, ScaleFactor, Scene, Size, Srgba, TextContext, ViewContext, WindowContext,
};
use motif_debug::{DebugServer, SceneSnapshot};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

// ============================================================================
// Sections
// ============================================================================

// --- Typography Section ---

fn paint_typography_section(cx: &mut DrawContext, text_ctx: &mut TextContext, x: f32, y: f32) {
    // Text metrics visualization
    let sample = "Hxpgq";
    let font_size = 120.0;
    let baseline_y = y + 160.0;

    let layout = text_ctx.layout_text(sample, font_size);
    let text_width = layout.width();

    if let Some(fm) = layout.font_metrics() {
        let thickness = 2.0;
        let w = text_width + 40.0;

        // Line height background
        let box_top = baseline_y - fm.ascent;
        let box_height = fm.ascent + fm.descent;
        cx.paint_quad(
            Rect::new(Point::new(x, box_top), Size::new(w, box_height)),
            Srgba::new(0.1, 0.1, 0.14, 1.0),
        );

        // Metric lines
        let lines: &[(f32, Srgba, &str)] = &[
            (
                baseline_y - fm.ascent,
                Srgba::new(0.3, 0.9, 0.3, 0.6),
                "ascent",
            ),
            (
                baseline_y - fm.cap_height,
                Srgba::new(1.0, 0.6, 0.2, 0.6),
                "cap",
            ),
            (
                baseline_y - fm.x_height,
                Srgba::new(0.3, 0.9, 1.0, 0.6),
                "x-height",
            ),
            (baseline_y, Srgba::new(1.0, 1.0, 1.0, 0.4), "baseline"),
            (
                baseline_y + fm.descent,
                Srgba::new(1.0, 0.3, 0.3, 0.6),
                "descent",
            ),
        ];

        for &(line_y, color, label) in lines {
            cx.paint_quad(
                Rect::new(Point::new(x, line_y), Size::new(w, thickness)),
                color,
            );
            // Label
            cx.paint_text(
                label,
                Point::new(x + w + 8.0, line_y + 4.0),
                9.0,
                color,
                text_ctx,
            );
        }
    }

    // Render the sample text
    cx.paint_text(
        sample,
        Point::new(x, baseline_y),
        font_size,
        Srgba::new(1.0, 1.0, 1.0, 1.0),
        text_ctx,
    );

    // Size samples
    let sizes = [48.0, 32.0, 24.0, 18.0, 14.0, 11.0];
    let mut sample_y = baseline_y + 80.0;
    for size in sizes {
        cx.paint_text(
            &format!("{}px — The quick brown fox jumps over the lazy dog", size as i32),
            Point::new(x, sample_y),
            size,
            Srgba::new(0.85, 0.85, 0.85, 1.0),
            text_ctx,
        );
        sample_y += size + 12.0;
    }
}

// --- Quad Rendering Section ---

fn paint_quad_section(cx: &mut DrawContext, x: f32, y: f32) {
    let size = 60.0;
    let gap = 16.0;

    // Row 1: Solid colors
    let colors = [
        Srgba::new(1.0, 0.3, 0.3, 1.0),
        Srgba::new(0.3, 1.0, 0.3, 1.0),
        Srgba::new(0.3, 0.3, 1.0, 1.0),
        Srgba::new(1.0, 1.0, 0.3, 1.0),
        Srgba::new(1.0, 0.3, 1.0, 1.0),
        Srgba::new(0.3, 1.0, 1.0, 1.0),
    ];

    for (i, color) in colors.iter().enumerate() {
        cx.paint_quad(
            Rect::new(
                Point::new(x + i as f32 * (size + gap), y),
                Size::new(size, size),
            ),
            *color,
        );
    }

    // Row 2: Borders and rounded corners
    let mut quad = motif_core::Quad::new(
        motif_core::DeviceRect::new(
            motif_core::DevicePoint::new(x * 2.0, (y + size + gap) * 2.0),
            motif_core::DeviceSize::new(size * 2.0, size * 2.0),
        ),
        Srgba::new(0.15, 0.15, 0.25, 1.0),
    );
    quad.border_color = Srgba::new(0.5, 0.7, 1.0, 1.0);
    quad.border_widths = motif_core::Edges::all(2.0);
    quad.corner_radii = motif_core::Corners::all(12.0);
    cx.paint(quad);

    let mut quad2 = motif_core::Quad::new(
        motif_core::DeviceRect::new(
            motif_core::DevicePoint::new((x + size + gap) * 2.0, (y + size + gap) * 2.0),
            motif_core::DeviceSize::new(size * 2.0, size * 2.0),
        ),
        Srgba::new(0.2, 0.15, 0.25, 1.0),
    );
    quad2.border_color = Srgba::new(1.0, 0.5, 0.7, 1.0);
    quad2.border_widths = motif_core::Edges::all(3.0);
    quad2.corner_radii = motif_core::Corners::all(30.0);
    cx.paint(quad2);

    let mut quad3 = motif_core::Quad::new(
        motif_core::DeviceRect::new(
            motif_core::DevicePoint::new((x + 2.0 * (size + gap)) * 2.0, (y + size + gap) * 2.0),
            motif_core::DeviceSize::new(size * 2.0, size * 2.0),
        ),
        Srgba::new(0.25, 0.2, 0.15, 1.0),
    );
    quad3.border_color = Srgba::new(1.0, 0.8, 0.3, 1.0);
    quad3.border_widths = motif_core::Edges::all(1.0);
    cx.paint(quad3);
}

// --- Element System Section ---

struct ElementDemo {
    frame: u32,
}

impl Render for ElementDemo {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        self.frame += 1;

        div()
            .bounds(Rect::new(Point::new(500.0, 30.0), Size::new(280.0, 80.0)))
            .background(Srgba::new(0.12, 0.14, 0.2, 1.0))
            .corner_radius(8.0)
            .border_color(Srgba::new(0.3, 0.5, 0.8, 1.0))
            .border_width(1.0)
            .child(
                text(format!("Render view — frame {}", self.frame))
                    .position(Point::new(516.0, 65.0))
                    .font_size(14.0)
                    .color(Srgba::new(0.8, 0.9, 1.0, 1.0)),
            )
            .child(
                text("Stateful: owns data, &mut self")
                    .position(Point::new(516.0, 88.0))
                    .font_size(11.0)
                    .color(Srgba::new(0.5, 0.6, 0.7, 1.0)),
            )
    }
}

struct StatusCard {
    label: ArcStr,
    value: ArcStr,
    position: Point,
    color: Srgba,
}

impl RenderOnce for StatusCard {
    fn render(self, _cx: &mut WindowContext) -> impl IntoElement {
        div()
            .bounds(Rect::new(self.position, Size::new(130.0, 56.0)))
            .background(Srgba::new(0.12, 0.14, 0.2, 1.0))
            .corner_radius(6.0)
            .child(
                text(self.value)
                    .position(Point::new(self.position.x + 12.0, self.position.y + 28.0))
                    .font_size(18.0)
                    .color(self.color),
            )
            .child(
                text(self.label)
                    .position(Point::new(self.position.x + 12.0, self.position.y + 46.0))
                    .font_size(9.0)
                    .color(Srgba::new(0.45, 0.45, 0.5, 1.0)),
            )
    }
}

// ============================================================================
// App
// ============================================================================

struct App {
    window: Option<Window>,
    renderer: Option<MetalRenderer>,
    surface: Option<MetalSurface>,
    scene: Scene,
    text_ctx: TextContext,
    element_demo: ElementDemo,
    debug_server: Option<DebugServer>,
}

impl Default for App {
    fn default() -> Self {
        let debug_server = DebugServer::new().ok();
        Self {
            window: None,
            renderer: None,
            surface: None,
            scene: Scene::new(),
            text_ctx: TextContext::new(),
            element_demo: ElementDemo { frame: 0 },
            debug_server,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("Motif — Playground")
                .with_inner_size(winit::dpi::LogicalSize::new(1200.0, 900.0));
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
                    let mut cx = DrawContext::new(&mut self.scene, scale);

                    // --- Section: Typography ---
                    paint_section_label(&mut cx, &mut self.text_ctx, "TYPOGRAPHY", 30.0, 20.0);
                    paint_typography_section(&mut cx, &mut self.text_ctx, 30.0, 30.0);

                    // --- Section: Quad Rendering ---
                    paint_section_label(&mut cx, &mut self.text_ctx, "QUADS", 30.0, 530.0);
                    paint_quad_section(&mut cx, 30.0, 545.0);

                    // --- Section: Element System ---
                    paint_section_label(&mut cx, &mut self.text_ctx, "ELEMENTS", 500.0, 20.0);

                    // Render stateful view
                    {
                        let mut wcx = WindowContext::new(
                            &mut self.scene,
                            &mut self.text_ctx,
                            scale,
                        );
                        element::render_view(&mut self.element_demo, &mut wcx);
                    }

                    // Render stateless cards
                    {
                        let cards = vec![
                            StatusCard {
                                label: "Quads".into(),
                                value: ArcStr::from(format!("{}", self.scene.quad_count())),
                                position: Point::new(500.0, 120.0),
                                color: Srgba::new(0.4, 0.9, 0.6, 1.0),
                            },
                            StatusCard {
                                label: "Text runs".into(),
                                value: ArcStr::from(format!("{}", self.scene.text_run_count())),
                                position: Point::new(646.0, 120.0),
                                color: Srgba::new(0.6, 0.7, 1.0, 1.0),
                            },
                        ];

                        for card in cards {
                            let mut wcx = WindowContext::new(
                                &mut self.scene,
                                &mut self.text_ctx,
                                scale,
                            );
                            let mut el = card.render(&mut wcx).into_element();
                            let mut pcx = PaintContext::new(
                                &mut self.scene,
                                &mut self.text_ctx,
                                scale,
                            );
                            el.paint(&mut pcx);
                        }
                    }

                    renderer.render(&self.scene, surface);

                    // Update the debug server with the current scene state.
                    if let Some(ref debug_server) = self.debug_server {
                        let phys = window.inner_size();
                        let viewport = (phys.width as f32, phys.height as f32);
                        let snapshot = SceneSnapshot::from_scene(
                            &self.scene,
                            viewport,
                            scale.0,
                        );
                        debug_server.update_scene(snapshot);
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn paint_section_label(
    cx: &mut DrawContext,
    text_ctx: &mut TextContext,
    label: &str,
    x: f32,
    y: f32,
) {
    cx.paint_text(label, Point::new(x, y), 10.0, Srgba::new(0.4, 0.4, 0.45, 1.0), text_ctx);
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
