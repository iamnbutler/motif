//! TodoMVC — canonical to-do list app built with motif's immediate-mode draw API.
//!
//! Demonstrates real-world interactive UI:
//! - Text input with keyboard editing (type to append, Backspace to delete, Enter to add)
//! - Toggle todo completion by clicking the checkbox
//! - Delete individual todos with the × button (visible on hover)
//! - Filter todos: All / Active / Completed
//! - Items remaining count and "Clear completed" button
//!
//! Run with: `cargo run --example todomvc`
//!
//! Note: requires macOS (Metal GPU backend).

use motif_core::{
    input::{ElementState, Key, MouseButton, NamedKey, ScrollDelta},
    metal::{MetalRenderer, MetalSurface},
    Corners, DevicePoint, DeviceRect, DeviceSize, DrawContext, Edges, ElementId, HitTree,
    InputState, Point, Quad, Rect, ScaleFactor, Scene, Size, Srgba, TextContext,
};
use motif_debug::{DebugServer, InputStateSnapshot, SceneSnapshot};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

// ── Domain ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Todo {
    id: u64,
    text: String,
    done: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    All,
    Active,
    Completed,
}

// ── Hit-test element IDs ───────────────────────────────────────────────────────
//
// ID space:
//   1          → new-todo input field
//   100 + i*2  → toggle checkbox for todo at index i (max 399 todos before collision)
//   101 + i*2  → delete button for todo at index i
//   900        → "All" filter tab
//   901        → "Active" filter tab
//   902        → "Completed" filter tab
//   903        → "Clear completed" button

const ID_INPUT: u64 = 1;
const ID_FILTER_ALL: u64 = 900;
const ID_FILTER_ACTIVE: u64 = 901;
const ID_FILTER_COMPLETED: u64 = 902;
const ID_CLEAR_COMPLETED: u64 = 903;

fn toggle_id(index: usize) -> ElementId {
    ElementId(100 + index as u64 * 2)
}

fn delete_id(index: usize) -> ElementId {
    ElementId(101 + index as u64 * 2)
}

// ── App state ─────────────────────────────────────────────────────────────────

struct App {
    window: Option<Window>,
    renderer: Option<MetalRenderer>,
    surface: Option<MetalSurface>,
    scene: Scene,
    text_ctx: TextContext,
    hit_tree: HitTree,
    input_state: InputState,
    todos: Vec<Todo>,
    next_id: u64,
    /// Text being composed in the new-todo field.
    input_text: String,
    /// Whether the new-todo field has keyboard focus.
    input_focused: bool,
    filter: Filter,
    debug_server: Option<DebugServer>,
}

impl Default for App {
    fn default() -> Self {
        let debug_server = DebugServer::new().ok();
        let todos = vec![
            Todo {
                id: 1,
                text: "Buy groceries".into(),
                done: true,
            },
            Todo {
                id: 2,
                text: "Write a motif app".into(),
                done: false,
            },
            Todo {
                id: 3,
                text: "Learn Rust".into(),
                done: false,
            },
        ];
        Self {
            window: None,
            renderer: None,
            surface: None,
            scene: Scene::new(),
            text_ctx: TextContext::new(),
            hit_tree: HitTree::new(),
            input_state: InputState::new(),
            todos,
            next_id: 4,
            input_text: String::new(),
            input_focused: false,
            filter: Filter::All,
            debug_server,
        }
    }
}

impl App {
    fn active_count(&self) -> usize {
        self.todos.iter().filter(|t| !t.done).count()
    }

    fn completed_count(&self) -> usize {
        self.todos.iter().filter(|t| t.done).count()
    }

    fn add_todo(&mut self) {
        let text = self.input_text.trim().to_string();
        if !text.is_empty() {
            self.todos.push(Todo {
                id: self.next_id,
                text,
                done: false,
            });
            self.next_id += 1;
            self.input_text.clear();
        }
    }

    fn handle_click(&mut self, id: ElementId) {
        let raw = id.0;

        if raw == ID_INPUT {
            self.input_focused = true;
            return;
        }
        if raw == ID_FILTER_ALL {
            self.filter = Filter::All;
            return;
        }
        if raw == ID_FILTER_ACTIVE {
            self.filter = Filter::Active;
            return;
        }
        if raw == ID_FILTER_COMPLETED {
            self.filter = Filter::Completed;
            return;
        }
        if raw == ID_CLEAR_COMPLETED {
            self.todos.retain(|t| !t.done);
            return;
        }

        // Todo toggle / delete — IDs start at 100
        if raw >= 100 && raw < 900 {
            let offset = raw - 100;
            let orig_idx = (offset / 2) as usize;
            let is_delete = (offset % 2) == 1;
            if orig_idx < self.todos.len() {
                if is_delete {
                    self.todos.remove(orig_idx);
                } else {
                    self.todos[orig_idx].done = !self.todos[orig_idx].done;
                }
            }
        }
    }

    fn handle_key_press(&mut self, key: &Key) {
        match key {
            Key::Named(NamedKey::Enter) => self.add_todo(),
            Key::Named(NamedKey::Backspace) => {
                // Remove the last Unicode scalar value
                let mut chars = self.input_text.chars();
                chars.next_back();
                self.input_text = chars.as_str().to_string();
            }
            Key::Named(NamedKey::Escape) => {
                self.input_text.clear();
                self.input_focused = false;
            }
            Key::Character(ch) => {
                self.input_text.push_str(ch.as_str());
            }
            _ => {}
        }
    }
}

// ── Layout constants (logical pixels) ─────────────────────────────────────────

const WIN_W: f32 = 550.0;
const WIN_H: f32 = 700.0;
const MARGIN: f32 = 30.0;
const PANEL_W: f32 = WIN_W - MARGIN * 2.0;
const INPUT_H: f32 = 50.0;
const ROW_H: f32 = 52.0;
const FOOTER_H: f32 = 44.0;
const CHECKBOX_SIZE: f32 = 18.0;
const CHECKBOX_LEFT: f32 = 14.0;
const TEXT_LEFT: f32 = CHECKBOX_LEFT + CHECKBOX_SIZE + 10.0;

// ── Paint helpers ─────────────────────────────────────────────────────────────

/// Paint a filled rounded rectangle.
fn paint_rrect(cx: &mut DrawContext, bounds: Rect, fill: Srgba, radius: f32, scale: f32) {
    let mut quad = Quad::new(
        DeviceRect::new(
            DevicePoint::new(bounds.origin.x * scale, bounds.origin.y * scale),
            DeviceSize::new(bounds.size.width * scale, bounds.size.height * scale),
        ),
        fill,
    );
    quad.corner_radii = Corners::all(radius * scale);
    cx.paint(quad);
}

/// Paint a rounded rectangle with a border.
fn paint_rrect_border(
    cx: &mut DrawContext,
    bounds: Rect,
    fill: Srgba,
    border: Srgba,
    border_w: f32,
    radius: f32,
    scale: f32,
) {
    let mut quad = Quad::new(
        DeviceRect::new(
            DevicePoint::new(bounds.origin.x * scale, bounds.origin.y * scale),
            DeviceSize::new(bounds.size.width * scale, bounds.size.height * scale),
        ),
        fill,
    );
    quad.border_color = border;
    quad.border_widths = Edges::all(border_w * scale);
    quad.corner_radii = Corners::all(radius * scale);
    cx.paint(quad);
}

// ── Rendering ─────────────────────────────────────────────────────────────────

impl App {
    fn paint(&mut self, scale: ScaleFactor) {
        self.scene.clear();
        self.hit_tree.clear();
        let s = scale.0;

        let mut cx = DrawContext::new(&mut self.scene, scale);

        // ── Background ────────────────────────────────────────────────────────
        cx.paint_quad(
            Rect::new(Point::new(0.0, 0.0), Size::new(WIN_W, WIN_H)),
            Srgba::new(0.09, 0.09, 0.11, 1.0),
        );

        let x = MARGIN;
        let mut y = MARGIN;

        // ── Title ─────────────────────────────────────────────────────────────
        cx.paint_text(
            "todos",
            Point::new(x + PANEL_W / 2.0 - 46.0, y + 38.0),
            44.0,
            Srgba::new(0.85, 0.35, 0.35, 0.60),
            &mut self.text_ctx,
        );
        y += 64.0;

        // ── New-todo input ─────────────────────────────────────────────────────
        let input_bounds = Rect::new(Point::new(x, y), Size::new(PANEL_W, INPUT_H));
        let border_col = if self.input_focused {
            Srgba::new(0.30, 0.60, 0.95, 1.0) // focused: accent
        } else {
            Srgba::new(0.25, 0.25, 0.30, 1.0)
        };
        paint_rrect_border(
            &mut cx,
            input_bounds,
            Srgba::new(0.15, 0.15, 0.18, 1.0),
            border_col,
            1.5,
            6.0,
            s,
        );
        self.hit_tree.push(ElementId(ID_INPUT), input_bounds);

        // Placeholder or typed text
        let (display_text, txt_color): (&str, Srgba) = if self.input_text.is_empty() {
            ("What needs to be done?", Srgba::new(0.35, 0.35, 0.40, 1.0))
        } else {
            (self.input_text.as_str(), Srgba::new(0.85, 0.85, 0.90, 1.0))
        };
        cx.paint_text(
            display_text,
            Point::new(x + 14.0, y + 33.0),
            16.0,
            txt_color,
            &mut self.text_ctx,
        );

        // Text cursor
        if self.input_focused && !self.input_text.is_empty() {
            let layout = self.text_ctx.layout_text(&self.input_text, 16.0);
            let cursor_x = x + 14.0 + layout.width();
            cx.paint_quad(
                Rect::new(Point::new(cursor_x, y + 13.0), Size::new(2.0, 22.0)),
                Srgba::new(0.30, 0.60, 0.95, 1.0),
            );
        }

        // Submit hint
        cx.paint_text(
            "↵ to add",
            Point::new(x + PANEL_W - 68.0, y + 33.0),
            11.0,
            Srgba::new(0.30, 0.30, 0.35, 1.0),
            &mut self.text_ctx,
        );

        y += INPUT_H + 2.0;

        // ── Todo list ──────────────────────────────────────────────────────────
        let visible: Vec<usize> = self
            .todos
            .iter()
            .enumerate()
            .filter(|(_, t)| match self.filter {
                Filter::All => true,
                Filter::Active => !t.done,
                Filter::Completed => t.done,
            })
            .map(|(i, _)| i)
            .collect();

        for (row, &orig_idx) in visible.iter().enumerate() {
            let todo = &self.todos[orig_idx];
            let row_y = y + row as f32 * ROW_H;
            let row_bounds = Rect::new(Point::new(x, row_y), Size::new(PANEL_W, ROW_H));

            // Hover state: row is hovered if either its toggle or delete ID is hovered
            let toggle = toggle_id(orig_idx);
            let del = delete_id(orig_idx);
            let hovered = self.input_state.hovered() == Some(toggle)
                || self.input_state.hovered() == Some(del);

            // Row background (alternating + hover)
            let row_fill = if hovered {
                Srgba::new(0.17, 0.17, 0.22, 1.0)
            } else if row % 2 == 0 {
                Srgba::new(0.12, 0.12, 0.15, 1.0)
            } else {
                Srgba::new(0.13, 0.13, 0.16, 1.0)
            };
            cx.paint_quad(row_bounds, row_fill);

            // Separator above rows (not the first)
            if row > 0 {
                cx.paint_quad(
                    Rect::new(Point::new(x, row_y), Size::new(PANEL_W, 1.0)),
                    Srgba::new(0.20, 0.20, 0.25, 1.0),
                );
            }

            // ── Checkbox ──────────────────────────────────────────────────────
            let cb_x = x + CHECKBOX_LEFT;
            let cb_y = row_y + (ROW_H - CHECKBOX_SIZE) / 2.0;
            let cb_bounds = Rect::new(
                Point::new(cb_x, cb_y),
                Size::new(CHECKBOX_SIZE, CHECKBOX_SIZE),
            );

            if todo.done {
                // Filled checkbox with checkmark
                paint_rrect(
                    &mut cx,
                    cb_bounds,
                    Srgba::new(0.20, 0.60, 0.40, 0.90),
                    4.0,
                    s,
                );
                cx.paint_text(
                    "✓",
                    Point::new(cb_x + 2.0, cb_y + 14.0),
                    13.0,
                    Srgba::new(1.0, 1.0, 1.0, 1.0),
                    &mut self.text_ctx,
                );
            } else {
                // Empty bordered checkbox
                paint_rrect_border(
                    &mut cx,
                    cb_bounds,
                    Srgba::new(0.0, 0.0, 0.0, 0.0),
                    Srgba::new(0.35, 0.35, 0.40, 1.0),
                    1.5,
                    4.0,
                    s,
                );
            }
            self.hit_tree.push(toggle, cb_bounds);

            // ── Todo text ─────────────────────────────────────────────────────
            let text_col = if todo.done {
                Srgba::new(0.40, 0.40, 0.45, 1.0) // dimmed / strikethrough effect
            } else {
                Srgba::new(0.85, 0.85, 0.90, 1.0)
            };
            cx.paint_text(
                &todo.text,
                Point::new(x + TEXT_LEFT, row_y + 33.0),
                16.0,
                text_col,
                &mut self.text_ctx,
            );

            // ── Delete button (shown on hover) ────────────────────────────────
            if hovered {
                let del_bounds = Rect::new(
                    Point::new(x + PANEL_W - 34.0, row_y + (ROW_H - 22.0) / 2.0),
                    Size::new(24.0, 22.0),
                );
                paint_rrect(
                    &mut cx,
                    del_bounds,
                    Srgba::new(0.55, 0.15, 0.15, 0.80),
                    4.0,
                    s,
                );
                cx.paint_text(
                    "×",
                    Point::new(del_bounds.origin.x + 5.0, del_bounds.origin.y + 16.0),
                    15.0,
                    Srgba::new(1.0, 0.75, 0.75, 1.0),
                    &mut self.text_ctx,
                );
                self.hit_tree.push(del, del_bounds);
            }
        }

        let list_h = visible.len() as f32 * ROW_H;
        y += list_h;

        // ── Footer ────────────────────────────────────────────────────────────
        if !self.todos.is_empty() {
            // Top border
            cx.paint_quad(
                Rect::new(Point::new(x, y), Size::new(PANEL_W, 1.0)),
                Srgba::new(0.20, 0.20, 0.25, 1.0),
            );

            // Items remaining count
            let count = self.active_count();
            let items_label = if count == 1 {
                "1 item left".to_string()
            } else {
                format!("{count} items left")
            };
            cx.paint_text(
                &items_label,
                Point::new(x + 6.0, y + 28.0),
                12.0,
                Srgba::new(0.45, 0.45, 0.50, 1.0),
                &mut self.text_ctx,
            );

            // Filter tabs: All | Active | Completed
            let tabs = [
                (Filter::All, "All", ID_FILTER_ALL),
                (Filter::Active, "Active", ID_FILTER_ACTIVE),
                (Filter::Completed, "Completed", ID_FILTER_COMPLETED),
            ];
            let tab_w = 72.0;
            let tabs_total = tabs.len() as f32 * tab_w;
            let mut tab_x = x + (PANEL_W - tabs_total) / 2.0;

            for (f, label, id) in &tabs {
                let active = self.filter == *f;
                let tab_bounds =
                    Rect::new(Point::new(tab_x, y + 8.0), Size::new(tab_w - 4.0, 28.0));

                if active {
                    paint_rrect_border(
                        &mut cx,
                        tab_bounds,
                        Srgba::new(0.20, 0.20, 0.28, 1.0),
                        Srgba::new(0.30, 0.60, 0.95, 1.0),
                        1.0,
                        4.0,
                        s,
                    );
                }

                cx.paint_text(
                    label,
                    Point::new(tab_x + 8.0, y + 27.0),
                    12.0,
                    if active {
                        Srgba::new(0.85, 0.85, 0.90, 1.0)
                    } else {
                        Srgba::new(0.45, 0.45, 0.50, 1.0)
                    },
                    &mut self.text_ctx,
                );
                self.hit_tree.push(ElementId(*id), tab_bounds);
                tab_x += tab_w;
            }

            // "Clear completed" button
            if self.completed_count() > 0 {
                let cc_x = x + PANEL_W - 114.0;
                let cc_bounds = Rect::new(Point::new(cc_x, y + 8.0), Size::new(110.0, 28.0));
                cx.paint_text(
                    "Clear completed",
                    Point::new(cc_x, y + 27.0),
                    12.0,
                    Srgba::new(0.45, 0.45, 0.50, 1.0),
                    &mut self.text_ctx,
                );
                self.hit_tree.push(ElementId(ID_CLEAR_COMPLETED), cc_bounds);
            }

            y += FOOTER_H;
        }

        // ── Empty state message ───────────────────────────────────────────────
        if visible.is_empty() {
            cx.paint_text(
                "No todos to show.",
                Point::new(x + PANEL_W / 2.0 - 50.0, y + 28.0),
                14.0,
                Srgba::new(0.30, 0.30, 0.35, 1.0),
                &mut self.text_ctx,
            );
        }

        let _ = y; // suppress unused warning after last use
    }
}

// ── winit ApplicationHandler ──────────────────────────────────────────────────

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("Motif — TodoMVC")
                .with_inner_size(winit::dpi::LogicalSize::new(WIN_W, WIN_H));
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

            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface {
                    surface.resize(size.width as f32, size.height as f32);
                }
            }

            WindowEvent::RedrawRequested => {
                // Paint first (needs exclusive &mut self, so do it before binding renderer/surface)
                let scale = if let Some(w) = &self.window {
                    ScaleFactor(w.scale_factor() as f32)
                } else {
                    return;
                };
                self.paint(scale);

                // Render the built scene
                if let (Some(renderer), Some(surface)) = (&mut self.renderer, &mut self.surface) {
                    renderer.render(&self.scene, surface);
                }

                // Debug server snapshot
                if let (Some(ref debug_server), Some(window)) = (&self.debug_server, &self.window) {
                    let phys = window.inner_size();
                    let viewport = (phys.width as f32, phys.height as f32);
                    let snapshot = SceneSnapshot::from_scene(&self.scene, viewport, scale.0);
                    debug_server.update_scene(snapshot);

                    if let Ok(inner_pos) = window.inner_position() {
                        debug_server.set_window_position(
                            inner_pos.x as f32 / scale.0,
                            inner_pos.y as f32 / scale.0,
                            scale.0,
                        );
                    }
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor() as f32)
                    .unwrap_or(1.0);
                self.input_state
                    .handle_cursor_moved(position.x, position.y, scale);
                if let Some(pos) = self.input_state.cursor_position {
                    let hovered = self.hit_tree.hit_test(pos);
                    self.input_state.set_hovered(hovered);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::CursorEntered { .. } => self.input_state.handle_cursor_entered(),

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
                    if let Some(clicked) = self.input_state.end_press() {
                        self.handle_click(clicked);
                    } else {
                        // Clicked outside all registered elements — blur input
                        self.input_focused = false;
                    }
                    self.input_state.handle_mouse_button(btn, false);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor() as f32)
                    .unwrap_or(1.0);
                self.input_state
                    .handle_scroll(ScrollDelta::from_winit(delta, scale));
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.input_state.handle_modifiers_changed(mods.state());
            }

            WindowEvent::KeyboardInput { event, .. } => {
                self.input_state.handle_key(
                    event.logical_key.clone(),
                    event.physical_key,
                    event.state,
                );

                if event.state == ElementState::Pressed && self.input_focused {
                    self.handle_key_press(&event.logical_key);
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

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
