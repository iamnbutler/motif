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

impl Edges<f32> {
    /// Returns a new [`Rect`] inset by these edge values.
    ///
    /// Shifts the origin right by `left` and down by `top`, then reduces the
    /// width by [`horizontal()`][Self::horizontal] and the height by
    /// [`vertical()`][Self::vertical]. Width and height clamp to `0.0` if the
    /// insets would invert the rect.
    pub fn inset_rect(&self, rect: Rect) -> Rect {
        let new_width = (rect.size.width - self.horizontal()).max(0.0);
        let new_height = (rect.size.height - self.vertical()).max(0.0);
        Rect::new(
            Point::new(rect.origin.x + self.left, rect.origin.y + self.top),
            Size::new(new_width, new_height),
        )
    }

    /// Returns a new [`Rect`] expanded by these edge values.
    ///
    /// Shifts the origin left by `left` and up by `top`, then increases the
    /// width by [`horizontal()`][Self::horizontal] and the height by
    /// [`vertical()`][Self::vertical].
    pub fn expand_rect(&self, rect: Rect) -> Rect {
        Rect::new(
            Point::new(rect.origin.x - self.left, rect.origin.y - self.top),
            Size::new(
                rect.size.width + self.horizontal(),
                rect.size.height + self.vertical(),
            ),
        )
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
    fn inset_rect_uniform() {
        let edges = Edges::all(10.0_f32);
        let rect = Rect::new(Point::new(0.0, 0.0), Size::new(100.0, 80.0));
        let inset = edges.inset_rect(rect);
        assert_eq!(inset.origin.x, 10.0);
        assert_eq!(inset.origin.y, 10.0);
        assert_eq!(inset.size.width, 80.0);
        assert_eq!(inset.size.height, 60.0);
    }

    #[test]
    fn inset_rect_asymmetric() {
        let edges = Edges {
            top: 5.0_f32,
            right: 10.0,
            bottom: 15.0,
            left: 20.0,
        };
        let rect = Rect::new(Point::new(0.0, 0.0), Size::new(100.0, 80.0));
        let inset = edges.inset_rect(rect);
        assert_eq!(inset.origin.x, 20.0);
        assert_eq!(inset.origin.y, 5.0);
        assert_eq!(inset.size.width, 70.0); // 100 - (20 + 10)
        assert_eq!(inset.size.height, 60.0); // 80 - (5 + 15)
    }

    #[test]
    fn inset_rect_preserves_offset_origin() {
        let edges = Edges::all(5.0_f32);
        let rect = Rect::new(Point::new(50.0, 30.0), Size::new(100.0, 80.0));
        let inset = edges.inset_rect(rect);
        assert_eq!(inset.origin.x, 55.0);
        assert_eq!(inset.origin.y, 35.0);
        assert_eq!(inset.size.width, 90.0);
        assert_eq!(inset.size.height, 70.0);
    }

    #[test]
    fn inset_rect_clamps_to_zero() {
        let edges = Edges::all(60.0_f32);
        let rect = Rect::new(Point::new(0.0, 0.0), Size::new(100.0, 80.0));
        let inset = edges.inset_rect(rect);
        // 100 - 120 < 0  and  80 - 120 < 0 → both clamp to 0
        assert_eq!(inset.size.width, 0.0);
        assert_eq!(inset.size.height, 0.0);
    }

    #[test]
    fn expand_rect_uniform() {
        let edges = Edges::all(10.0_f32);
        let rect = Rect::new(Point::new(10.0, 10.0), Size::new(80.0, 60.0));
        let expanded = edges.expand_rect(rect);
        assert_eq!(expanded.origin.x, 0.0);
        assert_eq!(expanded.origin.y, 0.0);
        assert_eq!(expanded.size.width, 100.0);
        assert_eq!(expanded.size.height, 80.0);
    }

    #[test]
    fn expand_rect_asymmetric() {
        let edges = Edges {
            top: 5.0_f32,
            right: 10.0,
            bottom: 15.0,
            left: 20.0,
        };
        let rect = Rect::new(Point::new(50.0, 30.0), Size::new(100.0, 80.0));
        let expanded = edges.expand_rect(rect);
        assert_eq!(expanded.origin.x, 30.0); // 50 - 20
        assert_eq!(expanded.origin.y, 25.0); // 30 - 5
        assert_eq!(expanded.size.width, 130.0); // 100 + (20 + 10)
        assert_eq!(expanded.size.height, 100.0); // 80 + (5 + 15)
    }

    #[test]
    fn inset_expand_roundtrip() {
        let edges = Edges {
            top: 5.0_f32,
            right: 8.0,
            bottom: 12.0,
            left: 3.0,
        };
        let original = Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 150.0));
        let inset = edges.inset_rect(original);
        let expanded = edges.expand_rect(inset);
        assert!((expanded.origin.x - original.origin.x).abs() < 1e-5);
        assert!((expanded.origin.y - original.origin.y).abs() < 1e-5);
        assert!((expanded.size.width - original.size.width).abs() < 1e-5);
        assert!((expanded.size.height - original.size.height).abs() < 1e-5);
    }
}
