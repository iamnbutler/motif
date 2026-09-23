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

    /// Get the main-axis dimension of a [`Size`].
    ///
    /// Returns `width` for horizontal, `height` for vertical.
    /// Useful in flex layout: `axis.main_size(container)` gives the sizing
    /// dimension without a match expression at every call site.
    pub fn main_size(self, size: Size) -> f32 {
        match self {
            Axis::Horizontal => size.width,
            Axis::Vertical => size.height,
        }
    }

    /// Get the cross-axis dimension of a [`Size`].
    ///
    /// Returns `height` for horizontal, `width` for vertical.
    pub fn cross_size(self, size: Size) -> f32 {
        match self {
            Axis::Horizontal => size.height,
            Axis::Vertical => size.width,
        }
    }

    /// Get the main-axis coordinate of a [`Point`].
    ///
    /// Returns `x` for horizontal, `y` for vertical.
    pub fn main_coord(self, point: Point) -> f32 {
        match self {
            Axis::Horizontal => point.x,
            Axis::Vertical => point.y,
        }
    }

    /// Get the cross-axis coordinate of a [`Point`].
    ///
    /// Returns `y` for horizontal, `x` for vertical.
    pub fn cross_coord(self, point: Point) -> f32 {
        match self {
            Axis::Horizontal => point.y,
            Axis::Vertical => point.x,
        }
    }

    /// Construct a [`Size`] from main-axis and cross-axis values.
    ///
    /// For horizontal: `Size::new(main, cross)`.
    /// For vertical: `Size::new(cross, main)`.
    pub fn make_size(self, main: f32, cross: f32) -> Size {
        match self {
            Axis::Horizontal => Size::new(main, cross),
            Axis::Vertical => Size::new(cross, main),
        }
    }

    /// Construct a [`Point`] from main-axis and cross-axis values.
    ///
    /// For horizontal: `Point::new(main, cross)`.
    /// For vertical: `Point::new(cross, main)`.
    pub fn make_point(self, main: f32, cross: f32) -> Point {
        match self {
            Axis::Horizontal => Point::new(main, cross),
            Axis::Vertical => Point::new(cross, main),
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
    fn axis_main_size_horizontal() {
        let size = Size::new(100.0, 50.0);
        assert_eq!(Axis::Horizontal.main_size(size), 100.0);
        assert_eq!(Axis::Vertical.main_size(size), 50.0);
    }

    #[test]
    fn axis_cross_size_horizontal() {
        let size = Size::new(100.0, 50.0);
        assert_eq!(Axis::Horizontal.cross_size(size), 50.0);
        assert_eq!(Axis::Vertical.cross_size(size), 100.0);
    }

    #[test]
    fn axis_main_cross_size_sum() {
        // main + cross should cover both dimensions regardless of axis
        let size = Size::new(80.0, 40.0);
        let h = Axis::Horizontal;
        assert_eq!(h.main_size(size) + h.cross_size(size), 120.0);
        let v = Axis::Vertical;
        assert_eq!(v.main_size(size) + v.cross_size(size), 120.0);
    }

    #[test]
    fn axis_main_coord_horizontal() {
        let point = Point::new(10.0, 20.0);
        assert_eq!(Axis::Horizontal.main_coord(point), 10.0);
        assert_eq!(Axis::Vertical.main_coord(point), 20.0);
    }

    #[test]
    fn axis_cross_coord_horizontal() {
        let point = Point::new(10.0, 20.0);
        assert_eq!(Axis::Horizontal.cross_coord(point), 20.0);
        assert_eq!(Axis::Vertical.cross_coord(point), 10.0);
    }

    #[test]
    fn axis_make_size_roundtrip() {
        let size = Size::new(300.0, 150.0);
        let h = Axis::Horizontal;
        let reconstructed = h.make_size(h.main_size(size), h.cross_size(size));
        assert_eq!(reconstructed.width, size.width);
        assert_eq!(reconstructed.height, size.height);

        let v = Axis::Vertical;
        let reconstructed_v = v.make_size(v.main_size(size), v.cross_size(size));
        assert_eq!(reconstructed_v.width, size.width);
        assert_eq!(reconstructed_v.height, size.height);
    }

    #[test]
    fn axis_make_point_roundtrip() {
        let point = Point::new(5.0, 15.0);
        let h = Axis::Horizontal;
        let reconstructed = h.make_point(h.main_coord(point), h.cross_coord(point));
        assert_eq!(reconstructed.x, point.x);
        assert_eq!(reconstructed.y, point.y);

        let v = Axis::Vertical;
        let reconstructed_v = v.make_point(v.main_coord(point), v.cross_coord(point));
        assert_eq!(reconstructed_v.x, point.x);
        assert_eq!(reconstructed_v.y, point.y);
    }

    #[test]
    fn axis_make_size_vertical_swaps() {
        // make_size(main=100, cross=50) on Vertical => Size::new(50, 100)
        let size = Axis::Vertical.make_size(100.0, 50.0);
        assert_eq!(size.width, 50.0);
        assert_eq!(size.height, 100.0);
    }
}
