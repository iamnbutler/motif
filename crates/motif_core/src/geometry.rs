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

/// Shrink a [`Rect`] inward by the given edge amounts.
///
/// The origin moves right and down by the left and top insets; the size shrinks
/// by the sum of the opposite-edge pairs. The resulting dimensions are clamped
/// to `0.0` so the rect cannot become negative.
///
/// **Common use:** compute the inner content bounds of an element after applying
/// padding.
///
/// ```ignore
/// let outer = Rect::new(Point::new(0.0, 0.0), Size::new(100.0, 50.0));
/// let inner = inset_rect(outer, Edges::all(10.0));
/// // inner: origin (10, 10), size (80 × 30)
/// ```
pub fn inset_rect(rect: Rect, edges: Edges<f32>) -> Rect {
    Rect::new(
        Point::new(rect.origin.x + edges.left, rect.origin.y + edges.top),
        Size::new(
            (rect.size.width - edges.horizontal()).max(0.0),
            (rect.size.height - edges.vertical()).max(0.0),
        ),
    )
}

/// Expand a [`Rect`] outward by the given edge amounts.
///
/// The origin moves left and up by the left and top amounts; the size grows by
/// the sum of the opposite-edge pairs.
///
/// **Common use:** compute the outer bounds of a rect when adding borders or
/// drop shadows.
///
/// ```ignore
/// let inner = Rect::new(Point::new(10.0, 10.0), Size::new(80.0, 30.0));
/// let outer = expand_rect(inner, Edges::all(10.0));
/// // outer: origin (0, 0), size (100 × 50)
/// ```
pub fn expand_rect(rect: Rect, edges: Edges<f32>) -> Rect {
    Rect::new(
        Point::new(rect.origin.x - edges.left, rect.origin.y - edges.top),
        Size::new(
            rect.size.width + edges.horizontal(),
            rect.size.height + edges.vertical(),
        ),
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
        let r = Rect::new(Point::new(0.0, 0.0), Size::new(100.0, 50.0));
        let result = inset_rect(r, Edges::all(10.0));
        assert_eq!(result.origin.x, 10.0);
        assert_eq!(result.origin.y, 10.0);
        assert_eq!(result.size.width, 80.0);
        assert_eq!(result.size.height, 30.0);
    }

    #[test]
    fn inset_rect_asymmetric() {
        let r = Rect::new(Point::new(5.0, 10.0), Size::new(200.0, 100.0));
        let edges = Edges {
            top: 5.0,
            right: 20.0,
            bottom: 10.0,
            left: 15.0,
        };
        let result = inset_rect(r, edges);
        assert_eq!(result.origin.x, 20.0); // 5 + 15
        assert_eq!(result.origin.y, 15.0); // 10 + 5
        assert_eq!(result.size.width, 165.0); // 200 - 20 - 15
        assert_eq!(result.size.height, 85.0); // 100 - 5 - 10
    }

    #[test]
    fn inset_rect_clamps_to_zero() {
        let r = Rect::new(Point::new(0.0, 0.0), Size::new(10.0, 10.0));
        let result = inset_rect(r, Edges::all(20.0));
        assert_eq!(result.size.width, 0.0);
        assert_eq!(result.size.height, 0.0);
    }

    #[test]
    fn expand_rect_uniform() {
        let r = Rect::new(Point::new(10.0, 10.0), Size::new(80.0, 30.0));
        let result = expand_rect(r, Edges::all(10.0));
        assert_eq!(result.origin.x, 0.0);
        assert_eq!(result.origin.y, 0.0);
        assert_eq!(result.size.width, 100.0);
        assert_eq!(result.size.height, 50.0);
    }

    #[test]
    fn expand_rect_asymmetric() {
        let r = Rect::new(Point::new(20.0, 15.0), Size::new(165.0, 85.0));
        let edges = Edges {
            top: 5.0,
            right: 20.0,
            bottom: 10.0,
            left: 15.0,
        };
        let result = expand_rect(r, edges);
        assert_eq!(result.origin.x, 5.0); // 20 - 15
        assert_eq!(result.origin.y, 10.0); // 15 - 5
        assert_eq!(result.size.width, 200.0); // 165 + 20 + 15
        assert_eq!(result.size.height, 100.0); // 85 + 5 + 10
    }

    #[test]
    fn inset_expand_roundtrip() {
        let original = Rect::new(Point::new(5.0, 10.0), Size::new(200.0, 100.0));
        let edges = Edges {
            top: 5.0,
            right: 20.0,
            bottom: 10.0,
            left: 15.0,
        };
        let inner = inset_rect(original, edges);
        let outer = expand_rect(inner, edges);
        assert_eq!(outer.origin.x, original.origin.x);
        assert_eq!(outer.origin.y, original.origin.y);
        assert_eq!(outer.size.width, original.size.width);
        assert_eq!(outer.size.height, original.size.height);
    }
}
