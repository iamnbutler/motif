//! Scene snapshot: a serializable capture of the current scene state.

use motif_core::input::{InputState, MouseButton};
use motif_core::{HitTree, Scene};
use serde::Serialize;

/// A debug overlay quad injected via the debug CLI.
///
/// These persist across frames until explicitly cleared.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OverlayQuad {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: ColorInfo,
    pub border_color: ColorInfo,
    pub border_width: f32,
    pub corner_radius: f32,
}

/// Serializable info about a single quad.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct QuadInfo {
    pub bounds: BoundsInfo,
    pub color: ColorInfo,
    pub border_color: ColorInfo,
    pub border_widths: EdgesInfo,
    pub corner_radii: CornersInfo,
    pub has_clip: bool,
    pub clip_bounds: Option<BoundsInfo>,
}

/// Serializable bounds (x, y, w, h).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoundsInfo {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Serializable RGBA color.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ColorInfo {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Serializable edge values (top, right, bottom, left).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EdgesInfo {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// Serializable corner values.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CornersInfo {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

/// Serializable summary of a single text run.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TextRunInfo {
    pub origin_x: f32,
    pub origin_y: f32,
    pub font_size: f32,
    pub glyph_count: usize,
    pub color: ColorInfo,
}

/// A serializable snapshot of the current input state.
#[derive(Debug, Clone, Serialize, Default)]
pub struct InputStateSnapshot {
    /// Cursor position in logical pixels, or null if outside window.
    pub cursor_position: Option<PointInfo>,
    /// List of currently pressed mouse buttons.
    pub mouse_buttons: Vec<String>,
    /// Current modifier key state.
    pub modifiers: ModifiersInfo,
    /// Element currently under the cursor (from hit testing).
    pub hovered_element: Option<u64>,
    /// Element where mouse button was pressed (for click detection).
    pub pressed_element: Option<u64>,
}

/// Serializable point (x, y).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PointInfo {
    pub x: f32,
    pub y: f32,
}

/// Serializable modifier key state.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct ModifiersInfo {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl InputStateSnapshot {
    /// Create a snapshot from InputState.
    pub fn from_input_state(state: &InputState) -> Self {
        let cursor_position = state.cursor_position.map(|p| PointInfo { x: p.x, y: p.y });

        let mouse_buttons: Vec<String> = state
            .mouse_buttons
            .iter()
            .map(|b| match b {
                MouseButton::Left => "left".to_string(),
                MouseButton::Right => "right".to_string(),
                MouseButton::Middle => "middle".to_string(),
                MouseButton::Back => "back".to_string(),
                MouseButton::Forward => "forward".to_string(),
                MouseButton::Other(n) => format!("other({})", n),
            })
            .collect();

        let modifiers = ModifiersInfo {
            shift: state.modifiers.shift_key(),
            control: state.modifiers.control_key(),
            alt: state.modifiers.alt_key(),
            super_key: state.modifiers.super_key(),
        };

        let hovered_element = state.hovered().map(|e| e.0);
        let pressed_element = state.pressed().map(|e| e.0);

        Self {
            cursor_position,
            mouse_buttons,
            modifiers,
            hovered_element,
            pressed_element,
        }
    }

    /// Return input state as a JSON value (for the `input.state` command).
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "cursor_position": self.cursor_position.as_ref().map(|p| {
                serde_json::json!({ "x": p.x, "y": p.y })
            }),
            "mouse_buttons": self.mouse_buttons,
            "modifiers": {
                "shift": self.modifiers.shift,
                "control": self.modifiers.control,
                "alt": self.modifiers.alt,
                "super": self.modifiers.super_key,
            },
            "hovered_element": self.hovered_element,
            "pressed_element": self.pressed_element,
        })
    }
}

/// A serializable snapshot of the current scene state.
#[derive(Debug, Clone, Serialize)]
pub struct SceneSnapshot {
    pub quads: Vec<QuadInfo>,
    pub text_runs: Vec<TextRunInfo>,
    pub text_run_count: usize,
    pub quad_count: usize,
    pub viewport_size: (f32, f32),
    pub scale_factor: f32,
}

impl SceneSnapshot {
    /// Create a snapshot from a scene and viewport metadata.
    pub fn from_scene(scene: &Scene, viewport_size: (f32, f32), scale_factor: f32) -> Self {
        let quads: Vec<QuadInfo> = scene
            .quads()
            .iter()
            .map(|q| {
                let bg = q.background;
                let bc = q.border_color;
                let bw = &q.border_widths;
                let cr = &q.corner_radii;

                QuadInfo {
                    bounds: BoundsInfo {
                        x: q.bounds.origin.x,
                        y: q.bounds.origin.y,
                        w: q.bounds.size.width,
                        h: q.bounds.size.height,
                    },
                    color: ColorInfo {
                        r: bg.red,
                        g: bg.green,
                        b: bg.blue,
                        a: bg.alpha,
                    },
                    border_color: ColorInfo {
                        r: bc.red,
                        g: bc.green,
                        b: bc.blue,
                        a: bc.alpha,
                    },
                    border_widths: EdgesInfo {
                        top: bw.top,
                        right: bw.right,
                        bottom: bw.bottom,
                        left: bw.left,
                    },
                    corner_radii: CornersInfo {
                        top_left: cr.top_left,
                        top_right: cr.top_right,
                        bottom_right: cr.bottom_right,
                        bottom_left: cr.bottom_left,
                    },
                    has_clip: q.clip_bounds.is_some(),
                    clip_bounds: q.clip_bounds.map(|cb| BoundsInfo {
                        x: cb.origin.x,
                        y: cb.origin.y,
                        w: cb.size.width,
                        h: cb.size.height,
                    }),
                }
            })
            .collect();

        let text_runs: Vec<TextRunInfo> = scene
            .text_runs()
            .iter()
            .map(|tr| {
                let c = tr.color;
                TextRunInfo {
                    origin_x: tr.origin.x,
                    origin_y: tr.origin.y,
                    font_size: tr.font_size,
                    glyph_count: tr.glyphs.len(),
                    color: ColorInfo {
                        r: c.red,
                        g: c.green,
                        b: c.blue,
                        a: c.alpha,
                    },
                }
            })
            .collect();

        Self {
            quad_count: quads.len(),
            quads,
            text_run_count: text_runs.len(),
            text_runs,
            viewport_size,
            scale_factor,
        }
    }

    /// Return scene stats as a JSON value (for the `scene.stats` command).
    pub fn stats(&self) -> serde_json::Value {
        serde_json::json!({
            "quad_count": self.quad_count,
            "text_run_count": self.text_run_count,
            "viewport_size": self.viewport_size,
            "scale_factor": self.scale_factor,
        })
    }

    /// Return quads as a JSON array (for the `scene.quads` command).
    pub fn quads_json(&self) -> serde_json::Value {
        let quads: Vec<serde_json::Value> = self
            .quads
            .iter()
            .map(|q| {
                serde_json::json!({
                    "bounds": {
                        "x": q.bounds.x,
                        "y": q.bounds.y,
                        "w": q.bounds.w,
                        "h": q.bounds.h,
                    },
                    "color": {
                        "r": q.color.r,
                        "g": q.color.g,
                        "b": q.color.b,
                        "a": q.color.a,
                    },
                    "border_color": {
                        "r": q.border_color.r,
                        "g": q.border_color.g,
                        "b": q.border_color.b,
                        "a": q.border_color.a,
                    },
                    "border_widths": {
                        "top": q.border_widths.top,
                        "right": q.border_widths.right,
                        "bottom": q.border_widths.bottom,
                        "left": q.border_widths.left,
                    },
                    "corner_radii": {
                        "top_left": q.corner_radii.top_left,
                        "top_right": q.corner_radii.top_right,
                        "bottom_right": q.corner_radii.bottom_right,
                        "bottom_left": q.corner_radii.bottom_left,
                    },
                    "has_clip": q.has_clip,
                    "clip_bounds": q.clip_bounds.as_ref().map(|cb| {
                        serde_json::json!({
                            "x": cb.x,
                            "y": cb.y,
                            "w": cb.w,
                            "h": cb.h,
                        })
                    }),
                })
            })
            .collect();
        serde_json::Value::Array(quads)
    }

    /// Return text runs as a JSON array (for the `scene.text_runs` command).
    pub fn text_runs_json(&self) -> serde_json::Value {
        let runs: Vec<serde_json::Value> = self
            .text_runs
            .iter()
            .map(|tr| {
                serde_json::json!({
                    "origin": {
                        "x": tr.origin_x,
                        "y": tr.origin_y,
                    },
                    "font_size": tr.font_size,
                    "glyph_count": tr.glyph_count,
                    "color": {
                        "r": tr.color.r,
                        "g": tr.color.g,
                        "b": tr.color.b,
                        "a": tr.color.a,
                    },
                })
            })
            .collect();
        serde_json::Value::Array(runs)
    }
}

/// Serializable info about a single hit-tree entry.
#[derive(Debug, Clone, Serialize)]
pub struct HitEntryInfo {
    pub id: u64,
    pub bounds: BoundsInfo,
    pub z_index: u32,
}

/// A serializable snapshot of the current hit tree.
#[derive(Debug, Clone, Serialize, Default)]
pub struct HitTreeSnapshot {
    pub entries: Vec<HitEntryInfo>,
}

impl HitTreeSnapshot {
    /// Create a snapshot from a HitTree.
    pub fn from_hit_tree(hit_tree: &HitTree) -> Self {
        let entries = hit_tree
            .entries()
            .iter()
            .map(|e| HitEntryInfo {
                id: e.id.0,
                bounds: BoundsInfo {
                    x: e.bounds.origin.x,
                    y: e.bounds.origin.y,
                    w: e.bounds.size.width,
                    h: e.bounds.size.height,
                },
                z_index: e.z_index,
            })
            .collect();
        Self { entries }
    }

    /// Return a flat JSON array of all elements sorted by z-order (for `element.list`).
    pub fn list_json(&self) -> serde_json::Value {
        let items: Vec<serde_json::Value> = self
            .entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "bounds": {
                        "x": e.bounds.x,
                        "y": e.bounds.y,
                        "w": e.bounds.w,
                        "h": e.bounds.h,
                    },
                    "z_index": e.z_index,
                })
            })
            .collect();
        serde_json::Value::Array(items)
    }

    /// Return a containment tree as a JSON array of root nodes (for `elements.tree`).
    ///
    /// Hierarchy is inferred spatially: an element is a child of the tightest
    /// enclosing ancestor that was painted before it (lower z_index). Elements
    /// that are not contained by any predecessor become roots.
    pub fn tree_json(&self) -> serde_json::Value {
        let n = self.entries.len();
        if n == 0 {
            return serde_json::Value::Array(vec![]);
        }

        // For each entry i, find its parent: the entry j < i (painted earlier)
        // whose bounds fully contain i's bounds, with the smallest area.
        let mut parent_idx: Vec<Option<usize>> = vec![None; n];
        for i in 0..n {
            let mut best: Option<usize> = None;
            let mut best_area = f32::MAX;
            for j in 0..i {
                if bounds_contains_fully(&self.entries[j].bounds, &self.entries[i].bounds) {
                    let area = self.entries[j].bounds.w * self.entries[j].bounds.h;
                    if area < best_area {
                        best_area = area;
                        best = Some(j);
                    }
                }
            }
            parent_idx[i] = best;
        }

        // Build children lists and collect root indices.
        let mut children: Vec<Vec<usize>> = vec![vec![]; n];
        let mut roots: Vec<usize> = vec![];
        for i in 0..n {
            match parent_idx[i] {
                Some(p) => children[p].push(i),
                None => roots.push(i),
            }
        }

        let roots_json: Vec<serde_json::Value> = roots
            .iter()
            .map(|&r| build_tree_node(r, &self.entries, &children))
            .collect();
        serde_json::Value::Array(roots_json)
    }
}

/// Returns true if `outer` fully contains `inner` (inclusive on all sides).
fn bounds_contains_fully(outer: &BoundsInfo, inner: &BoundsInfo) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.w <= outer.x + outer.w
        && inner.y + inner.h <= outer.y + outer.h
}

/// Recursively build a tree-node JSON value.
fn build_tree_node(
    i: usize,
    entries: &[HitEntryInfo],
    children: &[Vec<usize>],
) -> serde_json::Value {
    let e = &entries[i];
    let child_nodes: Vec<serde_json::Value> = children[i]
        .iter()
        .map(|&c| build_tree_node(c, entries, children))
        .collect();
    serde_json::json!({
        "id": e.id,
        "bounds": {
            "x": e.bounds.x,
            "y": e.bounds.y,
            "w": e.bounds.w,
            "h": e.bounds.h,
        },
        "z_index": e.z_index,
        "children": child_nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use linebender_resource_handle::Blob;
    use motif_core::input::ModifiersState;
    use motif_core::Point;
    use motif_core::{
        Corners, DevicePoint, DeviceRect, DeviceSize, Edges, FontData, Quad, Scene, Srgba, TextRun,
    };

    #[test]
    fn input_snapshot_from_empty_state() {
        let state = InputState::new();
        let snap = InputStateSnapshot::from_input_state(&state);

        assert!(snap.cursor_position.is_none());
        assert!(snap.mouse_buttons.is_empty());
        assert!(!snap.modifiers.shift);
        assert!(!snap.modifiers.control);
        assert!(!snap.modifiers.alt);
        assert!(!snap.modifiers.super_key);
        assert!(snap.hovered_element.is_none());
        assert!(snap.pressed_element.is_none());
    }

    #[test]
    fn input_snapshot_captures_cursor_position() {
        let mut state = InputState::new();
        state.cursor_position = Some(Point::new(123.5, 456.0));

        let snap = InputStateSnapshot::from_input_state(&state);

        let pos = snap.cursor_position.expect("should have position");
        assert_eq!(pos.x, 123.5);
        assert_eq!(pos.y, 456.0);
    }

    #[test]
    fn input_snapshot_captures_mouse_buttons() {
        let mut state = InputState::new();
        state.mouse_buttons.insert(MouseButton::Left);
        state.mouse_buttons.insert(MouseButton::Right);

        let snap = InputStateSnapshot::from_input_state(&state);

        assert_eq!(snap.mouse_buttons.len(), 2);
        assert!(snap.mouse_buttons.contains(&"left".to_string()));
        assert!(snap.mouse_buttons.contains(&"right".to_string()));
    }

    #[test]
    fn input_snapshot_captures_modifiers() {
        let mut state = InputState::new();
        state.modifiers = ModifiersState::SHIFT | ModifiersState::CONTROL;

        let snap = InputStateSnapshot::from_input_state(&state);

        assert!(snap.modifiers.shift);
        assert!(snap.modifiers.control);
        assert!(!snap.modifiers.alt);
        assert!(!snap.modifiers.super_key);
    }

    #[test]
    fn input_snapshot_captures_interaction_state() {
        use motif_core::ElementId;

        let mut state = InputState::new();
        state.set_hovered(Some(ElementId(42)));
        state.begin_press();

        let snap = InputStateSnapshot::from_input_state(&state);

        assert_eq!(snap.hovered_element, Some(42));
        assert_eq!(snap.pressed_element, Some(42));
    }

    #[test]
    fn input_snapshot_to_json() {
        use motif_core::ElementId;

        let mut state = InputState::new();
        state.cursor_position = Some(Point::new(100.0, 200.0));
        state.mouse_buttons.insert(MouseButton::Left);
        state.modifiers = ModifiersState::ALT;
        state.set_hovered(Some(ElementId(123)));

        let snap = InputStateSnapshot::from_input_state(&state);
        let json = snap.to_json();

        assert_eq!(json["cursor_position"]["x"], 100.0);
        assert_eq!(json["cursor_position"]["y"], 200.0);
        assert!(json["mouse_buttons"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("left")));
        assert_eq!(json["modifiers"]["alt"], true);
        assert_eq!(json["modifiers"]["shift"], false);
        assert_eq!(json["hovered_element"], 123);
        assert!(json["pressed_element"].is_null());
    }

    #[test]
    fn snapshot_from_empty_scene() {
        let scene = Scene::new();
        let snap = SceneSnapshot::from_scene(&scene, (800.0, 600.0), 2.0);

        assert_eq!(snap.quad_count, 0);
        assert_eq!(snap.text_run_count, 0);
        assert_eq!(snap.viewport_size, (800.0, 600.0));
        assert_eq!(snap.scale_factor, 2.0);
        assert!(snap.quads.is_empty());
    }

    #[test]
    fn snapshot_captures_quad_data() {
        let mut scene = Scene::new();
        let mut quad = Quad::new(
            DeviceRect::new(DevicePoint::new(10.0, 20.0), DeviceSize::new(100.0, 50.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        );
        quad.border_color = Srgba::new(0.0, 1.0, 0.0, 0.5);
        quad.border_widths = Edges::all(2.0);
        quad.corner_radii = Corners::all(8.0);
        scene.push_quad(quad);

        let snap = SceneSnapshot::from_scene(&scene, (1024.0, 768.0), 1.0);

        assert_eq!(snap.quad_count, 1);
        let qi = &snap.quads[0];
        assert_eq!(qi.bounds.x, 10.0);
        assert_eq!(qi.bounds.y, 20.0);
        assert_eq!(qi.bounds.w, 100.0);
        assert_eq!(qi.bounds.h, 50.0);
        assert_eq!(qi.color.r, 1.0);
        assert_eq!(qi.color.g, 0.0);
        assert_eq!(qi.border_color.g, 1.0);
        assert_eq!(qi.border_color.a, 0.5);
        assert_eq!(qi.border_widths.top, 2.0);
        assert_eq!(qi.corner_radii.top_left, 8.0);
    }

    #[test]
    fn snapshot_stats_json() {
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(
            DeviceRect::new(DevicePoint::new(0.0, 0.0), DeviceSize::new(10.0, 10.0)),
            Srgba::new(1.0, 1.0, 1.0, 1.0),
        ));
        scene.push_quad(Quad::new(
            DeviceRect::new(DevicePoint::new(20.0, 20.0), DeviceSize::new(10.0, 10.0)),
            Srgba::new(0.5, 0.5, 0.5, 1.0),
        ));

        let snap = SceneSnapshot::from_scene(&scene, (1920.0, 1080.0), 2.0);
        let stats = snap.stats();

        assert_eq!(stats["quad_count"], 2);
        assert_eq!(stats["text_run_count"], 0);
        assert_eq!(stats["viewport_size"][0], 1920.0);
        assert_eq!(stats["viewport_size"][1], 1080.0);
        assert_eq!(stats["scale_factor"], 2.0);
    }

    #[test]
    fn snapshot_serializes_to_json() {
        let scene = Scene::new();
        let snap = SceneSnapshot::from_scene(&scene, (800.0, 600.0), 1.0);
        let json = serde_json::to_string(&snap).unwrap();

        // Verify it's valid JSON that can be parsed
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["quad_count"], 0);
        assert_eq!(parsed["viewport_size"][0], 800.0);
    }

    fn dummy_font() -> FontData {
        FontData::new(Blob::from(vec![0u8; 4]), 0)
    }

    #[test]
    fn snapshot_captures_text_run_info() {
        let mut scene = Scene::new();
        let mut run = TextRun::new(
            DevicePoint::new(50.0, 100.0),
            Srgba::new(0.0, 0.0, 0.0, 1.0),
            16.0,
            dummy_font(),
        );
        run.push_glyph(1, 0.0, 0.0);
        run.push_glyph(2, 10.0, 0.0);
        run.push_glyph(3, 20.0, 0.0);
        scene.push_text_run(run);

        let snap = SceneSnapshot::from_scene(&scene, (800.0, 600.0), 1.0);

        assert_eq!(snap.text_runs.len(), 1);
        let tri = &snap.text_runs[0];
        assert_eq!(tri.origin_x, 50.0);
        assert_eq!(tri.origin_y, 100.0);
        assert_eq!(tri.font_size, 16.0);
        assert_eq!(tri.glyph_count, 3);
        assert_eq!(tri.color.r, 0.0);
        assert_eq!(tri.color.a, 1.0);
    }

    #[test]
    fn quads_json_returns_array() {
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(
            DeviceRect::new(DevicePoint::new(10.0, 20.0), DeviceSize::new(100.0, 50.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        ));
        scene.push_quad(Quad::new(
            DeviceRect::new(DevicePoint::new(30.0, 40.0), DeviceSize::new(200.0, 100.0)),
            Srgba::new(0.0, 1.0, 0.0, 0.5),
        ));

        let snap = SceneSnapshot::from_scene(&scene, (800.0, 600.0), 1.0);
        let json = snap.quads_json();
        let arr = json.as_array().expect("should be an array");

        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["bounds"]["x"], 10.0);
        assert_eq!(arr[0]["bounds"]["w"], 100.0);
        assert_eq!(arr[0]["color"]["r"], 1.0);
        assert_eq!(arr[1]["bounds"]["x"], 30.0);
        assert_eq!(arr[1]["color"]["g"], 1.0);
        assert_eq!(arr[1]["color"]["a"], 0.5);
    }

    #[test]
    fn quads_json_includes_clip_info() {
        let mut scene = Scene::new();
        let mut quad = Quad::new(
            DeviceRect::new(DevicePoint::new(0.0, 0.0), DeviceSize::new(50.0, 50.0)),
            Srgba::new(1.0, 1.0, 1.0, 1.0),
        );
        quad.clip_bounds = Some(DeviceRect::new(
            DevicePoint::new(5.0, 5.0),
            DeviceSize::new(40.0, 40.0),
        ));
        scene.push_quad(quad);

        let snap = SceneSnapshot::from_scene(&scene, (800.0, 600.0), 1.0);
        let json = snap.quads_json();
        let arr = json.as_array().unwrap();

        assert_eq!(arr[0]["has_clip"], true);
        assert_eq!(arr[0]["clip_bounds"]["x"], 5.0);
        assert_eq!(arr[0]["clip_bounds"]["w"], 40.0);
    }

    #[test]
    fn quads_json_empty_scene() {
        let scene = Scene::new();
        let snap = SceneSnapshot::from_scene(&scene, (800.0, 600.0), 1.0);
        let json = snap.quads_json();
        let arr = json.as_array().unwrap();
        assert!(arr.is_empty());
    }

    #[test]
    fn text_runs_json_returns_array() {
        let mut scene = Scene::new();
        let mut run = TextRun::new(
            DevicePoint::new(10.0, 20.0),
            Srgba::new(0.2, 0.3, 0.4, 1.0),
            14.0,
            dummy_font(),
        );
        run.push_glyph(1, 0.0, 0.0);
        run.push_glyph(2, 8.0, 0.0);
        scene.push_text_run(run);

        let snap = SceneSnapshot::from_scene(&scene, (800.0, 600.0), 1.0);
        let json = snap.text_runs_json();
        let arr = json.as_array().expect("should be an array");

        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["origin"]["x"], 10.0);
        assert_eq!(arr[0]["origin"]["y"], 20.0);
        assert_eq!(arr[0]["font_size"], 14.0);
        assert_eq!(arr[0]["glyph_count"], 2);
    }

    #[test]
    fn text_runs_json_empty_scene() {
        let scene = Scene::new();
        let snap = SceneSnapshot::from_scene(&scene, (800.0, 600.0), 1.0);
        let json = snap.text_runs_json();
        let arr = json.as_array().unwrap();
        assert!(arr.is_empty());
    }

    // --- HitTreeSnapshot tests ---

    fn hit_rect(x: f32, y: f32, w: f32, h: f32) -> motif_core::Rect {
        motif_core::Rect::new(motif_core::Point::new(x, y), motif_core::Size::new(w, h))
    }

    #[test]
    fn hit_tree_snapshot_from_empty_tree() {
        let tree = HitTree::new();
        let snap = HitTreeSnapshot::from_hit_tree(&tree);
        assert!(snap.entries.is_empty());
    }

    #[test]
    fn hit_tree_snapshot_captures_entries() {
        use motif_core::ElementId;
        let mut tree = HitTree::new();
        tree.push(ElementId(1), hit_rect(10.0, 20.0, 100.0, 50.0));
        tree.push(ElementId(2), hit_rect(50.0, 60.0, 30.0, 20.0));

        let snap = HitTreeSnapshot::from_hit_tree(&tree);

        assert_eq!(snap.entries.len(), 2);
        assert_eq!(snap.entries[0].id, 1);
        assert_eq!(snap.entries[0].bounds.x, 10.0);
        assert_eq!(snap.entries[0].bounds.y, 20.0);
        assert_eq!(snap.entries[0].bounds.w, 100.0);
        assert_eq!(snap.entries[0].bounds.h, 50.0);
        assert_eq!(snap.entries[0].z_index, 0);
        assert_eq!(snap.entries[1].id, 2);
        assert_eq!(snap.entries[1].z_index, 1);
    }

    #[test]
    fn list_json_returns_flat_array() {
        use motif_core::ElementId;
        let mut tree = HitTree::new();
        tree.push(ElementId(5), hit_rect(0.0, 0.0, 200.0, 100.0));
        tree.push(ElementId(7), hit_rect(10.0, 10.0, 50.0, 30.0));

        let snap = HitTreeSnapshot::from_hit_tree(&tree);
        let json = snap.list_json();
        let arr = json.as_array().unwrap();

        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["id"], 5);
        assert_eq!(arr[0]["z_index"], 0);
        assert_eq!(arr[0]["bounds"]["x"], 0.0);
        assert_eq!(arr[1]["id"], 7);
        assert_eq!(arr[1]["z_index"], 1);
        assert_eq!(arr[1]["bounds"]["x"], 10.0);
    }

    #[test]
    fn list_json_empty_tree() {
        let tree = HitTree::new();
        let snap = HitTreeSnapshot::from_hit_tree(&tree);
        let json = snap.list_json();
        assert!(json.as_array().unwrap().is_empty());
    }

    #[test]
    fn tree_json_empty() {
        let tree = HitTree::new();
        let snap = HitTreeSnapshot::from_hit_tree(&tree);
        let json = snap.tree_json();
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[test]
    fn tree_json_single_element() {
        use motif_core::ElementId;
        let mut tree = HitTree::new();
        tree.push(ElementId(1), hit_rect(0.0, 0.0, 100.0, 100.0));

        let snap = HitTreeSnapshot::from_hit_tree(&tree);
        let json = snap.tree_json();
        let arr = json.as_array().unwrap();

        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], 1);
        assert!(arr[0]["children"].as_array().unwrap().is_empty());
    }

    #[test]
    fn tree_json_nested_elements() {
        use motif_core::ElementId;
        let mut tree = HitTree::new();
        // Parent painted first (z=0), child inside it painted second (z=1).
        tree.push(ElementId(1), hit_rect(0.0, 0.0, 200.0, 200.0));
        tree.push(ElementId(2), hit_rect(50.0, 50.0, 100.0, 100.0));

        let snap = HitTreeSnapshot::from_hit_tree(&tree);
        let json = snap.tree_json();
        let arr = json.as_array().unwrap();

        assert_eq!(arr.len(), 1, "should have one root");
        assert_eq!(arr[0]["id"], 1);
        let children = arr[0]["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["id"], 2);
        assert!(children[0]["children"].as_array().unwrap().is_empty());
    }

    #[test]
    fn tree_json_siblings_become_roots() {
        use motif_core::ElementId;
        let mut tree = HitTree::new();
        // Two non-overlapping elements at the same level.
        tree.push(ElementId(1), hit_rect(0.0, 0.0, 100.0, 100.0));
        tree.push(ElementId(2), hit_rect(200.0, 0.0, 100.0, 100.0));

        let snap = HitTreeSnapshot::from_hit_tree(&tree);
        let json = snap.tree_json();
        let arr = json.as_array().unwrap();

        assert_eq!(arr.len(), 2, "both should be roots");
        assert!(arr[0]["children"].as_array().unwrap().is_empty());
        assert!(arr[1]["children"].as_array().unwrap().is_empty());
    }

    #[test]
    fn tree_json_three_levels_deep() {
        use motif_core::ElementId;
        let mut tree = HitTree::new();
        tree.push(ElementId(1), hit_rect(0.0, 0.0, 300.0, 300.0)); // root
        tree.push(ElementId(2), hit_rect(50.0, 50.0, 200.0, 200.0)); // child of 1
        tree.push(ElementId(3), hit_rect(100.0, 100.0, 100.0, 100.0)); // child of 2

        let snap = HitTreeSnapshot::from_hit_tree(&tree);
        let json = snap.tree_json();
        let arr = json.as_array().unwrap();

        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], 1);
        let l1 = arr[0]["children"].as_array().unwrap();
        assert_eq!(l1.len(), 1);
        assert_eq!(l1[0]["id"], 2);
        let l2 = l1[0]["children"].as_array().unwrap();
        assert_eq!(l2.len(), 1);
        assert_eq!(l2[0]["id"], 3);
    }

    #[test]
    fn tree_json_tightest_parent_wins() {
        use motif_core::ElementId;
        let mut tree = HitTree::new();
        // Outer container
        tree.push(ElementId(1), hit_rect(0.0, 0.0, 400.0, 400.0));
        // Inner container (both contain element 3)
        tree.push(ElementId(2), hit_rect(50.0, 50.0, 200.0, 200.0));
        // Leaf — inside both 1 and 2; should be child of 2 (smaller area).
        tree.push(ElementId(3), hit_rect(100.0, 100.0, 50.0, 50.0));

        let snap = HitTreeSnapshot::from_hit_tree(&tree);
        let json = snap.tree_json();
        let arr = json.as_array().unwrap();

        assert_eq!(arr.len(), 1);
        let root_children = arr[0]["children"].as_array().unwrap();
        // id=2 is child of root (id=1)
        assert_eq!(root_children.len(), 1);
        assert_eq!(root_children[0]["id"], 2);
        // id=3 is child of id=2, not id=1
        let inner_children = root_children[0]["children"].as_array().unwrap();
        assert_eq!(inner_children.len(), 1);
        assert_eq!(inner_children[0]["id"], 3);
    }
}
