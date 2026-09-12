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

impl<T> Edges<T> {
    /// Transform each edge value with a function, returning a new `Edges<U>`.
    ///
    /// Useful for unit conversions, clamping, scaling, or type transformations.
    ///
    /// # Example
    ///
    /// Scale logical padding to device pixels:
    /// ```
    /// # use motif_core::Edges;
    /// let logical = Edges::all(8.0_f32);
    /// let device = logical.map(|v| v * 2.0);
    /// assert_eq!(device, Edges::all(16.0));
    /// ```
    pub fn map<U>(self, f: impl Fn(T) -> U) -> Edges<U> {
        Edges {
            top: f(self.top),
            right: f(self.right),
            bottom: f(self.bottom),
            left: f(self.left),
        }
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

impl<T> Corners<T> {
    /// Transform each corner value with a function, returning a new `Corners<U>`.
    ///
    /// Useful for clamping radii, scaling, or type conversions.
    ///
    /// # Example
    ///
    /// Clamp all corner radii to a maximum:
    /// ```
    /// # use motif_core::Corners;
    /// let radii = Corners::all(20.0_f32);
    /// let clamped = radii.map(|r| r.min(10.0));
    /// assert_eq!(clamped, Corners::all(10.0));
    /// ```
    pub fn map<U>(self, f: impl Fn(T) -> U) -> Corners<U> {
        Corners {
            top_left: f(self.top_left),
            top_right: f(self.top_right),
            bottom_right: f(self.bottom_right),
            bottom_left: f(self.bottom_left),
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
    fn edges_map_scales_values() {
        let edges = Edges {
            top: 1.0_f32,
            right: 2.0,
            bottom: 3.0,
            left: 4.0,
        };
        let doubled = edges.map(|v| v * 2.0);
        assert_eq!(
            doubled,
            Edges {
                top: 2.0,
                right: 4.0,
                bottom: 6.0,
                left: 8.0,
            }
        );
    }

    #[test]
    fn edges_map_type_conversion() {
        let edges = Edges {
            top: 1_u32,
            right: 2,
            bottom: 3,
            left: 4,
        };
        let floats = edges.map(|v| v as f32);
        assert_eq!(
            floats,
            Edges {
                top: 1.0,
                right: 2.0,
                bottom: 3.0,
                left: 4.0,
            }
        );
    }

    #[test]
    fn edges_map_uniform() {
        let padding = Edges::all(8.0_f32);
        let device = padding.map(|v| v * 2.0);
        assert_eq!(device, Edges::all(16.0));
    }

    #[test]
    fn corners_map_scales_values() {
        let corners = Corners {
            top_left: 4.0_f32,
            top_right: 8.0,
            bottom_right: 12.0,
            bottom_left: 16.0,
        };
        let halved = corners.map(|v| v / 2.0);
        assert_eq!(
            halved,
            Corners {
                top_left: 2.0,
                top_right: 4.0,
                bottom_right: 6.0,
                bottom_left: 8.0,
            }
        );
    }

    #[test]
    fn corners_map_clamping() {
        let corners = Corners::all(20.0_f32);
        let clamped = corners.map(|r| r.min(10.0));
        assert_eq!(clamped, Corners::all(10.0));
    }

    #[test]
    fn corners_map_uniform() {
        let radii = Corners::all(4.0_f32);
        let doubled = radii.map(|r| r * 2.0);
        assert_eq!(doubled, Corners::all(8.0));
    }
}
