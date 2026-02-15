//! Example demonstrating text rendering with parley and Metal.

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
                .with_title("Motif - Hello Text")
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
                    if let Some(window) = &self.window {
                        let scale = window.scale_factor() as f32;
                        surface.resize(size.width as f32 * scale, size.height as f32 * scale);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(surface), Some(window)) =
                    (&mut self.renderer, &mut self.surface, &self.window)
                {
                    self.scene.clear();

                    let scale = ScaleFactor(window.scale_factor() as f32);
                    let mut cx = DrawContext::new(&mut self.scene, scale);

                    // Draw a background quad
                    cx.paint_quad(
                        Rect::new(Point::new(50.0, 50.0), Size::new(700.0, 100.0)),
                        Srgba::new(0.2, 0.2, 0.3, 1.0),
                    );

                    // Draw text
                    cx.paint_text(
                        "Hello, Motif!",
                        Point::new(70.0, 120.0), // baseline position
                        48.0,
                        Srgba::new(1.0, 1.0, 1.0, 1.0),
                        &mut self.text_ctx,
                    );

                    // Draw more text at different sizes
                    cx.paint_text(
                        "Text rendering with parley + swash + Metal",
                        Point::new(70.0, 180.0),
                        24.0,
                        Srgba::new(0.8, 0.8, 0.8, 1.0),
                        &mut self.text_ctx,
                    );

                    cx.paint_text(
                        "GPU-accelerated glyph atlas",
                        Point::new(70.0, 220.0),
                        18.0,
                        Srgba::new(0.6, 0.8, 1.0, 1.0),
                        &mut self.text_ctx,
                    );

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
