//! Scene snapshot: a serializable capture of the current scene state.

use motif_core::Scene;
use serde::Serialize;

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

#[cfg(test)]
mod tests {
    use super::*;
    use motif_core::{
        Corners, DevicePoint, DeviceRect, DeviceSize, Edges, FontData, Quad, Scene, Srgba,
        TextRun,
    };
    use linebender_resource_handle::Blob;

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
}
