//! Example demonstrating AccessKit integration with motif.
//!
//! This shows how to wire AccessKit's accessibility adapter into a motif application.

use accesskit_winit::Adapter;
use motif_core::{
    metal::{MetalRenderer, MetalSurface},
    AccessId, AccessNode, AccessRole, AccessTree, Corners, DeviceRect, Edges, FocusManager, Quad,
    Renderer, Scene, Srgba,
};
use glamour::{Point2, Size2};
use std::sync::{Arc, Mutex};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

/// Shared state for accessibility handlers.
/// Handlers may be called from different threads, so we use Arc<Mutex<>>.
struct AccessState {
    tree: AccessTree,
    focus: FocusManager,
}

impl AccessState {
    fn new() -> Self {
        // Root window node
        let root_id = AccessId(1);
        let mut tree = AccessTree::new(root_id);

        // Add root window node
        tree.push(
            AccessNode::new(root_id, AccessRole::Window, "Motif App".to_string())
                .with_child(AccessId(2)),
        );

        // Add a button
        tree.push(AccessNode::new(
            AccessId(2),
            AccessRole::Button,
            "Hello Button".to_string(),
        ));

        let mut focus = FocusManager::new();
        focus.set_focus_order(vec![AccessId(2)]);
        focus.set_focus(AccessId(2));

        Self { tree, focus }
    }

    fn build_tree_update(&self) -> accesskit::TreeUpdate {
        self.tree.build_initial_update(self.focus.focused())
    }
}

/// Activation handler - provides initial tree when screen reader connects.
struct ActivationHandler {
    state: Arc<Mutex<AccessState>>,
}

impl accesskit::ActivationHandler for ActivationHandler {
    fn request_initial_tree(&mut self) -> Option<accesskit::TreeUpdate> {
        let state = self.state.lock().unwrap();
        Some(state.build_tree_update())
    }
}

/// Action handler - handles requests from assistive technology.
struct ActionHandler {
    state: Arc<Mutex<AccessState>>,
}

impl accesskit::ActionHandler for ActionHandler {
    fn do_action(&mut self, request: accesskit::ActionRequest) {
        let mut state = self.state.lock().unwrap();
        match request.action {
            accesskit::Action::Focus => {
                // Set focus to the requested node
                state.focus.set_focus(AccessId(request.target_node.0));
                println!("Focus requested on node {:?}", request.target_node);
            }
            accesskit::Action::Click => {
                // Handle click/activate
                println!("Click action on node {:?}", request.target_node);
            }
            _ => {
                println!("Unhandled action: {:?}", request.action);
            }
        }
    }
}

/// Deactivation handler - cleans up when screen reader disconnects.
struct DeactivationHandler;

impl accesskit::DeactivationHandler for DeactivationHandler {
    fn deactivate_accessibility(&mut self) {
        println!("Accessibility deactivated");
    }
}

struct App {
    window: Option<Window>,
    renderer: Option<MetalRenderer>,
    surface: Option<MetalSurface>,
    scene: Scene,
    adapter: Option<Adapter>,
    access_state: Arc<Mutex<AccessState>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            surface: None,
            scene: Scene::new(),
            adapter: None,
            access_state: Arc::new(Mutex::new(AccessState::new())),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            // IMPORTANT: Window must start invisible for AccessKit
            let attrs = Window::default_attributes()
                .with_title("Motif - Accessible Window")
                .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
                .with_visible(false); // Start invisible!

            let window = event_loop.create_window(attrs).unwrap();

            // Create AccessKit adapter BEFORE showing window
            let adapter = Adapter::with_direct_handlers(
                event_loop,
                &window,
                ActivationHandler {
                    state: Arc::clone(&self.access_state),
                },
                ActionHandler {
                    state: Arc::clone(&self.access_state),
                },
                DeactivationHandler,
            );

            // Now show the window
            window.set_visible(true);

            let renderer = MetalRenderer::new();
            let surface = unsafe { MetalSurface::new(&window, renderer.device()) };

            window.request_redraw();
            self.window = Some(window);
            self.renderer = Some(renderer);
            self.surface = Some(surface);
            self.adapter = Some(adapter);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // Process accessibility events FIRST
        if let (Some(adapter), Some(window)) = (&mut self.adapter, &self.window) {
            adapter.process_event(window, &event);
        }

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
                    self.scene.clear();

                    let (width, height) = surface.drawable_size();
                    let quad_size = 200.0;

                    // Red quad representing the "button"
                    let mut quad = Quad::new(
                        DeviceRect::new(
                            Point2::new((width - quad_size) / 2.0, (height - quad_size) / 2.0),
                            Size2::new(quad_size, quad_size),
                        ),
                        Srgba::new(1.0, 0.0, 0.0, 1.0),
                    );
                    quad.border_color = Srgba::new(0.0, 0.0, 1.0, 1.0);
                    quad.border_widths = Edges::all(4.0);
                    quad.corner_radii = Corners::all(20.0);
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
