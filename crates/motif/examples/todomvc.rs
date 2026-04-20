//! TodoMVC example demonstrating text input, lists, and interactivity.
//!
//! Run with: cargo run --example todomvc

use motif_core::{
    checkbox,
    element::{Element, LayoutContext, PaintContext},
    input::{HandleKeyResult, InputState, MouseButton, TextEditState},
    metal::{MetalRenderer, MetalSurface},
    text_input, DrawContext, ElementId, HitTree, LayoutEngine, Point, Rect, Renderer, ScaleFactor,
    Scene, Size, Srgba, TextContext,
};
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

// ============================================================================
// Todo Model
// ============================================================================

#[derive(Clone)]
struct Todo {
    id: usize,
    text: String,
    completed: bool,
}

// ============================================================================
// App State
// ============================================================================

struct TodoApp {
    window: Option<Window>,
    surface: Option<MetalSurface>,
    renderer: Option<MetalRenderer>,
    scene: Scene,
    hit_tree: HitTree,
    text_ctx: TextContext,
    layout_engine: LayoutEngine,

    // Input state
    input_state: InputState,
    new_todo_state: TextEditState,
    new_todo_focused: bool,
    /// Whether the cursor is in the visible phase of its blink cycle.
    cursor_visible: bool,
    /// Timestamp of the last blink toggle (used to schedule the next one).
    cursor_blink_epoch: Instant,

    // Todo data
    todos: Vec<Todo>,
    next_id: usize,
}

impl TodoApp {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            renderer: None,
            scene: Scene::new(),
            hit_tree: HitTree::new(),
            text_ctx: TextContext::new(),
            layout_engine: LayoutEngine::new(),

            input_state: InputState::new(),
            new_todo_state: TextEditState::new(),
            new_todo_focused: true, // Start focused
            cursor_visible: true,
            cursor_blink_epoch: Instant::now(),

            todos: vec![
                Todo {
                    id: 1,
                    text: "Learn motif".to_string(),
                    completed: false,
                },
                Todo {
                    id: 2,
                    text: "Build a todo app".to_string(),
                    completed: true,
                },
                Todo {
                    id: 3,
                    text: "Ship it!".to_string(),
                    completed: false,
                },
            ],
            next_id: 4,
        }
    }

    fn add_todo(&mut self, text: String) {
        if text.trim().is_empty() {
            return;
        }
        self.todos.push(Todo {
            id: self.next_id,
            text: text.trim().to_string(),
            completed: false,
        });
        self.next_id += 1;
        self.new_todo_state.set_content("");
    }

    fn toggle_todo(&mut self, id: usize) {
        if let Some(todo) = self.todos.iter_mut().find(|t| t.id == id) {
            todo.completed = !todo.completed;
        }
    }

    fn delete_todo(&mut self, id: usize) {
        self.todos.retain(|t| t.id != id);
    }

    fn items_left(&self) -> usize {
        self.todos.iter().filter(|t| !t.completed).count()
    }
}

// ============================================================================
// Rendering
// ============================================================================

impl TodoApp {
    fn paint(&mut self) {
        let scale = ScaleFactor(
            self.window
                .as_ref()
                .map(|w| w.scale_factor() as f32)
                .unwrap_or(1.0),
        );

        self.scene.clear();
        self.hit_tree.clear();

        // Get window size
        let size = self
            .window
            .as_ref()
            .map(|w| {
                let s = w.inner_size();
                Size::new(s.width as f32 / scale.0, s.height as f32 / scale.0)
            })
            .unwrap_or(Size::new(800.0, 600.0));

        // Background
        {
            let mut cx = DrawContext::new(&mut self.scene, scale);
            cx.paint_quad(
                Rect::new(Point::new(0.0, 0.0), size),
                Srgba::new(0.96, 0.96, 0.96, 1.0), // Light gray background
            );
        }

        // App container
        let container_width = 500.0;
        let container_x = (size.width - container_width) / 2.0;
        let mut y = 40.0;

        // Title
        {
            let mut cx = DrawContext::new(&mut self.scene, scale);
            cx.paint_text(
                "todos",
                Point::new(container_x + container_width / 2.0 - 60.0, y + 40.0),
                48.0,
                Srgba::new(0.69, 0.54, 0.54, 0.3), // Muted red
                &mut self.text_ctx,
            );
        }
        y += 100.0;

        // Input card background
        {
            let mut cx = DrawContext::new(&mut self.scene, scale);
            cx.paint_quad(
                Rect::new(Point::new(container_x, y), Size::new(container_width, 60.0)),
                Srgba::new(1.0, 1.0, 1.0, 1.0),
            );
        }

        // New todo input
        {
            let input_id = ElementId(1000);
            let input_bounds = Rect::new(
                Point::new(container_x + 16.0, y + 12.0),
                Size::new(container_width - 32.0, 36.0),
            );

            let mut input = text_input(self.new_todo_state.content(), input_id)
                .placeholder("What needs to be done?")
                .focused(self.new_todo_focused)
                .cursor_visible(self.cursor_visible)
                .cursor_pos(self.new_todo_state.cursor_offset())
                .selection(self.new_todo_state.selected_range().clone())
                .font_size(18.0);

            // Layout phase
            let mut layout_cx =
                LayoutContext::new(&mut self.layout_engine, &mut self.text_ctx, scale);
            let node_id = input.request_layout(&mut layout_cx);
            self.layout_engine
                .compute_layout(node_id, 800.0, 600.0, &mut self.text_ctx);

            // Paint at desired position with offset
            let layout_bounds = self.layout_engine.layout_bounds(node_id);
            let offset = Point::new(
                input_bounds.origin.x - layout_bounds.origin.x,
                input_bounds.origin.y - layout_bounds.origin.y,
            );

            let mut pcx = PaintContext::new(
                &mut self.scene,
                &mut self.text_ctx,
                &mut self.hit_tree,
                &self.layout_engine,
                scale,
            );
            pcx.set_offset(offset);
            input.paint(input_bounds, &mut pcx);
        }

        y += 60.0;

        // Todo list
        let todos_snapshot = self.todos.to_vec();
        for todo in &todos_snapshot {
            // Todo item background
            {
                let mut cx = DrawContext::new(&mut self.scene, scale);
                cx.paint_quad(
                    Rect::new(Point::new(container_x, y), Size::new(container_width, 50.0)),
                    Srgba::new(1.0, 1.0, 1.0, 1.0),
                );

                // Separator line
                cx.paint_quad(
                    Rect::new(Point::new(container_x, y), Size::new(container_width, 1.0)),
                    Srgba::new(0.9, 0.9, 0.9, 1.0),
                );
            }

            // Checkbox - ID is 2000 + todo.id
            {
                let checkbox_id = ElementId(2000 + todo.id as u64);
                let checkbox_pos = Point::new(container_x + 16.0, y + 13.0);
                let checkbox_bounds = Rect::new(checkbox_pos, Size::new(18.0, 18.0));

                let mut cb = checkbox(checkbox_id).checked(todo.completed);

                // Layout phase
                let mut layout_cx =
                    LayoutContext::new(&mut self.layout_engine, &mut self.text_ctx, scale);
                let node_id = cb.request_layout(&mut layout_cx);
                self.layout_engine
                    .compute_layout(node_id, 800.0, 600.0, &mut self.text_ctx);

                // Paint at desired position with offset
                let layout_bounds = self.layout_engine.layout_bounds(node_id);
                let offset = Point::new(
                    checkbox_bounds.origin.x - layout_bounds.origin.x,
                    checkbox_bounds.origin.y - layout_bounds.origin.y,
                );

                let mut pcx = PaintContext::new(
                    &mut self.scene,
                    &mut self.text_ctx,
                    &mut self.hit_tree,
                    &self.layout_engine,
                    scale,
                );
                pcx.set_offset(offset);
                cb.paint(checkbox_bounds, &mut pcx);
            }

            // Todo text
            {
                let text_color = if todo.completed {
                    Srgba::new(0.7, 0.7, 0.7, 1.0) // Gray for completed
                } else {
                    Srgba::new(0.2, 0.2, 0.2, 1.0)
                };
                let mut cx = DrawContext::new(&mut self.scene, scale);
                cx.paint_text(
                    &todo.text,
                    Point::new(container_x + 56.0, y + 32.0),
                    18.0,
                    text_color,
                    &mut self.text_ctx,
                );
            }

            // Delete button (X) - ID is 3000 + todo.id
            {
                let delete_id = ElementId(3000 + todo.id as u64);
                let delete_bounds = Rect::new(
                    Point::new(container_x + container_width - 40.0, y + 15.0),
                    Size::new(20.0, 20.0),
                );

                let mut cx = DrawContext::new(&mut self.scene, scale);
                cx.paint_text(
                    "×",
                    delete_bounds.origin + Point::new(4.0, 16.0),
                    20.0,
                    Srgba::new(0.8, 0.4, 0.4, 1.0),
                    &mut self.text_ctx,
                );
                self.hit_tree.push(delete_id, delete_bounds);
            }

            y += 50.0;
        }

        // Footer
        if !self.todos.is_empty() {
            y += 10.0;
            let items_left = self.items_left();
            let footer_text = if items_left == 1 {
                "1 item left".to_string()
            } else {
                format!("{} items left", items_left)
            };
            let mut cx = DrawContext::new(&mut self.scene, scale);
            cx.paint_text(
                &footer_text,
                Point::new(container_x + 16.0, y + 20.0),
                14.0,
                Srgba::new(0.6, 0.6, 0.6, 1.0),
                &mut self.text_ctx,
            );
        }

        // Update hit testing
        if let Some(pos) = self.input_state.cursor_position {
            self.input_state.set_hovered(self.hit_tree.hit_test(pos));
        }
    }
}

// ============================================================================
// Event Handling
// ============================================================================

impl ApplicationHandler for TodoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attrs = Window::default_attributes()
            .with_title("TodoMVC - motif")
            .with_inner_size(winit::dpi::LogicalSize::new(600, 500));
        let window = event_loop.create_window(window_attrs).unwrap();
        let renderer = MetalRenderer::new();
        let surface = unsafe { MetalSurface::new(&window, renderer.device()) };

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.surface = Some(surface);
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        // When the blink timer fires, toggle the cursor and redraw.
        if let StartCause::ResumeTimeReached { .. } = cause {
            self.cursor_visible = !self.cursor_visible;
            self.cursor_blink_epoch = Instant::now();
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.new_todo_focused {
            const BLINK_INTERVAL: Duration = Duration::from_millis(530);
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                self.cursor_blink_epoch + BLINK_INTERVAL,
            ));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if self.surface.is_none() || self.renderer.is_none() {
                    return;
                }

                self.paint();

                if let (Some(surface), Some(renderer)) = (&mut self.surface, &mut self.renderer) {
                    renderer.render(&self.scene, surface);
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface {
                    surface.resize(size.width as f32, size.height as f32);
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
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = MouseButton::from_winit(button);
                if state == winit::event::ElementState::Pressed {
                    self.input_state.handle_mouse_button(btn, true);
                    self.input_state.begin_press();
                } else {
                    if let Some(clicked) = self.input_state.end_press() {
                        let id = clicked.0;

                        // New todo input clicked
                        if id == 1000 {
                            self.new_todo_focused = true;
                            // Reset blink so cursor is immediately visible after click
                            self.cursor_visible = true;
                            self.cursor_blink_epoch = Instant::now();

                            // Click-to-cursor
                            if let Some(click_pos) = self.input_state.cursor_position {
                                let container_x = self
                                    .window
                                    .as_ref()
                                    .map(|w| {
                                        let size = w.inner_size();
                                        let scale = w.scale_factor() as f32;
                                        (size.width as f32 / scale - 500.0) / 2.0
                                    })
                                    .unwrap_or(50.0);
                                let input_x = container_x + 16.0;
                                let text_x = click_pos.x - input_x - 8.0;

                                let scale = self
                                    .window
                                    .as_ref()
                                    .map(|w| w.scale_factor() as f32)
                                    .unwrap_or(1.0);
                                let layout = self
                                    .text_ctx
                                    .layout_text(self.new_todo_state.content(), 18.0 * scale);
                                let index = layout
                                    .index_for_x(text_x * scale, self.new_todo_state.content());
                                self.new_todo_state.move_to(index);
                            }
                        } else {
                            self.new_todo_focused = false;
                        }

                        // Checkbox clicked (2000 + todo_id)
                        if (2000..3000).contains(&id) {
                            let todo_id = (id - 2000) as usize;
                            self.toggle_todo(todo_id);
                        }

                        // Delete clicked (3000 + todo_id)
                        if (3000..4000).contains(&id) {
                            let todo_id = (id - 3000) as usize;
                            self.delete_todo(todo_id);
                        }
                    } else {
                        self.new_todo_focused = false;
                    }
                    self.input_state.handle_mouse_button(btn, false);
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
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

                if self.new_todo_focused && event.state == winit::event::ElementState::Pressed {
                    // Keep cursor visible during active typing
                    self.cursor_visible = true;
                    self.cursor_blink_epoch = Instant::now();

                    let modifiers = winit::event::Modifiers::from(self.input_state.modifiers);
                    match self
                        .new_todo_state
                        .handle_key_event(&event.logical_key, &modifiers)
                    {
                        HandleKeyResult::Handled => {}
                        HandleKeyResult::NotHandled => {}
                        HandleKeyResult::Blur
                        | HandleKeyResult::FocusNext
                        | HandleKeyResult::FocusPrev => {
                            self.new_todo_focused = false;
                        }
                        HandleKeyResult::Submit => {
                            let text = self.new_todo_state.content().to_string();
                            self.add_todo(text);
                        }
                        HandleKeyResult::Copy(_)
                        | HandleKeyResult::Cut(_)
                        | HandleKeyResult::Paste => {
                            // TODO: Clipboard
                        }
                    }

                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = TodoApp::new();
    event_loop.run_app(&mut app).unwrap();
}
