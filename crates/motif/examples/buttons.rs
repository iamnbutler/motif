//! Grid of clickable buttons for testing input simulation.
//!
//! Run with: cargo run --example buttons

use motif_core::{
    button,
    element::PaintContext,
    input::{InputState, MouseButton},
    metal::{MetalRenderer, MetalSurface},
    DrawContext, Element, ElementId, HitTree, Point, Rect, Renderer, ScaleFactor, Scene, Size,
    Srgba, TextContext,
};
use motif_debug::{DebugServer, InputStateSnapshot, SceneSnapshot};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

const COLS: usize = 4;
const ROWS: usize = 4;
const BUTTON_SIZE: f32 = 100.0;
const GAP: f32 = 20.0;
const MARGIN: f32 = 40.0;

struct App {
    window: Option<Window>,
    renderer: Option<MetalRenderer>,
    surface: Option<MetalSurface>,
    scene: Scene,
    text_ctx: TextContext,
    hit_tree: HitTree,
    debug_server: Option<DebugServer>,
    input_state: InputState,
    /// Click count for each button
    clicks: [[u32; COLS]; ROWS],
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            surface: None,
            scene: Scene::new(),
            text_ctx: TextContext::new(),
            hit_tree: HitTree::new(),
            debug_server: DebugServer::new().ok(),
            input_state: InputState::new(),
            clicks: [[0; COLS]; ROWS],
        }
    }
}

fn button_id(row: usize, col: usize) -> ElementId {
    ElementId((row * COLS + col) as u64)
}

fn button_bounds(row: usize, col: usize) -> Rect {
    Rect::new(
        Point::new(
            MARGIN + col as f32 * (BUTTON_SIZE + GAP),
            MARGIN + row as f32 * (BUTTON_SIZE + GAP),
        ),
        Size::new(BUTTON_SIZE, BUTTON_SIZE),
    )
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let width = MARGIN * 2.0 + COLS as f32 * BUTTON_SIZE + (COLS - 1) as f32 * GAP;
            let height = MARGIN * 2.0 + ROWS as f32 * BUTTON_SIZE + (ROWS - 1) as f32 * GAP;

            let attrs = Window::default_attributes()
                .with_title("Motif — Button Grid")
                .with_inner_size(winit::dpi::LogicalSize::new(width, height))
                .with_resizable(false);
            let window = event_loop.create_window(attrs).unwrap();

            let renderer = MetalRenderer::new();
            let surface = unsafe { MetalSurface::new(&window, renderer.device()) };

            if let Some(ref debug_server) = self.debug_server {
                if let Some(id) = motif_core::metal::window_id(&window) {
                    debug_server.set_window_id(id);
                }
            }

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
                    self.hit_tree.clear();

                    let scale = ScaleFactor(window.scale_factor() as f32);
                    let mut cx = DrawContext::new(&mut self.scene, scale);

                    // Background
                    let phys = window.inner_size();
                    cx.paint_quad(
                        Rect::new(
                            Point::new(0.0, 0.0),
                            Size::new(phys.width as f32 / scale.0, phys.height as f32 / scale.0),
                        ),
                        Srgba::new(0.08, 0.08, 0.1, 1.0),
                    );

                    // Paint button grid
                    for row in 0..ROWS {
                        for col in 0..COLS {
                            let id = button_id(row, col);
                            let bounds = button_bounds(row, col);
                            let clicks = self.clicks[row][col];

                            let is_hovered = self.input_state.hovered() == Some(id);
                            let is_pressed = self.input_state.pressed() == Some(id);

                            let color = if is_pressed {
                                Srgba::new(0.2, 0.4, 0.7, 1.0)
                            } else if is_hovered {
                                Srgba::new(0.3, 0.5, 0.8, 1.0)
                            } else {
                                Srgba::new(0.15, 0.2, 0.3, 1.0)
                            };

                            cx.paint_quad(bounds, color);
                            self.hit_tree.push(id, bounds);

                            // Button label
                            let label = format!("{}", clicks);
                            cx.paint_text(
                                &label,
                                Point::new(
                                    bounds.origin.x + BUTTON_SIZE / 2.0 - 10.0,
                                    bounds.origin.y + BUTTON_SIZE / 2.0 - 12.0,
                                ),
                                32.0,
                                Srgba::new(1.0, 1.0, 1.0, 1.0),
                                &mut self.text_ctx,
                            );

                            // Row,col label
                            let pos_label = format!("{},{}", row, col);
                            cx.paint_text(
                                &pos_label,
                                Point::new(bounds.origin.x + 8.0, bounds.origin.y + 20.0),
                                12.0,
                                Srgba::new(0.5, 0.5, 0.6, 1.0),
                                &mut self.text_ctx,
                            );
                        }
                    }

                    renderer.render(&self.scene, surface);

                    // Update debug server
                    if let Some(ref debug_server) = self.debug_server {
                        let phys = window.inner_size();
                        let viewport = (phys.width as f32, phys.height as f32);
                        let snapshot =
                            SceneSnapshot::from_scene(&self.scene, viewport, scale.0);
                        debug_server.update_scene(snapshot);

                        if let Ok(inner_pos) = window.inner_position() {
                            // inner_position is physical pixels, convert to logical for CGEvent
                            debug_server.set_window_position(
                                inner_pos.x as f32 / scale.0,
                                inner_pos.y as f32 / scale.0,
                                scale.0,
                            );
                        }
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.window.as_ref().map(|w| w.scale_factor() as f32).unwrap_or(1.0);
                self.input_state.handle_cursor_moved(position.x, position.y, scale);

                if let Some(pos) = self.input_state.cursor_position {
                    let hovered = self.hit_tree.hit_test(pos);
                    self.input_state.set_hovered(hovered);
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.input_state.handle_cursor_left();
                self.input_state.set_hovered(None);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = MouseButton::from_winit(button);
                if state == winit::event::ElementState::Pressed {
                    self.input_state.handle_mouse_button(btn, true);
                    self.input_state.begin_press();
                } else {
                    if let Some(clicked_id) = self.input_state.end_press() {
                        // Find which button was clicked
                        let id = clicked_id.0 as usize;
                        let row = id / COLS;
                        let col = id % COLS;
                        if row < ROWS && col < COLS {
                            self.clicks[row][col] += 1;
                        }
                    }
                    self.input_state.handle_mouse_button(btn, false);
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }

        if let Some(ref debug_server) = self.debug_server {
            let snapshot = InputStateSnapshot::from_input_state(&self.input_state);
            debug_server.update_input(snapshot);
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
