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

/// Linear interpolation between two `f32` values.
///
/// Returns `a` when `t = 0.0`, `b` when `t = 1.0`, and a blend in between.
/// `t` is not clamped — callers may pass values outside `[0.0, 1.0]` to
/// extrapolate.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Linearly interpolate between two [`Point`] values component-wise.
pub fn lerp_point(a: Point, b: Point, t: f32) -> Point {
    Point::new(lerp(a.x, b.x, t), lerp(a.y, b.y, t))
}

/// Linearly interpolate between two [`DevicePoint`] values component-wise.
pub fn lerp_device_point(a: DevicePoint, b: DevicePoint, t: f32) -> DevicePoint {
    DevicePoint::new(lerp(a.x, b.x, t), lerp(a.y, b.y, t))
}

/// Linearly interpolate between two [`Size`] values component-wise.
pub fn lerp_size(a: Size, b: Size, t: f32) -> Size {
    Size::new(lerp(a.width, b.width, t), lerp(a.height, b.height, t))
}

/// Linearly interpolate between two [`DeviceSize`] values component-wise.
pub fn lerp_device_size(a: DeviceSize, b: DeviceSize, t: f32) -> DeviceSize {
    DeviceSize::new(lerp(a.width, b.width, t), lerp(a.height, b.height, t))
}

/// Linearly interpolate between two [`Rect`] values (origin and size both lerped).
pub fn lerp_rect(a: Rect, b: Rect, t: f32) -> Rect {
    Rect::new(
        lerp_point(a.origin, b.origin, t),
        lerp_size(a.size, b.size, t),
    )
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
    fn lerp_midpoint() {
        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
    }

    #[test]
    fn lerp_endpoints() {
        assert_eq!(lerp(3.0, 7.0, 0.0), 3.0);
        assert_eq!(lerp(3.0, 7.0, 1.0), 7.0);
    }

    #[test]
    fn lerp_point_midpoint() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(10.0, 20.0);
        let mid = lerp_point(a, b, 0.5);
        assert_eq!(mid.x, 5.0);
        assert_eq!(mid.y, 10.0);
    }

    #[test]
    fn lerp_point_at_endpoints() {
        let a = Point::new(1.0, 2.0);
        let b = Point::new(9.0, 8.0);
        let at_zero = lerp_point(a, b, 0.0);
        let at_one = lerp_point(a, b, 1.0);
        assert_eq!(at_zero.x, 1.0);
        assert_eq!(at_zero.y, 2.0);
        assert_eq!(at_one.x, 9.0);
        assert_eq!(at_one.y, 8.0);
    }

    #[test]
    fn lerp_device_point_midpoint() {
        let a = DevicePoint::new(0.0, 0.0);
        let b = DevicePoint::new(4.0, 8.0);
        let mid = lerp_device_point(a, b, 0.5);
        assert_eq!(mid.x, 2.0);
        assert_eq!(mid.y, 4.0);
    }

    #[test]
    fn lerp_size_midpoint() {
        let a = Size::new(0.0, 0.0);
        let b = Size::new(100.0, 200.0);
        let mid = lerp_size(a, b, 0.5);
        assert_eq!(mid.width, 50.0);
        assert_eq!(mid.height, 100.0);
    }

    #[test]
    fn lerp_device_size_midpoint() {
        let a = DeviceSize::new(10.0, 20.0);
        let b = DeviceSize::new(30.0, 60.0);
        let mid = lerp_device_size(a, b, 0.5);
        assert_eq!(mid.width, 20.0);
        assert_eq!(mid.height, 40.0);
    }

    #[test]
    fn lerp_rect_midpoint() {
        let a = Rect::new(Point::new(0.0, 0.0), Size::new(100.0, 100.0));
        let b = Rect::new(Point::new(100.0, 100.0), Size::new(200.0, 200.0));
        let mid = lerp_rect(a, b, 0.5);
        assert_eq!(mid.origin.x, 50.0);
        assert_eq!(mid.origin.y, 50.0);
        assert_eq!(mid.size.width, 150.0);
        assert_eq!(mid.size.height, 150.0);
    }

    #[test]
    fn lerp_extrapolation() {
        // t outside [0, 1] extrapolates beyond the range
        assert_eq!(lerp(0.0, 10.0, 2.0), 20.0);
        assert_eq!(lerp(0.0, 10.0, -1.0), -10.0);
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
}
