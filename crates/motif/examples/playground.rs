//! Visual playground for debugging and testing motif features.
//!
//! A screen-filling canvas with sections for typography, rendering,
//! elements, and anything else that needs visual verification.
//!
//! Run with: cargo run --example playground

use motif_core::{
    checkbox, div,
    element::{Element, LayoutContext, PaintContext},
    focus::{FocusEvent, FocusHandle, FocusState},
    input::{InputState, MouseButton, ScrollDelta, TextEditState},
    metal::{MetalRenderer, MetalSurface},
    text, text_input, ArcStr, DrawContext, ElementId, HitTree, IntoElement, LayoutEngine,
    ParentElement, Point, Rect, Render, RenderOnce, Renderer, ScaleFactor, Scene, Size, Srgba,
    TextContext, ViewContext, WindowContext,
};
use motif_debug::{DebugServer, InputStateSnapshot, SceneSnapshot};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

// ============================================================================
// Sections
// ============================================================================

// --- Typography Section ---

fn paint_typography_section(cx: &mut DrawContext, text_ctx: &mut TextContext, x: f32, y: f32) {
    // Text metrics visualization
    let sample = "Hxpgq";
    let font_size = 120.0;
    let baseline_y = y + 160.0;

    let layout = text_ctx.layout_text(sample, font_size);
    let text_width = layout.width();

    if let Some(fm) = layout.font_metrics() {
        let thickness = 2.0;
        let w = text_width + 40.0;

        // Line height background
        let box_top = baseline_y - fm.ascent;
        let box_height = fm.ascent + fm.descent;
        cx.paint_quad(
            Rect::new(Point::new(x, box_top), Size::new(w, box_height)),
            Srgba::new(0.1, 0.1, 0.14, 1.0),
        );

        // Metric lines
        let lines: &[(f32, Srgba, &str)] = &[
            (
                baseline_y - fm.ascent,
                Srgba::new(0.3, 0.9, 0.3, 0.6),
                "ascent",
            ),
            (
                baseline_y - fm.cap_height,
                Srgba::new(1.0, 0.6, 0.2, 0.6),
                "cap",
            ),
            (
                baseline_y - fm.x_height,
                Srgba::new(0.3, 0.9, 1.0, 0.6),
                "x-height",
            ),
            (baseline_y, Srgba::new(1.0, 1.0, 1.0, 0.4), "baseline"),
            (
                baseline_y + fm.descent,
                Srgba::new(1.0, 0.3, 0.3, 0.6),
                "descent",
            ),
        ];

        for &(line_y, color, label) in lines {
            cx.paint_quad(
                Rect::new(Point::new(x, line_y), Size::new(w, thickness)),
                color,
            );
            // Label
            cx.paint_text(
                label,
                Point::new(x + w + 8.0, line_y + 4.0),
                9.0,
                color,
                text_ctx,
            );
        }
    }

    // Render the sample text
    cx.paint_text(
        sample,
        Point::new(x, baseline_y),
        font_size,
        Srgba::new(1.0, 1.0, 1.0, 1.0),
        text_ctx,
    );

    // Size samples
    let sizes = [48.0, 32.0, 24.0, 18.0, 14.0, 11.0];
    let mut sample_y = baseline_y + 80.0;
    for size in sizes {
        cx.paint_text(
            &format!(
                "{}px — The quick brown fox jumps over the lazy dog",
                size as i32
            ),
            Point::new(x, sample_y),
            size,
            Srgba::new(0.85, 0.85, 0.85, 1.0),
            text_ctx,
        );
        sample_y += size + 12.0;
    }
}

// --- Quad Rendering Section ---

fn paint_quad_section(cx: &mut DrawContext, x: f32, y: f32) {
    let size = 60.0;
    let gap = 16.0;

    // Row 1: Solid colors
    let colors = [
        Srgba::new(1.0, 0.3, 0.3, 1.0),
        Srgba::new(0.3, 1.0, 0.3, 1.0),
        Srgba::new(0.3, 0.3, 1.0, 1.0),
        Srgba::new(1.0, 1.0, 0.3, 1.0),
        Srgba::new(1.0, 0.3, 1.0, 1.0),
        Srgba::new(0.3, 1.0, 1.0, 1.0),
    ];

    for (i, color) in colors.iter().enumerate() {
        cx.paint_quad(
            Rect::new(
                Point::new(x + i as f32 * (size + gap), y),
                Size::new(size, size),
            ),
            *color,
        );
    }

    // Row 2: Borders and rounded corners
    let mut quad = motif_core::Quad::new(
        motif_core::DeviceRect::new(
            motif_core::DevicePoint::new(x * 2.0, (y + size + gap) * 2.0),
            motif_core::DeviceSize::new(size * 2.0, size * 2.0),
        ),
        Srgba::new(0.15, 0.15, 0.25, 1.0),
    );
    quad.border_color = Srgba::new(0.5, 0.7, 1.0, 1.0);
    quad.border_widths = motif_core::Edges::all(2.0);
    quad.corner_radii = motif_core::Corners::all(12.0);
    cx.paint(quad);

    let mut quad2 = motif_core::Quad::new(
        motif_core::DeviceRect::new(
            motif_core::DevicePoint::new((x + size + gap) * 2.0, (y + size + gap) * 2.0),
            motif_core::DeviceSize::new(size * 2.0, size * 2.0),
        ),
        Srgba::new(0.2, 0.15, 0.25, 1.0),
    );
    quad2.border_color = Srgba::new(1.0, 0.5, 0.7, 1.0);
    quad2.border_widths = motif_core::Edges::all(3.0);
    quad2.corner_radii = motif_core::Corners::all(30.0);
    cx.paint(quad2);

    let mut quad3 = motif_core::Quad::new(
        motif_core::DeviceRect::new(
            motif_core::DevicePoint::new((x + 2.0 * (size + gap)) * 2.0, (y + size + gap) * 2.0),
            motif_core::DeviceSize::new(size * 2.0, size * 2.0),
        ),
        Srgba::new(0.25, 0.2, 0.15, 1.0),
    );
    quad3.border_color = Srgba::new(1.0, 0.8, 0.3, 1.0);
    quad3.border_widths = motif_core::Edges::all(1.0);
    cx.paint(quad3);
}

// --- Element System Section ---

struct ElementDemo {
    frame: u32,
}

impl Render for ElementDemo {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        self.frame += 1;

        div()
            .size(Size::new(280.0, 80.0))
            .flex_col()
            .padding(16.0)
            .gap(8.0)
            .background(Srgba::new(0.12, 0.14, 0.2, 1.0))
            .corner_radius(8.0)
            .border_color(Srgba::new(0.3, 0.5, 0.8, 1.0))
            .border_width(1.0)
            .child(
                text(format!("Render view — frame {}", self.frame))
                    .font_size(14.0)
                    .color(Srgba::new(0.8, 0.9, 1.0, 1.0)),
            )
            .child(
                text("Stateful: owns data, &mut self")
                    .font_size(11.0)
                    .color(Srgba::new(0.5, 0.6, 0.7, 1.0)),
            )
    }
}

struct StatusCard {
    label: ArcStr,
    value: ArcStr,
    color: Srgba,
}

impl RenderOnce for StatusCard {
    fn render(self, _cx: &mut WindowContext) -> impl IntoElement {
        div()
            .size(Size::new(130.0, 56.0))
            .flex_col()
            .padding(12.0)
            .background(Srgba::new(0.12, 0.14, 0.2, 1.0))
            .corner_radius(6.0)
            .child(text(self.value).font_size(18.0).color(self.color))
            .child(
                text(self.label)
                    .font_size(9.0)
                    .color(Srgba::new(0.45, 0.45, 0.5, 1.0)),
            )
    }
}

// ============================================================================
// App
// ============================================================================

struct App {
    window: Option<Window>,
    renderer: Option<MetalRenderer>,
    surface: Option<MetalSurface>,
    scene: Scene,
    text_ctx: TextContext,
    hit_tree: HitTree,
    layout_engine: LayoutEngine,
    element_demo: ElementDemo,
    debug_server: Option<DebugServer>,
    input_state: InputState,
    focus_state: FocusState,
    /// Focus handles for demo input fields
    input_handles: [FocusHandle; 3],
    /// Click counter for demo
    click_count: u32,
    // --- Controls demo state ---
    checkbox_states: [bool; 3],
    text_edit_state: TextEditState,
    text_input_focused: bool,
}

impl Default for App {
    fn default() -> Self {
        let debug_server = DebugServer::new().ok();
        Self {
            window: None,
            renderer: None,
            surface: None,
            scene: Scene::new(),
            text_ctx: TextContext::new(),
            hit_tree: HitTree::new(),
            layout_engine: LayoutEngine::new(),
            element_demo: ElementDemo { frame: 0 },
            debug_server,
            input_state: InputState::new(),
            focus_state: FocusState::new(),
            input_handles: [FocusHandle::new(), FocusHandle::new(), FocusHandle::new()],
            click_count: 0,
            checkbox_states: [true, false, false],
            text_edit_state: {
                let mut state = TextEditState::new();
                state.set_content("Hello, Motif!");
                state.move_to(13); // cursor at end
                state
            },
            text_input_focused: false,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("Motif — Playground")
                .with_inner_size(winit::dpi::LogicalSize::new(1200.0, 900.0));
            let window = event_loop.create_window(attrs).unwrap();

            let renderer = MetalRenderer::new();
            let surface = unsafe { MetalSurface::new(&window, renderer.device()) };

            // Pass the window ID to the debug server for native screenshots
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
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface {
                    surface.resize(size.width as f32, size.height as f32);
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(surface), Some(window)) =
                    (&mut self.renderer, &mut self.surface, &self.window)
                {
                    self.scene.clear();
                    self.hit_tree.clear();

                    let scale = ScaleFactor(window.scale_factor() as f32);

                    // --- Section: Typography ---
                    {
                        let mut cx = DrawContext::new(&mut self.scene, scale);
                        paint_section_label(&mut cx, &mut self.text_ctx, "TYPOGRAPHY", 30.0, 20.0);
                        paint_typography_section(&mut cx, &mut self.text_ctx, 30.0, 30.0);
                    }

                    // --- Section: Quad Rendering ---
                    {
                        let mut cx = DrawContext::new(&mut self.scene, scale);
                        paint_section_label(&mut cx, &mut self.text_ctx, "QUADS", 30.0, 530.0);
                        paint_quad_section(&mut cx, 30.0, 545.0);
                    }

                    // --- Section: Element System ---
                    {
                        let mut cx = DrawContext::new(&mut self.scene, scale);
                        paint_section_label(&mut cx, &mut self.text_ctx, "ELEMENTS", 500.0, 20.0);
                    }

                    // Render stateful view (manually positioned at 500, 30)
                    {
                        let wcx = WindowContext::new(&mut self.scene, &mut self.text_ctx, scale);
                        let mut el = self
                            .element_demo
                            .render(&mut ViewContext::new(wcx))
                            .into_element();

                        // Layout phase
                        let mut layout_cx =
                            LayoutContext::new(&mut self.layout_engine, &mut self.text_ctx, scale);
                        let node_id = el.request_layout(&mut layout_cx);
                        self.layout_engine.compute_layout(
                            node_id,
                            800.0,
                            600.0,
                            &mut self.text_ctx,
                        );

                        // Paint at desired position with offset for children
                        let layout_bounds = self.layout_engine.layout_bounds(node_id);
                        let desired_pos = Point::new(500.0, 30.0);
                        let offset = Point::new(
                            desired_pos.x - layout_bounds.origin.x,
                            desired_pos.y - layout_bounds.origin.y,
                        );

                        let mut pcx = PaintContext::new(
                            &mut self.scene,
                            &mut self.text_ctx,
                            &mut self.hit_tree,
                            &self.layout_engine,
                            scale,
                        );
                        pcx.set_offset(offset);

                        let paint_bounds = Rect::new(desired_pos, layout_bounds.size);
                        el.paint(paint_bounds, &mut pcx);
                    }

                    // Render stateless cards (manually positioned)
                    {
                        let quad_count = self.scene.quad_count();
                        let text_count = self.scene.text_run_count();
                        let cards = vec![
                            (
                                StatusCard {
                                    label: "Quads".into(),
                                    value: ArcStr::from(format!("{}", quad_count)),
                                    color: Srgba::new(0.4, 0.9, 0.6, 1.0),
                                },
                                Point::new(500.0, 120.0),
                            ),
                            (
                                StatusCard {
                                    label: "Text runs".into(),
                                    value: ArcStr::from(format!("{}", text_count)),
                                    color: Srgba::new(0.6, 0.7, 1.0, 1.0),
                                },
                                Point::new(646.0, 120.0),
                            ),
                        ];

                        for (card, desired_pos) in cards {
                            let mut wcx =
                                WindowContext::new(&mut self.scene, &mut self.text_ctx, scale);
                            let mut el = card.render(&mut wcx).into_element();

                            // Layout phase
                            let mut layout_cx = LayoutContext::new(
                                &mut self.layout_engine,
                                &mut self.text_ctx,
                                scale,
                            );
                            let node_id = el.request_layout(&mut layout_cx);
                            self.layout_engine.compute_layout(
                                node_id,
                                800.0,
                                600.0,
                                &mut self.text_ctx,
                            );

                            // Paint at desired position with offset for children
                            let layout_bounds = self.layout_engine.layout_bounds(node_id);
                            let offset = Point::new(
                                desired_pos.x - layout_bounds.origin.x,
                                desired_pos.y - layout_bounds.origin.y,
                            );

                            let mut pcx = PaintContext::new(
                                &mut self.scene,
                                &mut self.text_ctx,
                                &mut self.hit_tree,
                                &self.layout_engine,
                                scale,
                            );
                            pcx.set_offset(offset);

                            let paint_bounds = Rect::new(desired_pos, layout_bounds.size);
                            el.paint(paint_bounds, &mut pcx);
                        }
                    }

                    // --- Section: Interactive Button ---
                    {
                        let mut cx = DrawContext::new(&mut self.scene, scale);
                        paint_section_label(
                            &mut cx,
                            &mut self.text_ctx,
                            "INTERACTIONS",
                            500.0,
                            200.0,
                        );

                        // Paint an interactive button
                        let button_id = ElementId(1000); // Fixed ID for the demo button
                        let button_bounds =
                            Rect::new(Point::new(500.0, 220.0), Size::new(180.0, 50.0));

                        // Determine button visual state
                        let is_hovered = self.input_state.hovered() == Some(button_id);
                        let is_pressed = self.input_state.pressed() == Some(button_id);

                        let button_color = if is_pressed {
                            Srgba::new(0.2, 0.5, 0.9, 1.0) // Pressed: darker blue
                        } else if is_hovered {
                            Srgba::new(0.4, 0.7, 1.0, 1.0) // Hover: lighter blue
                        } else {
                            Srgba::new(0.3, 0.6, 0.95, 1.0) // Normal: blue
                        };

                        cx.paint_quad(button_bounds, button_color);
                        self.hit_tree.push(button_id, button_bounds);

                        // Button label
                        let label = format!("Clicks: {}", self.click_count);
                        cx.paint_text(
                            &label,
                            Point::new(
                                button_bounds.origin.x + 20.0,
                                button_bounds.origin.y + 16.0,
                            ),
                            18.0,
                            Srgba::new(1.0, 1.0, 1.0, 1.0),
                            &mut self.text_ctx,
                        );
                    }

                    // --- Section: Focus Demo ---
                    {
                        let mut cx = DrawContext::new(&mut self.scene, scale);
                        paint_section_label(&mut cx, &mut self.text_ctx, "FOCUS", 500.0, 290.0);

                        // Three focusable "input" boxes
                        let labels = ["Input 1", "Input 2", "Input 3"];
                        for (i, (handle, label)) in
                            self.input_handles.iter().zip(labels.iter()).enumerate()
                        {
                            let y = 310.0 + i as f32 * 50.0;
                            let bounds = Rect::new(Point::new(500.0, y), Size::new(280.0, 40.0));
                            let element_id = ElementId(2000 + i as u64);

                            let is_focused = handle.is_focused(&self.focus_state);
                            let is_hovered = self.input_state.hovered() == Some(element_id);

                            // Background
                            let bg_color = if is_focused {
                                Srgba::new(0.15, 0.18, 0.25, 1.0)
                            } else {
                                Srgba::new(0.1, 0.1, 0.14, 1.0)
                            };

                            // Border
                            let border_color = if is_focused {
                                Srgba::new(0.4, 0.7, 1.0, 1.0) // Blue when focused
                            } else if is_hovered {
                                Srgba::new(0.3, 0.4, 0.5, 1.0) // Subtle on hover
                            } else {
                                Srgba::new(0.2, 0.2, 0.25, 1.0) // Dim border
                            };

                            let mut quad = motif_core::Quad::new(
                                motif_core::DeviceRect::new(
                                    motif_core::DevicePoint::new(
                                        bounds.origin.x * scale.0,
                                        bounds.origin.y * scale.0,
                                    ),
                                    motif_core::DeviceSize::new(
                                        bounds.size.width * scale.0,
                                        bounds.size.height * scale.0,
                                    ),
                                ),
                                bg_color,
                            );
                            quad.border_color = border_color;
                            quad.border_widths =
                                motif_core::Edges::all(if is_focused { 2.0 } else { 1.0 });
                            quad.corner_radii = motif_core::Corners::all(4.0);
                            cx.paint(quad);

                            // Register for hit testing
                            self.hit_tree.push(element_id, bounds);

                            // Label text
                            let text_color = if is_focused {
                                Srgba::new(0.9, 0.9, 0.95, 1.0)
                            } else {
                                Srgba::new(0.5, 0.5, 0.55, 1.0)
                            };
                            cx.paint_text(
                                label,
                                Point::new(bounds.origin.x + 12.0, bounds.origin.y + 14.0),
                                14.0,
                                text_color,
                                &mut self.text_ctx,
                            );

                            // Show focus indicator
                            if is_focused {
                                cx.paint_text(
                                    "(focused)",
                                    Point::new(bounds.origin.x + 200.0, bounds.origin.y + 14.0),
                                    11.0,
                                    Srgba::new(0.4, 0.7, 1.0, 0.8),
                                    &mut self.text_ctx,
                                );
                            }
                        }

                        // Instructions
                        cx.paint_text(
                            "Click to focus • Tab to cycle (future)",
                            Point::new(500.0, 470.0),
                            10.0,
                            Srgba::new(0.4, 0.4, 0.45, 1.0),
                            &mut self.text_ctx,
                        );
                    }

                    // --- Section: Controls (Checkbox, TextInput) ---
                    {
                        let mut cx = DrawContext::new(&mut self.scene, scale);
                        paint_section_label(&mut cx, &mut self.text_ctx, "CONTROLS", 500.0, 500.0);
                    }

                    // Checkboxes
                    {
                        let labels = ["Enable feature", "Dark mode", "Notifications"];
                        for (i, label) in labels.iter().enumerate() {
                            let checkbox_id = ElementId(3000 + i as u64);
                            let y = 520.0 + i as f32 * 32.0;
                            let is_hovered = self.input_state.hovered() == Some(checkbox_id);

                            let mut cb = checkbox(checkbox_id)
                                .checked(self.checkbox_states[i])
                                .hovered(is_hovered);

                            // Layout phase
                            let mut layout_cx = LayoutContext::new(
                                &mut self.layout_engine,
                                &mut self.text_ctx,
                                scale,
                            );
                            let node_id = cb.request_layout(&mut layout_cx);
                            self.layout_engine.compute_layout(
                                node_id,
                                800.0,
                                600.0,
                                &mut self.text_ctx,
                            );

                            // Paint at desired position (offset bounds)
                            let mut bounds = self.layout_engine.layout_bounds(node_id);
                            bounds.origin = Point::new(500.0, y);

                            let mut pcx = PaintContext::new(
                                &mut self.scene,
                                &mut self.text_ctx,
                                &mut self.hit_tree,
                                &self.layout_engine,
                                scale,
                            );
                            cb.paint(bounds, &mut pcx);

                            // Label next to checkbox
                            let mut cx = DrawContext::new(&mut self.scene, scale);
                            cx.paint_text(
                                label,
                                Point::new(526.0, y + 4.0),
                                13.0,
                                Srgba::new(0.8, 0.8, 0.85, 1.0),
                                &mut self.text_ctx,
                            );
                        }
                    }

                    // Text input
                    {
                        let input_id = ElementId(3100);
                        let is_hovered = self.input_state.hovered() == Some(input_id);
                        let input_bounds =
                            Rect::new(Point::new(500.0, 620.0), Size::new(280.0, 36.0));

                        let mut input = text_input(self.text_edit_state.content(), input_id)
                            .placeholder("Type something...")
                            .bounds(input_bounds)
                            .focused(self.text_input_focused)
                            .cursor_pos(self.text_edit_state.cursor_offset())
                            .selection(self.text_edit_state.selected_range().clone());

                        // Add hover effect via border color
                        if is_hovered && !self.text_input_focused {
                            input = input.border_color(Srgba::new(0.5, 0.5, 0.55, 1.0));
                        }

                        // Layout phase
                        let mut layout_cx =
                            LayoutContext::new(&mut self.layout_engine, &mut self.text_ctx, scale);
                        let node_id = input.request_layout(&mut layout_cx);
                        self.layout_engine.compute_layout(
                            node_id,
                            800.0,
                            600.0,
                            &mut self.text_ctx,
                        );

                        // Paint at desired position
                        let mut pcx = PaintContext::new(
                            &mut self.scene,
                            &mut self.text_ctx,
                            &mut self.hit_tree,
                            &self.layout_engine,
                            scale,
                        );
                        input.paint(input_bounds, &mut pcx);

                        // Label
                        let mut cx = DrawContext::new(&mut self.scene, scale);
                        cx.paint_text(
                            "TextInput (using TextEditState):",
                            Point::new(500.0, 605.0),
                            10.0,
                            Srgba::new(0.5, 0.5, 0.55, 1.0),
                            &mut self.text_ctx,
                        );
                    }

                    // --- Debug overlays ---
                    // Paint any debug overlay quads on top of the scene.
                    if let Some(ref debug_server) = self.debug_server {
                        let mut cx = DrawContext::new(&mut self.scene, scale);
                        for overlay in debug_server.overlays() {
                            let mut quad = motif_core::Quad::new(
                                motif_core::DeviceRect::new(
                                    motif_core::DevicePoint::new(
                                        overlay.x * scale.0,
                                        overlay.y * scale.0,
                                    ),
                                    motif_core::DeviceSize::new(
                                        overlay.w * scale.0,
                                        overlay.h * scale.0,
                                    ),
                                ),
                                Srgba::new(
                                    overlay.color.r,
                                    overlay.color.g,
                                    overlay.color.b,
                                    overlay.color.a,
                                ),
                            );
                            quad.border_color = Srgba::new(
                                overlay.border_color.r,
                                overlay.border_color.g,
                                overlay.border_color.b,
                                overlay.border_color.a,
                            );
                            quad.border_widths =
                                motif_core::Edges::all(overlay.border_width * scale.0);
                            quad.corner_radii =
                                motif_core::Corners::all(overlay.corner_radius * scale.0);
                            cx.paint(quad);
                        }
                    }

                    renderer.render(&self.scene, surface);

                    // Update the debug server with the current scene state.
                    if let Some(ref debug_server) = self.debug_server {
                        let phys = window.inner_size();
                        let viewport = (phys.width as f32, phys.height as f32);
                        let snapshot = SceneSnapshot::from_scene(&self.scene, viewport, scale.0);
                        debug_server.update_scene(snapshot);

                        // Update window position for input simulation
                        // Use inner_position (content area) not outer_position (includes title bar)
                        // Convert from physical pixels to logical for CGEvent
                        if let Ok(inner_pos) = window.inner_position() {
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
            // --- Input events ---
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor() as f32)
                    .unwrap_or(1.0);
                self.input_state
                    .handle_cursor_moved(position.x, position.y, scale);

                // Update hover state from hit tree
                if let Some(pos) = self.input_state.cursor_position {
                    let hovered = self.hit_tree.hit_test(pos);
                    self.input_state.set_hovered(hovered);
                }

                // Request redraw for hover feedback
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorEntered { .. } => {
                self.input_state.handle_cursor_entered();
            }
            WindowEvent::CursorLeft { .. } => {
                self.input_state.handle_cursor_left();
                self.input_state.set_hovered(None);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = MouseButton::from_winit(button);
                if state == winit::event::ElementState::Pressed {
                    // Raw input tracking
                    self.input_state.handle_mouse_button(btn, true);
                    // Interaction tracking: record press target
                    self.input_state.begin_press();
                } else {
                    // Interaction tracking: check for click
                    if let Some(clicked_element) = self.input_state.end_press() {
                        let id = clicked_element.0;

                        // Check if clicked on button
                        if clicked_element == ElementId(1000) {
                            self.click_count += 1;
                        }
                        // Check if clicked on focusable inputs (IDs 2000-2002)
                        if (2000..2003).contains(&id) {
                            let index = (id - 2000) as usize;
                            self.input_handles[index].focus(&mut self.focus_state);
                        }
                        // Check if clicked on checkboxes (IDs 3000-3002)
                        if (3000..3003).contains(&id) {
                            let index = (id - 3000) as usize;
                            self.checkbox_states[index] = !self.checkbox_states[index];
                        }
                        // Check if clicked on text input (ID 3100)
                        if id == 3100 {
                            self.text_input_focused = true;

                            // Click-to-cursor: convert click position to byte offset
                            if let Some(click_pos) = self.input_state.cursor_position {
                                // Text input bounds (must match rendering)
                                let input_bounds =
                                    Rect::new(Point::new(500.0, 620.0), Size::new(280.0, 36.0));
                                let padding = 8.0;
                                let font_size = 14.0;

                                // Calculate x position relative to text start
                                let text_x = click_pos.x - input_bounds.origin.x - padding;

                                // Get scale factor
                                let scale = self
                                    .window
                                    .as_ref()
                                    .map(|w| w.scale_factor() as f32)
                                    .unwrap_or(1.0);

                                // Layout the text to get index for position
                                let layout = self
                                    .text_ctx
                                    .layout_text(self.text_edit_state.content(), font_size * scale);

                                // Convert x to scaled coordinates and find index
                                let index = layout
                                    .index_for_x(text_x * scale, self.text_edit_state.content());

                                // Move cursor to clicked position
                                self.text_edit_state.move_to(index);
                            }
                        } else {
                            self.text_input_focused = false;
                        }
                    } else {
                        // Clicked outside any element - blur focus
                        self.focus_state.blur();
                        self.text_input_focused = false;
                    }
                    // Raw input tracking
                    self.input_state.handle_mouse_button(btn, false);
                }

                // Process focus events (for logging/debugging)
                for event in self.focus_state.take_events() {
                    match event {
                        FocusEvent::Focus { id } => {
                            eprintln!("Focus: {:?}", id);
                        }
                        FocusEvent::Blur { id } => {
                            eprintln!("Blur: {:?}", id);
                        }
                    }
                }

                // Request redraw for press feedback
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

                // Handle text input when focused
                if self.text_input_focused && event.state == winit::event::ElementState::Pressed {
                    use motif_core::input::HandleKeyResult;

                    let modifiers = winit::event::Modifiers::from(self.input_state.modifiers);
                    match self
                        .text_edit_state
                        .handle_key_event(&event.logical_key, &modifiers)
                    {
                        HandleKeyResult::Handled => {}
                        HandleKeyResult::NotHandled => {}
                        HandleKeyResult::Cancel => {
                            // Escape: discard in-progress edit and blur.
                            self.text_input_focused = false;
                        }
                        HandleKeyResult::Blur => {}
                        HandleKeyResult::Copy(_text) => {
                            // TODO: Copy to system clipboard
                        }
                        HandleKeyResult::Cut(_text) => {
                            // TODO: Copy to system clipboard (text already removed)
                        }
                        HandleKeyResult::Paste => {
                            // TODO: Read from system clipboard and call paste()
                        }
                        HandleKeyResult::Submit => {
                            // In a real app: submit form, add todo item, etc.
                            eprintln!("Submit: '{}'", self.text_edit_state.content());
                        }
                        HandleKeyResult::FocusNext => {
                            // In a real app: move focus to next input
                            eprintln!("Focus next (Tab)");
                            self.text_input_focused = false;
                        }
                        HandleKeyResult::FocusPrev => {
                            // In a real app: move focus to previous input
                            eprintln!("Focus prev (Shift+Tab)");
                            self.text_input_focused = false;
                        }
                    }

                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            _ => {}
        }

        // Update debug server with current input state after any event.
        if let Some(ref debug_server) = self.debug_server {
            let snapshot = InputStateSnapshot::from_input_state(&self.input_state);
            debug_server.update_input(snapshot);
        }
    }
}

fn paint_section_label(
    cx: &mut DrawContext,
    text_ctx: &mut TextContext,
    label: &str,
    x: f32,
    y: f32,
) {
    cx.paint_text(
        label,
        Point::new(x, y),
        10.0,
        Srgba::new(0.4, 0.4, 0.45, 1.0),
        text_ctx,
    );
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
