# Hit Testing Infrastructure Design

Full integration test harness with real Metal rendering for hit testing.

## Goals

- Hit test: given a point, find which element was clicked
- Track element bounds during paint with z-order
- Full integration test harness with real windows and rendering
- No mocks — real element trees, real paint, real Metal

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    TestHarness                          │
├─────────────────────────────────────────────────────────┤
│  - window: Window (real, hidden)                        │
│  - renderer: MetalRenderer                              │
│  - surface: MetalSurface                                │
│  - scene: Scene                                         │
│  - text_ctx: TextContext                                │
│  - debug_server: DebugServer                            │
│  - hit_tree: HitTree (bounds collection)                │
├─────────────────────────────────────────────────────────┤
│  new(width, height) -> TestHarness                      │
│  render<F: FnOnce(&mut TestRenderContext)>(f)           │
│  hit_test(point: Point) -> Option<ElementId>            │
│  hit_test_all(point: Point) -> Vec<ElementId>           │
│  screenshot(path) -> Result<()>                         │
│  assert_hit(point, expected_id)                         │
│  assert_no_hit(point)                                   │
└─────────────────────────────────────────────────────────┘
```

## Core Types

### ElementId

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementId(pub u64);

impl ElementId {
    pub fn next(counter: &mut u64) -> Self {
        let id = *counter;
        *counter += 1;
        Self(id)
    }
}
```

### HitTree

```rust
#[derive(Debug, Clone)]
pub struct HitEntry {
    pub id: ElementId,
    pub bounds: Rect,
    pub z_index: u32,
}

#[derive(Debug, Default)]
pub struct HitTree {
    entries: Vec<HitEntry>,
    next_z: u32,
}

impl HitTree {
    pub fn push(&mut self, id: ElementId, bounds: Rect);
    pub fn hit_test(&self, point: Point) -> Option<ElementId>;
    pub fn hit_test_all(&self, point: Point) -> Vec<ElementId>;
    pub fn clear(&mut self);
}
```

### TestHarness

```rust
pub struct TestHarness {
    window: Window,
    renderer: MetalRenderer,
    surface: MetalSurface,
    scene: Scene,
    text_ctx: TextContext,
    hit_tree: HitTree,
    debug_server: DebugServer,
    scale_factor: ScaleFactor,
    next_element_id: u64,
}

impl TestHarness {
    pub fn new(width: u32, height: u32) -> Self;
    pub fn element_id(&mut self) -> ElementId;
    pub fn render(&mut self, f: impl FnOnce(&mut TestRenderContext));
    pub fn hit_test(&self, point: Point) -> Option<ElementId>;
    pub fn hit_test_all(&self, point: Point) -> Vec<ElementId>;
    pub fn screenshot(&self, path: &str) -> Result<(), Error>;
}
```

### TestRenderContext

```rust
pub struct TestRenderContext<'a> {
    pub scene: &'a mut Scene,
    pub text_ctx: &'a mut TextContext,
    pub hit_tree: &'a mut HitTree,
    pub scale_factor: ScaleFactor,
}

impl TestRenderContext<'_> {
    pub fn register_hit(&mut self, id: ElementId, bounds: Rect);
    pub fn paint_div(&mut self, bounds: Rect, color: Srgba);
}
```

## Crate Structure

```
crates/motif_test/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── harness.rs      # TestHarness struct
│   ├── hit_tree.rs     # HitTree and ElementId
│   ├── assertions.rs   # assert_hit, assert_bounds, etc.
│   └── fixtures.rs     # common test element trees
```

## Test Examples

### Basic hit test

```rust
#[test]
fn hit_test_single_div() {
    let mut harness = TestHarness::new(800, 600);
    let button_id = harness.element_id();

    harness.render(|cx| {
        let bounds = Rect::new(Point::new(100.0, 100.0), Size::new(200.0, 50.0));
        cx.paint_div(bounds, Srgba::new(0.2, 0.4, 0.8, 1.0));
        cx.register_hit(button_id, bounds);
    });

    assert_eq!(harness.hit_test(Point::new(150.0, 125.0)), Some(button_id));
    assert_eq!(harness.hit_test(Point::new(50.0, 50.0)), None);
}
```

### Overlapping elements (z-order)

```rust
#[test]
fn hit_test_overlapping_returns_topmost() {
    let mut harness = TestHarness::new(800, 600);
    let back_id = harness.element_id();
    let front_id = harness.element_id();

    harness.render(|cx| {
        let back = Rect::new(Point::new(100.0, 100.0), Size::new(200.0, 200.0));
        cx.paint_div(back, Srgba::new(1.0, 0.0, 0.0, 1.0));
        cx.register_hit(back_id, back);

        let front = Rect::new(Point::new(150.0, 150.0), Size::new(100.0, 100.0));
        cx.paint_div(front, Srgba::new(0.0, 1.0, 0.0, 1.0));
        cx.register_hit(front_id, front);
    });

    assert_eq!(harness.hit_test(Point::new(175.0, 175.0)), Some(front_id));
    assert_eq!(harness.hit_test(Point::new(110.0, 110.0)), Some(back_id));
}
```

### Nested elements

```rust
#[test]
fn hit_test_nested_elements() {
    let mut harness = TestHarness::new(800, 600);
    let parent_id = harness.element_id();
    let child_id = harness.element_id();

    harness.render(|cx| {
        let parent = Rect::new(Point::new(50.0, 50.0), Size::new(300.0, 200.0));
        cx.paint_div(parent, Srgba::new(0.2, 0.2, 0.2, 1.0));
        cx.register_hit(parent_id, parent);

        let child = Rect::new(Point::new(100.0, 100.0), Size::new(100.0, 50.0));
        cx.paint_div(child, Srgba::new(0.8, 0.2, 0.2, 1.0));
        cx.register_hit(child_id, child);
    });

    let hits = harness.hit_test_all(Point::new(125.0, 115.0));
    assert_eq!(hits, vec![child_id, parent_id]);
}
```

## Implementation Sequence

1. **Create motif_test crate** — Cargo.toml, basic structure
2. **Implement HitTree and ElementId** — with unit tests
3. **Implement TestHarness** — hidden window, Metal setup
4. **Add TestRenderContext** — paint helpers, hit registration
5. **Integration tests** — basic, overlapping, nested
6. **Assertions and fixtures** — ergonomic test helpers

## Spool Tasks

- `mmax58mh-ho0r` — Create motif_test crate with TestHarness
- `mmax5985-tqmr` — Implement HitTree and ElementId
- `mmax59ud-loue` — Integrate HitTree with PaintContext
- `mmax5aht-vtgg` — Test assertions and fixtures
