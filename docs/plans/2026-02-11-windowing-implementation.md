# Windowing Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add winit-based windowing to open a window and run an event loop.

**Architecture:** Use winit directly for now (no wrapper). Create example that opens window and handles events.

**Tech Stack:** Rust, winit 0.30

---

## Task 1: Add winit dependency

**Files:**
- Modify: `crates/gesso_core/Cargo.toml`

**Step 1: Add winit**

```toml
[dependencies]
glam = "0.32"
glamour = "0.18"
palette = "0.7"
winit = "0.30"
```

**Step 2: Verify it compiles**

Run: `cargo check -p gesso_core`

**Step 3: Commit**

```bash
git add crates/gesso_core/Cargo.toml
git commit -m "Add winit dependency for windowing"
```

---

## Task 2: Create hello window example

**Files:**
- Create: `crates/gesso/examples/hello_window.rs`
- Modify: `crates/gesso/Cargo.toml` (if needed for example)

**Step 1: Create example**

Create `crates/gesso/examples/hello_window.rs`:

```rust
//! Opens a window and runs until closed.

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

#[derive(Default)]
struct App {
    window: Option<Window>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("Gesso - Hello Window")
                .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
            self.window = Some(event_loop.create_window(attrs).unwrap());
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // TODO: render here
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
```

**Step 2: Run the example**

Run: `cargo run -p gesso --example hello_window`
Expected: Window opens with title "Gesso - Hello Window", closes when X clicked

**Step 3: Commit**

```bash
git add crates/gesso/examples/hello_window.rs
git commit -m "Add hello_window example"
```

---

## Task 3: Update llms.txt and spool

**Files:**
- Modify: `llms.txt`

**Step 1: Update llms.txt**

Add to Key Files:
```markdown
- [hello_window.rs](crates/gesso/examples/hello_window.rs) - Basic window example
```

**Step 2: Commit**

```bash
git add llms.txt .spool
git commit -m "Update llms.txt with hello_window example"
```

---

## Summary

After completing:
- winit dependency added
- hello_window example that opens a window and handles close
- Foundation for attaching metal renderer next
