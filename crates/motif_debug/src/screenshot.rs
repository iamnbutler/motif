//! Window screenshot capture using macOS native APIs.
//!
//! Uses `CGWindowListCreateImage` to capture the actual rendered window
//! pixels — exactly what's on screen, including Metal rendering, text, etc.

use std::io;
use std::path::Path;

/// Capture a window to a PNG file using macOS screen capture.
///
/// `window_id` is the CGWindowID of the window to capture.
/// Returns `Ok(())` on success, or an `io::Error` on failure.
pub fn capture_window_to_png(window_id: u32, path: &str) -> io::Result<()> {
    capture_window_to_png_impl(window_id, path)
}

#[cfg(target_os = "macos")]
fn capture_window_to_png_impl(window_id: u32, path: &str) -> io::Result<()> {
    use core_graphics::display::*;
    use core_graphics::geometry::{CGPoint, CGRect, CGSize};
    use image::RgbaImage;

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
    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to capture window"))?;

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
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to create image buffer"))?;

    img.save(Path::new(path))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

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
}
