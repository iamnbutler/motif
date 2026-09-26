//! Core geometry primitives for motif.

// Re-export Unit trait for users defining custom coordinate spaces
pub use glamour::Unit;

/// Logical pixels - DPI-independent coordinate space.
pub struct LogicalPixels;

impl glamour::Unit for LogicalPixels {
    type Scalar = f32;
}

/// Device pixels - physical pixel coordinate space.
pub struct DevicePixels;

impl glamour::Unit for DevicePixels {
    type Scalar = f32;
}

// Logical space type aliases
pub type Point = glamour::Point2<LogicalPixels>;
pub type Size = glamour::Size2<LogicalPixels>;
pub type Rect = glamour::Rect<LogicalPixels>;
pub type Vector = glamour::Vector2<LogicalPixels>;

// Device space type aliases
pub type DevicePoint = glamour::Point2<DevicePixels>;
pub type DeviceSize = glamour::Size2<DevicePixels>;
pub type DeviceRect = glamour::Rect<DevicePixels>;

/// Scale factor for converting between logical and device pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleFactor(pub f32);

impl ScaleFactor {
    pub fn scale_point(&self, p: Point) -> DevicePoint {
        DevicePoint::new(p.x * self.0, p.y * self.0)
    }

    pub fn scale_size(&self, s: Size) -> DeviceSize {
        DeviceSize::new(s.width * self.0, s.height * self.0)
    }

    pub fn scale_rect(&self, r: Rect) -> DeviceRect {
        DeviceRect::new(self.scale_point(r.origin), self.scale_size(r.size))
    }

    pub fn unscale_point(&self, p: DevicePoint) -> Point {
        Point::new(p.x / self.0, p.y / self.0)
    }

    pub fn unscale_size(&self, s: DeviceSize) -> Size {
        Size::new(s.width / self.0, s.height / self.0)
    }

    pub fn unscale_rect(&self, r: DeviceRect) -> Rect {
        Rect::new(self.unscale_point(r.origin), self.unscale_size(r.size))
    }
}

/// Edge values for padding, margin, border widths.
/// Follows CSS order: top, right, bottom, left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Edges<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy> Edges<T> {
    pub fn all(value: T) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn symmetric(vertical: T, horizontal: T) -> Self {
        Self {
            top: vertical,
            bottom: vertical,
            left: horizontal,
            right: horizontal,
        }
    }
}

impl<T: Copy + std::ops::Add<Output = T>> Edges<T> {
    pub fn horizontal(&self) -> T {
        self.left + self.right
    }

    pub fn vertical(&self) -> T {
        self.top + self.bottom
    }
}

/// Corner values for border radii.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Corners<T> {
    pub top_left: T,
    pub top_right: T,
    pub bottom_right: T,
    pub bottom_left: T,
}

impl<T: Copy> Corners<T> {
    pub fn all(value: T) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_right: value,
            bottom_left: value,
        }
    }

    pub fn top_bottom(top: T, bottom: T) -> Self {
        Self {
            top_left: top,
            top_right: top,
            bottom_left: bottom,
            bottom_right: bottom,
        }
    }
}

/// Axis in 2D space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub fn invert(self) -> Self {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }
}

/// Extension methods for [`Point`] (logical pixel coordinates).
///
/// Import this trait to unlock helper methods on `Point` values.
pub trait PointExt {
    /// Round coordinates to the nearest device pixel boundary.
    ///
    /// Converts to device pixels, rounds to the nearest integer, then
    /// converts back to logical pixels. Eliminates sub-pixel rendering
    /// artifacts on HiDPI displays.
    fn snap_to_pixel(self, scale: ScaleFactor) -> Point;

    /// Euclidean distance to another point.
    fn distance_to(self, other: Point) -> f32;
}

impl PointExt for Point {
    fn snap_to_pixel(self, scale: ScaleFactor) -> Point {
        let device = scale.scale_point(self);
        let snapped = DevicePoint::new(device.x.round(), device.y.round());
        scale.unscale_point(snapped)
    }

    fn distance_to(self, other: Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Extension methods for [`Size`] (logical pixel dimensions).
///
/// Import this trait to unlock helper methods on `Size` values.
pub trait SizeExt {
    /// Total area in square logical pixels (`width × height`).
    fn area(self) -> f32;

    /// The larger of `width` and `height`.
    fn max_dimension(self) -> f32;

    /// The smaller of `width` and `height`.
    fn min_dimension(self) -> f32;

    /// Returns `true` if either `width` or `height` is zero.
    fn is_empty(self) -> bool;

    /// Round both dimensions to the nearest device pixel boundary.
    fn snap_to_pixel(self, scale: ScaleFactor) -> Size;
}

impl SizeExt for Size {
    fn area(self) -> f32 {
        self.width * self.height
    }

    fn max_dimension(self) -> f32 {
        self.width.max(self.height)
    }

    fn min_dimension(self) -> f32 {
        self.width.min(self.height)
    }

    fn is_empty(self) -> bool {
        self.width == 0.0 || self.height == 0.0
    }

    fn snap_to_pixel(self, scale: ScaleFactor) -> Size {
        let device = scale.scale_size(self);
        let snapped = DeviceSize::new(device.width.round(), device.height.round());
        scale.unscale_size(snapped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_factor_roundtrip() {
        let scale = ScaleFactor(2.0);
        let original = Point::new(10.0, 20.0);
        let scaled = scale.scale_point(original);
        let back = scale.unscale_point(scaled);
        assert_eq!(original.x, back.x);
        assert_eq!(original.y, back.y);
    }

    #[test]
    fn scale_factor_rect_roundtrip() {
        let scale = ScaleFactor(1.5);
        let original = Rect::new(Point::new(5.0, 10.0), Size::new(100.0, 200.0));
        let scaled = scale.scale_rect(original);
        let back = scale.unscale_rect(scaled);
        assert_eq!(original.origin.x, back.origin.x);
        assert_eq!(original.origin.y, back.origin.y);
        assert_eq!(original.size.width, back.size.width);
        assert_eq!(original.size.height, back.size.height);
    }

    #[test]
    fn edges_sums() {
        let edges = Edges {
            top: 1.0,
            right: 2.0,
            bottom: 3.0,
            left: 4.0,
        };
        assert_eq!(edges.horizontal(), 6.0); // 4 + 2
        assert_eq!(edges.vertical(), 4.0); // 1 + 3
    }

    #[test]
    fn point_ext_snap_to_pixel_2x() {
        // 0.333 * 2 = 0.666 → rounds to 1.0 → / 2 = 0.5
        let scale = ScaleFactor(2.0);
        let snapped = Point::new(0.333, 0.333).snap_to_pixel(scale);
        assert_eq!(snapped.x, 0.5);
        assert_eq!(snapped.y, 0.5);
    }

    #[test]
    fn point_ext_snap_already_on_boundary() {
        // Points already on device-pixel boundaries are unchanged
        let scale = ScaleFactor(2.0);
        let snapped = Point::new(1.0, 2.0).snap_to_pixel(scale);
        assert_eq!(snapped.x, 1.0);
        assert_eq!(snapped.y, 2.0);
    }

    #[test]
    fn point_ext_distance_to_3_4_5() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(3.0, 4.0);
        assert_eq!(a.distance_to(b), 5.0);
    }

    #[test]
    fn point_ext_distance_same_point() {
        let p = Point::new(5.0, 10.0);
        assert_eq!(p.distance_to(p), 0.0);
    }

    #[test]
    fn size_ext_area() {
        assert_eq!(Size::new(4.0, 5.0).area(), 20.0);
    }

    #[test]
    fn size_ext_area_zero_dimension() {
        assert_eq!(Size::new(0.0, 5.0).area(), 0.0);
    }

    #[test]
    fn size_ext_max_min_dimension() {
        let s = Size::new(4.0, 7.0);
        assert_eq!(s.max_dimension(), 7.0);
        assert_eq!(s.min_dimension(), 4.0);
    }

    #[test]
    fn size_ext_is_empty() {
        assert!(Size::new(0.0, 5.0).is_empty());
        assert!(Size::new(3.0, 0.0).is_empty());
        assert!(!Size::new(1.0, 1.0).is_empty());
    }

    #[test]
    fn size_ext_snap_to_pixel_2x() {
        let scale = ScaleFactor(2.0);
        // 0.4 * 2 = 0.8 → rounds to 1.0 → / 2 = 0.5
        // 0.6 * 2 = 1.2 → rounds to 1.0 → / 2 = 0.5
        let s = Size::new(0.4, 0.6).snap_to_pixel(scale);
        assert_eq!(s.width, 0.5);
        assert_eq!(s.height, 0.5);
    }
}
