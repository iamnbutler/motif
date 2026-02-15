//! Debug example showing typographic metrics visualization.
//!
//! Draws colored quads behind text to highlight:
//! - Line height box (dark background)
//! - Ascent line (green)
//! - Cap height line (orange)
//! - X-height line (cyan)
//! - Baseline (white)
//! - Descent line (red)

use motif_core::{
    metal::{MetalRenderer, MetalSurface},
    DrawContext, Point, Rect, Renderer, ScaleFactor, Scene, Size, Srgba, TextContext,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

struct App {
    window: Option<Window>,
    renderer: Option<MetalRenderer>,
    surface: Option<MetalSurface>,
    scene: Scene,
    text_ctx: TextContext,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            surface: None,
            scene: Scene::new(),
            text_ctx: TextContext::new(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("Motif - Text Metrics Debug")
                .with_inner_size(winit::dpi::LogicalSize::new(900.0, 700.0));
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
                // Note: size is already in physical pixels
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

                    // Draw one large text sample with metrics visualization
                    // Using huge size to make the diagram clear
                    draw_text_with_metrics(&mut cx, &mut self.text_ctx, "Hxpgq", 200.0, 50.0, 350.0);

                    // Draw legend
                    draw_legend(&mut cx, 50.0, 580.0);

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

fn draw_text_with_metrics(
    cx: &mut DrawContext,
    text_ctx: &mut TextContext,
    text: &str,
    font_size: f32,
    x: f32,
    baseline_y: f32,
) {
    // Get layout and metrics
    let layout = text_ctx.layout_text(text, font_size);
    let text_width = layout.width();

    // Get font metrics for cap height, x-height, etc.
    let font_metrics = layout.font_metrics();

    if let Some(fm) = font_metrics {
        let line_thickness = 4.0;
        let width = text_width + 60.0;

        // Both ascent and descent are positive values from swash:
        // - ascent: distance above baseline
        // - descent: distance below baseline

        // Line height background (dark gray) - from ascent to descent
        let box_top = baseline_y - fm.ascent;
        let box_height = fm.ascent + fm.descent;
        cx.paint_quad(
            Rect::new(Point::new(x, box_top), Size::new(width, box_height)),
            Srgba::new(0.15, 0.15, 0.2, 1.0),
        );

        // Ascent line (green) - top of ascenders like 'h', 'd', 'l'
        let ascent_y = baseline_y - fm.ascent;
        cx.paint_quad(
            Rect::new(Point::new(x, ascent_y), Size::new(width, line_thickness)),
            Srgba::new(0.3, 0.9, 0.3, 1.0),
        );

        // Cap height line (orange) - top of capital letters
        let cap_y = baseline_y - fm.cap_height;
        cx.paint_quad(
            Rect::new(Point::new(x, cap_y), Size::new(width, line_thickness)),
            Srgba::new(1.0, 0.6, 0.2, 1.0),
        );

        // X-height line (cyan) - top of lowercase 'x', 'a', 'e', etc.
        let x_height_y = baseline_y - fm.x_height;
        cx.paint_quad(
            Rect::new(Point::new(x, x_height_y), Size::new(width, line_thickness)),
            Srgba::new(0.3, 0.9, 1.0, 1.0),
        );

        // Baseline (white) - where letters sit
        cx.paint_quad(
            Rect::new(
                Point::new(x, baseline_y - line_thickness / 2.0),
                Size::new(width, line_thickness),
            ),
            Srgba::new(1.0, 1.0, 1.0, 1.0),
        );

        // Descent line (red) - bottom of descenders like 'p', 'g', 'y'
        // descent is positive, so add it to baseline
        let descent_y = baseline_y + fm.descent;
        cx.paint_quad(
            Rect::new(Point::new(x, descent_y), Size::new(width, line_thickness)),
            Srgba::new(1.0, 0.3, 0.3, 1.0),
        );
    }

    // Draw the actual text
    cx.paint_text(
        text,
        Point::new(x, baseline_y),
        font_size,
        Srgba::new(1.0, 1.0, 1.0, 1.0),
        text_ctx,
    );
}

fn draw_legend(cx: &mut DrawContext, x: f32, y: f32) {
    let colors = [
        (Srgba::new(0.2, 0.8, 0.2, 1.0), "Ascent"),
        (Srgba::new(1.0, 0.6, 0.2, 1.0), "Cap height"),
        (Srgba::new(0.2, 0.8, 0.9, 1.0), "X-height"),
        (Srgba::new(1.0, 1.0, 1.0, 1.0), "Baseline"),
        (Srgba::new(0.9, 0.2, 0.2, 1.0), "Descent"),
    ];

    for (i, (color, _label)) in colors.iter().enumerate() {
        let item_y = y + i as f32 * 25.0;

        // Color swatch
        cx.paint_quad(
            Rect::new(Point::new(x, item_y), Size::new(20.0, 16.0)),
            *color,
        );
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
