//! Opens a window and renders a red quad using Metal.

use gesso_core::{
    metal::{MetalRenderer, MetalSurface},
    DeviceRect, Quad, Renderer, Scene, Srgba,
};
use glamour::{Point2, Size2};
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
                .with_title("Gesso - Hello Quad")
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
                if let (Some(renderer), Some(surface)) = (&mut self.renderer, &mut self.surface) {
                    // Build scene: red quad centered in window
                    self.scene.clear();

                    let (width, height) = surface.drawable_size();
                    let quad_size = 200.0;
                    let quad = Quad::new(
                        DeviceRect::new(
                            Point2::new((width - quad_size) / 2.0, (height - quad_size) / 2.0),
                            Size2::new(quad_size, quad_size),
                        ),
                        Srgba::new(1.0, 0.0, 0.0, 1.0), // Red
                    );
                    self.scene.push_quad(quad);

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
