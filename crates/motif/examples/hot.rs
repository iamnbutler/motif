//! Hot reloading demo using cargo-hot.
//!
//! Run with: cargo hot --example hot --features hot
//!
//! Try changing the background color or text while running!

use motif::hot;
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

/// The render function - this is what gets hot-reloaded.
/// Change colors, text, layout here and see updates instantly!
fn render(scene: &mut Scene, text_ctx: &mut TextContext, scale: ScaleFactor, size: (f32, f32)) {
    let mut cx = DrawContext::new(scene, scale);

    // Background - try changing this color!
    cx.paint_quad(
        Rect::new(Point::new(0.0, 0.0), Size::new(size.0, size.1)),
        Srgba::new(0.3, 0.1, 0.6, 1.0), // <-- Try purple, red, blue!
    );

    // Title - try changing the text!
    cx.paint_text(
        "Hot Reload Works!", // <-- Change me!
        Point::new(40.0, 60.0),
        32.0,
        Srgba::new(0.5, 1.0, 1.0, 1.0),
        text_ctx,
    );

    // Subtitle
    cx.paint_text(
        "Edit this file and fdsaf save to see changes",
        Point::new(40.0, 100.0),
        16.0,
        Srgba::new(0.6, 0.6, 0.7, 1.0),
        text_ctx,
    );

    // Colored boxes - try changing colors or positions!
    let colors = [
        Srgba::new(0.9, 0.3, 0.3, 1.0), // Red
        Srgba::new(0.3, 0.9, 0.3, 1.0), // Green
        Srgba::new(0.3, 0.3, 0.9, 1.0), // Blue
    ];

    for (i, color) in colors.iter().enumerate() {
        cx.paint_quad(
            Rect::new(
                Point::new(40.0 + i as f32 * 100.0, 140.0),
                Size::new(80.0, 80.0),
            ),
            *color,
        );
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("Motif — Hot Reload")
                .with_inner_size(winit::dpi::LogicalSize::new(500.0, 300.0));
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
                    let size = (phys.width as f32 / scale.0, phys.height as f32 / scale.0);

                    hot::call(render, &mut self.scene, &mut self.text_ctx, scale, size);

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
    hot::connect();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll); // Poll for hot reload responsiveness
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
