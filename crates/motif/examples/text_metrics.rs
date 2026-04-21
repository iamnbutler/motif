//! Text metrics debug example.
//!
//! Renders text at multiple font sizes and overlays colored horizontal lines
//! at each typographic metric position — baseline, ascent, descent, cap height,
//! x height, and underline offset — to help developers understand and debug
//! text layout.
//!
//! Run with: cargo run --example text_metrics

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

const MARGIN: f32 = 48.0;
const BLOCK_GAP: f32 = 28.0;
/// Width of horizontal metric lines (leaves room for labels on the right).
const LINE_WIDTH: f32 = 640.0;
const LABEL_FONT_SIZE: f32 = 9.5;
const LABEL_OFFSET: f32 = 6.0;
const WINDOW_WIDTH: f32 = 920.0;
const WINDOW_HEIGHT: f32 = 640.0;
const FONT_SIZES: &[f32] = &[14.0, 20.0, 32.0, 56.0];
const SAMPLE_TEXT: &str = "The quick brown fox jumps.";

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

/// Draw a 1px horizontal rule at `y` with a metric label to its right.
fn draw_metric_line(
    cx: &mut DrawContext,
    text_ctx: &mut TextContext,
    x: f32,
    y: f32,
    color: Srgba,
    label: &str,
    value: f32,
) {
    cx.paint_quad(
        Rect::new(Point::new(x, y - 0.5), Size::new(LINE_WIDTH, 1.0)),
        color,
    );
    cx.paint_text(
        &format!("{}: {:.1}px", label, value),
        Point::new(x + LINE_WIDTH + LABEL_OFFSET, y),
        LABEL_FONT_SIZE,
        color,
        text_ctx,
    );
}

/// Render one text sample block at `origin` for `font_size`.
///
/// Returns the line height of the block so the caller can advance `y`.
fn draw_text_sample(
    cx: &mut DrawContext,
    text_ctx: &mut TextContext,
    text: &str,
    origin: Point,
    font_size: f32,
) -> f32 {
    // Layout at logical font size to extract metrics in logical pixels.
    let layout = text_ctx.layout_text(text, font_size);
    let line_metrics = layout.line_metrics();
    let font_metrics = layout.font_metrics();

    let Some(lm) = line_metrics.first() else {
        return font_size;
    };

    // `origin.y` is the top of the block; `lm.baseline` is the offset from
    // line top to baseline, so the text baseline lands at `baseline_y`.
    let baseline_y = origin.y + lm.baseline;

    // Small font-size label above the block.
    cx.paint_text(
        &format!("{}pt", font_size),
        Point::new(origin.x, origin.y - 2.0),
        11.0,
        Srgba::new(0.45, 0.45, 0.55, 1.0),
        text_ctx,
    );

    // Sample text rendered on top of the metric lines.
    cx.paint_text(
        text,
        Point::new(origin.x, baseline_y),
        font_size,
        Srgba::new(0.92, 0.92, 0.92, 1.0),
        text_ctx,
    );

    // --- Metric overlays ---

    // Baseline (blue)
    draw_metric_line(
        cx, text_ctx, origin.x, baseline_y,
        Srgba::new(0.35, 0.65, 1.0, 0.65),
        "baseline", 0.0,
    );
    // Ascent (green) — top of the alignment box above baseline
    draw_metric_line(
        cx, text_ctx, origin.x, baseline_y - lm.ascent,
        Srgba::new(0.30, 0.85, 0.40, 0.70),
        "ascent", lm.ascent,
    );
    // Descent (red) — bottom of the alignment box below baseline
    draw_metric_line(
        cx, text_ctx, origin.x, baseline_y + lm.descent,
        Srgba::new(0.90, 0.30, 0.30, 0.70),
        "descent", lm.descent,
    );

    if let Some(fm) = font_metrics {
        // Cap height (purple) — top of capital letters
        draw_metric_line(
            cx, text_ctx, origin.x, baseline_y - fm.cap_height,
            Srgba::new(0.70, 0.35, 0.90, 0.70),
            "cap_height", fm.cap_height,
        );
        // x height (orange) — top of lowercase letters
        draw_metric_line(
            cx, text_ctx, origin.x, baseline_y - fm.x_height,
            Srgba::new(0.95, 0.60, 0.15, 0.70),
            "x_height", fm.x_height,
        );
        // Underline (gray) — underline_offset is negative in swash (below baseline),
        // so subtracting it places the line below baseline.
        draw_metric_line(
            cx, text_ctx, origin.x, baseline_y - fm.underline_offset,
            Srgba::new(0.55, 0.55, 0.60, 0.60),
            "underline", fm.underline_offset,
        );
    }

    lm.line_height
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("Motif — Text Metrics")
                .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
                .with_resizable(false);
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
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(surface), Some(window)) =
                    (&mut self.renderer, &mut self.surface, &self.window)
                {
                    self.scene.clear();
                    let scale = ScaleFactor(window.scale_factor() as f32);
                    let phys = window.inner_size();
                    let (w, h) = (phys.width as f32 / scale.0, phys.height as f32 / scale.0);

                    let mut cx = DrawContext::new(&mut self.scene, scale);

                    // Dark background
                    cx.paint_quad(
                        Rect::new(Point::new(0.0, 0.0), Size::new(w, h)),
                        Srgba::new(0.07, 0.07, 0.09, 1.0),
                    );

                    // Header
                    cx.paint_text(
                        "Text Metrics",
                        Point::new(MARGIN, MARGIN),
                        16.0,
                        Srgba::new(0.50, 0.50, 0.60, 1.0),
                        &mut self.text_ctx,
                    );

                    // One block per font size
                    let mut y = MARGIN + 36.0;
                    for &font_size in FONT_SIZES {
                        let block_height = draw_text_sample(
                            &mut cx,
                            &mut self.text_ctx,
                            SAMPLE_TEXT,
                            Point::new(MARGIN, y),
                            font_size,
                        );
                        y += block_height + BLOCK_GAP;
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
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
