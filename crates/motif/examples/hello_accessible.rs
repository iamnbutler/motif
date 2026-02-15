//! Example demonstrating AccessKit integration with motif text.
//!
//! Run with: cargo run --example hello_accessible
//! Then enable VoiceOver (Cmd+F5) and navigate to hear the text.

use accesskit_winit::Adapter;
use motif_core::{
    metal::{MetalRenderer, MetalSurface},
    AccessId, AccessNode, AccessRole, AccessTree, DrawContext, FocusManager, Point, Rect,
    Renderer, ScaleFactor, Scene, Size, Srgba, TextContext,
};
use std::sync::{Arc, Mutex};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

/// Shared state for accessibility - rebuilt each frame.
struct AccessState {
    tree: AccessTree,
    focus: FocusManager,
}

impl AccessState {
    fn new() -> Self {
        let root_id = AccessId(0);
        let tree = AccessTree::new(root_id);
        let focus = FocusManager::new();
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
                state.focus.set_focus(AccessId(request.target_node.0));
                println!("Focus requested on node {:?}", request.target_node);
            }
            accesskit::Action::Click => {
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
    text_ctx: TextContext,
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
            text_ctx: TextContext::new(),
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
                .with_title("Motif - Accessible Text")
                .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
                .with_visible(false);

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

                    // Rebuild accessibility tree each frame
                    let mut state = self.access_state.lock().unwrap();
                    state.tree.clear();

                    // Add root window node
                    let root_id = AccessId(0);
                    let window_bounds = Rect::new(Point::new(0.0, 0.0), Size::new(800.0, 600.0));
                    state.tree.push(
                        AccessNode::new(root_id, AccessRole::Window, "Motif Accessible App".to_string())
                            .with_bounds(window_bounds)
                            .with_child(AccessId(1))
                            .with_child(AccessId(2))
                            .with_child(AccessId(3)),
                    );

                    let scale = ScaleFactor(window.scale_factor() as f32);
                    let mut cx = DrawContext::with_accessibility(
                        &mut self.scene,
                        &mut state.tree,
                        scale,
                    );

                    // Background
                    cx.paint_quad(
                        Rect::new(Point::new(50.0, 50.0), Size::new(700.0, 200.0)),
                        Srgba::new(0.15, 0.15, 0.2, 1.0),
                    );

                    // Paint text - automatically creates accessibility nodes!
                    cx.paint_text(
                        "Hello, Accessibility!",
                        Point::new(70.0, 120.0),
                        48.0,
                        Srgba::new(1.0, 1.0, 1.0, 1.0),
                        &mut self.text_ctx,
                    );

                    cx.paint_text(
                        "VoiceOver can read this text.",
                        Point::new(70.0, 180.0),
                        24.0,
                        Srgba::new(0.8, 0.8, 0.8, 1.0),
                        &mut self.text_ctx,
                    );

                    cx.paint_text(
                        "Press Cmd+F5 to enable VoiceOver, then Ctrl+Option+arrows to navigate.",
                        Point::new(70.0, 220.0),
                        16.0,
                        Srgba::new(0.6, 0.6, 0.6, 1.0),
                        &mut self.text_ctx,
                    );

                    // Update focus order with the text element IDs (1, 2, 3)
                    state.focus.set_focus_order(vec![AccessId(1), AccessId(2), AccessId(3)]);

                    // Update the adapter with the new tree
                    drop(state); // Release lock before updating adapter

                    if let Some(adapter) = &mut self.adapter {
                        let state = self.access_state.lock().unwrap();
                        adapter.update_if_active(|| state.build_tree_update());
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
    println!("Accessible Text Example");
    println!("========================");
    println!("1. Enable VoiceOver: Cmd+F5");
    println!("2. Navigate with: Ctrl+Option+Arrow keys");
    println!("3. VoiceOver should announce the text content");
    println!();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
