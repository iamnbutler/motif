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

/// A serializable snapshot of the current scene state.
#[derive(Debug, Clone, Serialize)]
pub struct SceneSnapshot {
    pub quads: Vec<QuadInfo>,
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
                }
            })
            .collect();

        Self {
            quad_count: quads.len(),
            quads,
            text_run_count: scene.text_run_count(),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use motif_core::{Corners, DevicePoint, DeviceRect, DeviceSize, Edges, Quad, Scene, Srgba};

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
}
