//! Window screenshot capture and comparison utilities.
//!
//! Uses `CGWindowListCreateImage` to capture the actual rendered window
//! pixels — exactly what's on screen, including Metal rendering, text, etc.
//!
//! Also provides [`diff_screenshots`] for comparing two PNG files and producing
//! pixel-level diff statistics, which is useful for visual regression testing.

use std::io;

/// Capture a window to a PNG file using macOS screen capture.
///
/// `window_id` is the CGWindowID of the window to capture.
/// Returns `Ok(())` on success, or an `io::Error` on failure.
pub fn capture_window_to_png(window_id: u32, path: &str) -> io::Result<()> {
    capture_window_to_png_impl(window_id, path)
}

/// Result of comparing two screenshots pixel-by-pixel.
#[derive(Debug, Clone)]
pub struct DiffResult {
    /// Number of pixels that differ (per-channel max exceeds threshold).
    pub changed_pixels: u64,
    /// Total number of pixels in the images.
    pub total_pixels: u64,
    /// Fraction of changed pixels (0.0 = identical, 1.0 = all different).
    pub diff_ratio: f64,
    /// Maximum per-channel difference seen across all pixels (0..=255).
    pub max_delta: u8,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// Compare two PNG screenshots and return pixel-level diff statistics.
///
/// Both images must exist and have the same dimensions.  If `output_path`
/// is `Some`, a diff image is written there: changed pixels are shown in red
/// with brightness proportional to the per-channel delta; unchanged pixels are
/// rendered as a dimmed grayscale blend of the two originals.
///
/// `threshold` is the minimum per-channel delta (0–255) that counts as
/// "changed".  Use `0` to flag any difference whatsoever.
pub fn diff_screenshots(
    path_a: &str,
    path_b: &str,
    output_path: Option<&str>,
    threshold: u8,
) -> io::Result<DiffResult> {
    use image::GenericImageView;
    use image::RgbaImage;
    use std::path::Path;

    let img_a = image::open(Path::new(path_a)).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Cannot open '{path_a}': {e}"),
        )
    })?;
    let img_b = image::open(Path::new(path_b)).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Cannot open '{path_b}': {e}"),
        )
    })?;

    let (width_a, height_a) = img_a.dimensions();
    let (width_b, height_b) = img_b.dimensions();

    if width_a != width_b || height_a != height_b {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Image dimensions differ: {}x{} vs {}x{}",
                width_a, height_a, width_b, height_b
            ),
        ));
    }

    let width = width_a;
    let height = height_a;
    let total_pixels = width as u64 * height as u64;

    let pixels_a = img_a.to_rgba8();
    let pixels_b = img_b.to_rgba8();

    let mut changed_pixels: u64 = 0;
    let mut max_delta: u8 = 0;
    // Allocate diff buffer only when an output image is requested.
    let mut diff_data = if output_path.is_some() {
        vec![0u8; (width * height * 4) as usize]
    } else {
        Vec::new()
    };

    for y in 0..height {
        for x in 0..width {
            let pa = pixels_a.get_pixel(x, y);
            let pb = pixels_b.get_pixel(x, y);

            let delta_r = pa[0].abs_diff(pb[0]);
            let delta_g = pa[1].abs_diff(pb[1]);
            let delta_b = pa[2].abs_diff(pb[2]);
            let delta_a = pa[3].abs_diff(pb[3]);
            let max_ch = delta_r.max(delta_g).max(delta_b).max(delta_a);

            if max_ch > max_delta {
                max_delta = max_ch;
            }

            if max_ch > threshold {
                changed_pixels += 1;
                if output_path.is_some() {
                    let idx = (y * width + x) as usize * 4;
                    // Red channel proportional to delta; green/blue black; fully opaque.
                    diff_data[idx] = max_ch;
                    diff_data[idx + 1] = 0;
                    diff_data[idx + 2] = 0;
                    diff_data[idx + 3] = 255;
                }
            } else if output_path.is_some() {
                // Unchanged pixel: dimmed grayscale average of the two originals.
                let idx = (y * width + x) as usize * 4;
                let gray = ((pa[0] as u16 + pb[0] as u16) / 4) as u8;
                diff_data[idx] = gray;
                diff_data[idx + 1] = gray;
                diff_data[idx + 2] = gray;
                diff_data[idx + 3] = 255;
            }
        }
    }

    if let Some(out_path) = output_path {
        let img = RgbaImage::from_raw(width, height, diff_data)
            .ok_or_else(|| io::Error::other("Failed to create diff image buffer"))?;
        img.save(Path::new(out_path)).map_err(io::Error::other)?;
    }

    let diff_ratio = if total_pixels > 0 {
        changed_pixels as f64 / total_pixels as f64
    } else {
        0.0
    };

    Ok(DiffResult {
        changed_pixels,
        total_pixels,
        diff_ratio,
        max_delta,
        width,
        height,
    })
}

#[cfg(target_os = "macos")]
fn capture_window_to_png_impl(window_id: u32, path: &str) -> io::Result<()> {
    use core_graphics::display::*;
    use core_graphics::geometry::{CGPoint, CGRect, CGSize};
    use image::RgbaImage;
    use std::path::Path;

    // Capture the specific window
    let cg_image = CGDisplay::screenshot(
        CGRect::new(
            &CGPoint::new(0.0, 0.0),
            &CGSize::new(0.0, 0.0), // zero rect = capture whole window
        ),
        kCGWindowListOptionIncludingWindow,
        window_id,
        kCGWindowImageBoundsIgnoreFraming,
    )
    .ok_or_else(|| io::Error::other("Failed to capture window"))?;

    let width = cg_image.width() as u32;
    let height = cg_image.height() as u32;
    let bytes_per_row = cg_image.bytes_per_row();
    let data = cg_image.data();
    let raw_bytes = data.bytes();

    // CGImage gives us BGRA, convert to RGBA for PNG
    let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height as usize {
        for x in 0..width as usize {
            let offset = y * bytes_per_row + x * 4;
            if offset + 3 < raw_bytes.len() {
                let b = raw_bytes[offset];
                let g = raw_bytes[offset + 1];
                let r = raw_bytes[offset + 2];
                let a = raw_bytes[offset + 3];
                rgba_data.extend_from_slice(&[r, g, b, a]);
            }
        }
    }

    let img = RgbaImage::from_raw(width, height, rgba_data)
        .ok_or_else(|| io::Error::other("Failed to create image buffer"))?;

    img.save(Path::new(path)).map_err(io::Error::other)?;

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn capture_window_to_png_impl(_window_id: u32, _path: &str) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Window capture is only supported on macOS",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_nonexistent_window_returns_error() {
        let result = capture_window_to_png(999999, "/tmp/motif-test-bad-window.png");
        assert!(result.is_err());
    }

    // --- diff_screenshots tests ---

    /// Write a tiny PNG from raw RGBA bytes and return its path.
    fn write_test_png(name: &str, width: u32, height: u32, pixels: &[u8]) -> String {
        use image::RgbaImage;
        let path = format!("/tmp/motif-diff-test-{name}.png");
        let img = RgbaImage::from_raw(width, height, pixels.to_vec()).unwrap();
        img.save(&path).unwrap();
        path
    }

    #[test]
    fn diff_identical_images_returns_zero_changed() {
        let pixels = [255u8, 0, 0, 255, 0, 255, 0, 255]; // 2x1 RGBA
        let a = write_test_png("identical-a", 2, 1, &pixels);
        let b = write_test_png("identical-b", 2, 1, &pixels);

        let result = diff_screenshots(&a, &b, None, 0).unwrap();
        assert_eq!(result.changed_pixels, 0);
        assert_eq!(result.total_pixels, 2);
        assert_eq!(result.diff_ratio, 0.0);
        assert_eq!(result.max_delta, 0);
        assert_eq!(result.width, 2);
        assert_eq!(result.height, 1);
    }

    #[test]
    fn diff_completely_different_images() {
        let a_pixels = [0u8, 0, 0, 255, 0, 0, 0, 255]; // 2x1 black
        let b_pixels = [255u8, 255, 255, 255, 255, 255, 255, 255]; // 2x1 white

        let a = write_test_png("diff-black", 2, 1, &a_pixels);
        let b = write_test_png("diff-white", 2, 1, &b_pixels);

        let result = diff_screenshots(&a, &b, None, 0).unwrap();
        assert_eq!(result.changed_pixels, 2);
        assert_eq!(result.total_pixels, 2);
        assert_eq!(result.diff_ratio, 1.0);
        assert_eq!(result.max_delta, 255);
    }

    #[test]
    fn diff_partial_change() {
        // 2x1: first pixel identical, second differs.
        let a_pixels = [100u8, 100, 100, 255, 200, 200, 200, 255];
        let b_pixels = [100u8, 100, 100, 255, 100, 100, 100, 255]; // second pixel changed by 100

        let a = write_test_png("partial-a", 2, 1, &a_pixels);
        let b = write_test_png("partial-b", 2, 1, &b_pixels);

        let result = diff_screenshots(&a, &b, None, 0).unwrap();
        assert_eq!(result.changed_pixels, 1);
        assert_eq!(result.total_pixels, 2);
        assert!((result.diff_ratio - 0.5).abs() < f64::EPSILON);
        assert_eq!(result.max_delta, 100);
    }

    #[test]
    fn diff_threshold_filters_small_changes() {
        // 2x1: both pixels differ by 10.
        let a_pixels = [50u8, 50, 50, 255, 50, 50, 50, 255];
        let b_pixels = [60u8, 60, 60, 255, 60, 60, 60, 255];

        let a = write_test_png("thresh-a", 2, 1, &a_pixels);
        let b = write_test_png("thresh-b", 2, 1, &b_pixels);

        // threshold=10 → delta of 10 is NOT above threshold → 0 changed
        let result_filtered = diff_screenshots(&a, &b, None, 10).unwrap();
        assert_eq!(result_filtered.changed_pixels, 0);

        // threshold=9 → delta of 10 IS above threshold → 2 changed
        let result_all = diff_screenshots(&a, &b, None, 9).unwrap();
        assert_eq!(result_all.changed_pixels, 2);
    }

    #[test]
    fn diff_dimension_mismatch_returns_error() {
        let a_pixels = [255u8, 0, 0, 255, 0, 255, 0, 255]; // 2x1
        let b_pixels = [0u8, 0, 255, 255]; // 1x1

        let a = write_test_png("dim-a", 2, 1, &a_pixels);
        let b = write_test_png("dim-b", 1, 1, &b_pixels);

        let result = diff_screenshots(&a, &b, None, 0);
        assert!(result.is_err(), "dimension mismatch should return error");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("dimensions differ"),
            "error should mention dimensions: {msg}"
        );
    }

    #[test]
    fn diff_nonexistent_file_returns_error() {
        let result = diff_screenshots(
            "/tmp/motif-nonexistent-a.png",
            "/tmp/motif-nonexistent-b.png",
            None,
            0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn diff_writes_output_image() {
        let a_pixels = [0u8, 0, 0, 255]; // 1x1 black
        let b_pixels = [255u8, 0, 0, 255]; // 1x1 red

        let a = write_test_png("out-a", 1, 1, &a_pixels);
        let b = write_test_png("out-b", 1, 1, &b_pixels);
        let output = "/tmp/motif-diff-out.png";

        let result = diff_screenshots(&a, &b, Some(output), 0).unwrap();
        assert_eq!(result.changed_pixels, 1);
        assert!(
            std::path::Path::new(output).exists(),
            "output diff image should be written"
        );
    }
}
