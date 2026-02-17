//! Software-rendered screenshot capture from a scene snapshot.
//!
//! Renders a simplified version of the scene to an in-memory image buffer
//! and saves it as PNG. This is not pixel-perfect with the Metal renderer
//! but shows layout, colors, and positions -- enough to verify structure.

use std::io;
use std::path::Path;

use image::{ImageBuffer, Rgba};

use crate::snapshot::SceneSnapshot;

/// Capture a scene snapshot to a PNG file via software rendering.
///
/// Each quad is rendered as a filled rectangle with its background color.
/// Text runs are approximated as small rectangles at the text origin.
///
/// Returns `Ok(())` on success, or an `io::Error` on failure.
pub fn capture_scene_to_png(
    snapshot: &SceneSnapshot,
    path: &str,
    width: u32,
    height: u32,
) -> io::Result<()> {
    let buffer = render_scene_to_buffer(snapshot, width, height);
    buffer
        .save(Path::new(path))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

/// Render a scene snapshot to an RGBA image buffer (no file I/O).
///
/// Useful for testing pixel contents without touching the filesystem.
pub fn render_scene_to_buffer(
    snapshot: &SceneSnapshot,
    width: u32,
    height: u32,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(width, height, Rgba([255, 255, 255, 255]));

    // Draw quads as filled rectangles.
    for quad in &snapshot.quads {
        let color = srgba_to_rgba8(&quad.color);

        let x_start = (quad.bounds.x as i32).max(0) as u32;
        let y_start = (quad.bounds.y as i32).max(0) as u32;
        let x_end = ((quad.bounds.x + quad.bounds.w) as u32).min(width);
        let y_end = ((quad.bounds.y + quad.bounds.h) as u32).min(height);

        // Determine effective clip region.
        let (cx_start, cy_start, cx_end, cy_end) = if let Some(clip) = &quad.clip_bounds {
            let cs = (clip.x as i32).max(0) as u32;
            let ce = ((clip.x + clip.w) as u32).min(width);
            let rs = (clip.y as i32).max(0) as u32;
            let re = ((clip.y + clip.h) as u32).min(height);
            (cs, rs, ce, re)
        } else {
            (0, 0, width, height)
        };

        for y in y_start..y_end {
            for x in x_start..x_end {
                if x >= cx_start && x < cx_end && y >= cy_start && y < cy_end {
                    blend_pixel(&mut img, x, y, color);
                }
            }
        }
    }

    // Draw text runs as small indicator rectangles at the text origin.
    for text_run in &snapshot.text_runs {
        let color = srgba_to_rgba8(&text_run.color);

        // Draw a small rectangle proportional to font size at the text origin.
        let indicator_w = (text_run.glyph_count as f32 * text_run.font_size * 0.6) as u32;
        let indicator_h = text_run.font_size as u32;

        let x_start = (text_run.origin_x as i32).max(0) as u32;
        let y_start = ((text_run.origin_y - text_run.font_size) as i32).max(0) as u32;
        let x_end = (x_start + indicator_w).min(width);
        let y_end = (y_start + indicator_h).min(height);

        for y in y_start..y_end {
            for x in x_start..x_end {
                blend_pixel(&mut img, x, y, color);
            }
        }
    }

    img
}

/// Convert a floating-point SRGBA color to a byte RGBA pixel.
fn srgba_to_rgba8(c: &crate::snapshot::ColorInfo) -> Rgba<u8> {
    Rgba([
        (c.r * 255.0).round() as u8,
        (c.g * 255.0).round() as u8,
        (c.b * 255.0).round() as u8,
        (c.a * 255.0).round() as u8,
    ])
}

/// Simple alpha-over blending onto a pixel in the buffer.
fn blend_pixel(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, x: u32, y: u32, src: Rgba<u8>) {
    let dst = img.get_pixel(x, y);
    let sa = src[3] as f32 / 255.0;
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);

    if out_a < f32::EPSILON {
        *img.get_pixel_mut(x, y) = Rgba([0, 0, 0, 0]);
        return;
    }

    let blend = |s: u8, d: u8| -> u8 {
        let s = s as f32 / 255.0;
        let d = d as f32 / 255.0;
        ((s * sa + d * da * (1.0 - sa)) / out_a * 255.0).round() as u8
    };

    *img.get_pixel_mut(x, y) = Rgba([
        blend(src[0], dst[0]),
        blend(src[1], dst[1]),
        blend(src[2], dst[2]),
        (out_a * 255.0).round() as u8,
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{
        BoundsInfo, ColorInfo, CornersInfo, EdgesInfo, QuadInfo, SceneSnapshot, TextRunInfo,
    };
    use std::path::Path;

    fn empty_snapshot(width: f32, height: f32) -> SceneSnapshot {
        SceneSnapshot {
            quads: vec![],
            text_runs: vec![],
            quad_count: 0,
            text_run_count: 0,
            viewport_size: (width, height),
            scale_factor: 1.0,
        }
    }

    fn red_quad(x: f32, y: f32, w: f32, h: f32) -> QuadInfo {
        QuadInfo {
            bounds: BoundsInfo { x, y, w, h },
            color: ColorInfo {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            border_color: ColorInfo {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            border_widths: EdgesInfo {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            },
            corner_radii: CornersInfo {
                top_left: 0.0,
                top_right: 0.0,
                bottom_right: 0.0,
                bottom_left: 0.0,
            },
            has_clip: false,
            clip_bounds: None,
        }
    }

    #[test]
    fn empty_scene_produces_white_image() {
        let snap = empty_snapshot(100.0, 80.0);
        let img = render_scene_to_buffer(&snap, 100, 80);

        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 80);

        // Every pixel should be white (the background fill).
        for pixel in img.pixels() {
            assert_eq!(*pixel, Rgba([255, 255, 255, 255]));
        }
    }

    #[test]
    fn single_quad_fills_correct_region() {
        let mut snap = empty_snapshot(100.0, 100.0);
        snap.quads.push(red_quad(10.0, 20.0, 30.0, 40.0));
        snap.quad_count = 1;

        let img = render_scene_to_buffer(&snap, 100, 100);

        // Pixel inside the quad should be red.
        assert_eq!(*img.get_pixel(15, 30), Rgba([255, 0, 0, 255]));

        // Pixel outside the quad should be white (background).
        assert_eq!(*img.get_pixel(0, 0), Rgba([255, 255, 255, 255]));
        assert_eq!(*img.get_pixel(50, 50), Rgba([255, 255, 255, 255]));
    }

    #[test]
    fn quad_clipped_to_image_bounds() {
        let mut snap = empty_snapshot(50.0, 50.0);
        // Quad extends beyond image edges.
        snap.quads.push(red_quad(40.0, 40.0, 100.0, 100.0));
        snap.quad_count = 1;

        let img = render_scene_to_buffer(&snap, 50, 50);

        // Pixel inside the visible part of the quad.
        assert_eq!(*img.get_pixel(45, 45), Rgba([255, 0, 0, 255]));

        // Should not panic -- the function handles out-of-bounds gracefully.
        assert_eq!(img.width(), 50);
        assert_eq!(img.height(), 50);
    }

    #[test]
    fn quad_with_clip_bounds_clips_rendering() {
        let mut snap = empty_snapshot(100.0, 100.0);
        let mut q = red_quad(10.0, 10.0, 80.0, 80.0);
        q.has_clip = true;
        q.clip_bounds = Some(BoundsInfo {
            x: 30.0,
            y: 30.0,
            w: 40.0,
            h: 40.0,
        });
        snap.quads.push(q);
        snap.quad_count = 1;

        let img = render_scene_to_buffer(&snap, 100, 100);

        // Inside both quad bounds AND clip bounds: red.
        assert_eq!(*img.get_pixel(40, 40), Rgba([255, 0, 0, 255]));

        // Inside quad bounds but OUTSIDE clip bounds: white (not drawn).
        assert_eq!(*img.get_pixel(15, 15), Rgba([255, 255, 255, 255]));
    }

    #[test]
    fn semi_transparent_quad_blends_over_background() {
        let mut snap = empty_snapshot(10.0, 10.0);
        let mut q = red_quad(0.0, 0.0, 10.0, 10.0);
        q.color.a = 0.5;
        snap.quads.push(q);
        snap.quad_count = 1;

        let img = render_scene_to_buffer(&snap, 10, 10);

        let pixel = *img.get_pixel(5, 5);
        // Red blended over white at ~50% alpha.
        // Due to u8 quantization of the alpha channel, results may be off
        // by 1 from the ideal float calculation. Use a tolerance of 1.
        assert_eq!(pixel[0], 255); // red channel stays 255
        assert!((pixel[1] as i16 - 128).unsigned_abs() <= 1); // green ~128
        assert!((pixel[2] as i16 - 128).unsigned_abs() <= 1); // blue ~128
    }

    #[test]
    fn multiple_quads_render_in_order() {
        let mut snap = empty_snapshot(100.0, 100.0);

        // First quad: red, covering the center.
        snap.quads.push(red_quad(20.0, 20.0, 60.0, 60.0));

        // Second quad: blue, overlapping the right half.
        let mut blue_quad = red_quad(50.0, 20.0, 40.0, 60.0);
        blue_quad.color = ColorInfo {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        };
        snap.quads.push(blue_quad);
        snap.quad_count = 2;

        let img = render_scene_to_buffer(&snap, 100, 100);

        // Left of overlap: red.
        assert_eq!(*img.get_pixel(30, 40), Rgba([255, 0, 0, 255]));

        // In the overlap region: blue (second quad drawn on top).
        assert_eq!(*img.get_pixel(60, 40), Rgba([0, 0, 255, 255]));
    }

    #[test]
    fn text_run_renders_indicator_rectangle() {
        let mut snap = empty_snapshot(200.0, 200.0);
        snap.text_runs.push(TextRunInfo {
            origin_x: 10.0,
            origin_y: 30.0,
            font_size: 16.0,
            glyph_count: 5,
            color: ColorInfo {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        });
        snap.text_run_count = 1;

        let img = render_scene_to_buffer(&snap, 200, 200);

        // The indicator rectangle starts at (origin_x, origin_y - font_size)
        // = (10, 14). It should be black.
        let pixel = *img.get_pixel(12, 20);
        assert_eq!(pixel, Rgba([0, 0, 0, 255]));

        // Outside the indicator: white.
        assert_eq!(*img.get_pixel(0, 0), Rgba([255, 255, 255, 255]));
    }

    #[test]
    fn capture_scene_to_png_writes_file() {
        let snap = empty_snapshot(64.0, 48.0);
        let path = "/tmp/motif-test-screenshot.png";

        // Clean up from any previous run.
        let _ = std::fs::remove_file(path);

        capture_scene_to_png(&snap, path, 64, 48).expect("should save PNG");

        assert!(Path::new(path).exists(), "PNG file should be created");

        // Read it back and verify dimensions.
        let loaded = image::open(path).expect("should load saved PNG");
        assert_eq!(loaded.width(), 64);
        assert_eq!(loaded.height(), 48);

        // Clean up.
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn capture_scene_with_quads_to_png() {
        let mut snap = empty_snapshot(100.0, 100.0);
        snap.quads.push(red_quad(10.0, 10.0, 50.0, 50.0));
        snap.quad_count = 1;

        let path = "/tmp/motif-test-screenshot-quads.png";
        let _ = std::fs::remove_file(path);

        capture_scene_to_png(&snap, path, 100, 100).expect("should save PNG");

        let loaded = image::open(path).expect("should load saved PNG");
        assert_eq!(loaded.width(), 100);
        assert_eq!(loaded.height(), 100);

        // Verify the red quad is present in the loaded image.
        let rgba = loaded.to_rgba8();
        let pixel = *rgba.get_pixel(30, 30);
        assert_eq!(pixel[0], 255); // red
        assert_eq!(pixel[1], 0);
        assert_eq!(pixel[2], 0);

        let _ = std::fs::remove_file(path);
    }
}
