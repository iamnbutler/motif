//! Debug example for Metal instancing issue.
//!
//! Draws a grid of quads to test which instances render.

use motif_core::{
    metal::{MetalRenderer, MetalSurface},
    DrawContext, Point, Rect, Renderer, ScaleFactor, Scene, Size, Srgba,
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
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            surface: None,
            scene: Scene::new(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("Motif - Instancing Debug")
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
                // Note: size is already in physical pixels on macOS
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

                    // Draw a 5x5 grid of quads
                    let quad_size = 60.0;
                    let spacing = 80.0;
                    let start_x = 50.0;
                    let start_y = 50.0;

                    let colors = [
                        Srgba::new(1.0, 0.0, 0.0, 1.0), // Red
                        Srgba::new(0.0, 1.0, 0.0, 1.0), // Green
                        Srgba::new(0.0, 0.0, 1.0, 1.0), // Blue
                        Srgba::new(1.0, 1.0, 0.0, 1.0), // Yellow
                        Srgba::new(1.0, 0.0, 1.0, 1.0), // Magenta
                    ];

                    let mut count = 0;
                    for row in 0..5 {
                        for col in 0..5 {
                            let x = start_x + col as f32 * spacing;
                            let y = start_y + row as f32 * spacing;
                            let color = colors[(row * 5 + col) % colors.len()];

                            cx.paint_quad(
                                Rect::new(Point::new(x, y), Size::new(quad_size, quad_size)),
                                color,
                            );
                            count += 1;
                        }
                    }

                    // Only print once
                    static PRINTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                    if !PRINTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                        eprintln!("Drawing {} quads", count);
                        eprintln!("Scene has {} quads", self.scene.quad_count());
                        eprintln!("Scale factor: {}", scale.0);

                        // Print positions of ALL quads
                        for (i, quad) in self.scene.quads().iter().enumerate() {
                            eprintln!("  Quad {}: pos=({:.0}, {:.0}) size=({:.0}, {:.0}) color=({:.1}, {:.1}, {:.1})",
                                i, quad.bounds.origin.x, quad.bounds.origin.y,
                                quad.bounds.size.width, quad.bounds.size.height,
                                quad.background.red, quad.background.green, quad.background.blue);
                        }
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
    eprintln!("=== Metal Instancing Debug ===");
    eprintln!("Should display a 5x5 grid of colored quads");

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
