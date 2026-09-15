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

impl<T: std::ops::Add<Output = T>> std::ops::Add for Edges<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            top: self.top + rhs.top,
            right: self.right + rhs.right,
            bottom: self.bottom + rhs.bottom,
            left: self.left + rhs.left,
        }
    }
}

impl<T: std::ops::Sub<Output = T>> std::ops::Sub for Edges<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            top: self.top - rhs.top,
            right: self.right - rhs.right,
            bottom: self.bottom - rhs.bottom,
            left: self.left - rhs.left,
        }
    }
}

impl<T, S> std::ops::Mul<S> for Edges<T>
where
    T: std::ops::Mul<S, Output = T>,
    S: Copy,
{
    type Output = Self;
    fn mul(self, rhs: S) -> Self {
        Self {
            top: self.top * rhs,
            right: self.right * rhs,
            bottom: self.bottom * rhs,
            left: self.left * rhs,
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

impl<T: std::ops::Add<Output = T>> std::ops::Add for Corners<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            top_left: self.top_left + rhs.top_left,
            top_right: self.top_right + rhs.top_right,
            bottom_right: self.bottom_right + rhs.bottom_right,
            bottom_left: self.bottom_left + rhs.bottom_left,
        }
    }
}

impl<T: std::ops::Sub<Output = T>> std::ops::Sub for Corners<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            top_left: self.top_left - rhs.top_left,
            top_right: self.top_right - rhs.top_right,
            bottom_right: self.bottom_right - rhs.bottom_right,
            bottom_left: self.bottom_left - rhs.bottom_left,
        }
    }
}

impl<T, S> std::ops::Mul<S> for Corners<T>
where
    T: std::ops::Mul<S, Output = T>,
    S: Copy,
{
    type Output = Self;
    fn mul(self, rhs: S) -> Self {
        Self {
            top_left: self.top_left * rhs,
            top_right: self.top_right * rhs,
            bottom_right: self.bottom_right * rhs,
            bottom_left: self.bottom_left * rhs,
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
    fn edges_add() {
        let a = Edges {
            top: 1.0_f32,
            right: 2.0,
            bottom: 3.0,
            left: 4.0,
        };
        let b = Edges {
            top: 10.0,
            right: 20.0,
            bottom: 30.0,
            left: 40.0,
        };
        let c = a + b;
        assert_eq!(
            c,
            Edges {
                top: 11.0,
                right: 22.0,
                bottom: 33.0,
                left: 44.0
            }
        );
    }

    #[test]
    fn edges_sub() {
        let a = Edges {
            top: 10.0_f32,
            right: 20.0,
            bottom: 30.0,
            left: 40.0,
        };
        let b = Edges {
            top: 1.0,
            right: 2.0,
            bottom: 3.0,
            left: 4.0,
        };
        let c = a - b;
        assert_eq!(
            c,
            Edges {
                top: 9.0,
                right: 18.0,
                bottom: 27.0,
                left: 36.0
            }
        );
    }

    #[test]
    fn edges_mul_scalar() {
        let a = Edges {
            top: 1.0_f32,
            right: 2.0,
            bottom: 3.0,
            left: 4.0,
        };
        let b = a * 2.0_f32;
        assert_eq!(
            b,
            Edges {
                top: 2.0,
                right: 4.0,
                bottom: 6.0,
                left: 8.0
            }
        );
    }

    #[test]
    fn corners_add() {
        let a = Corners {
            top_left: 1.0_f32,
            top_right: 2.0,
            bottom_right: 3.0,
            bottom_left: 4.0,
        };
        let b = Corners {
            top_left: 10.0,
            top_right: 20.0,
            bottom_right: 30.0,
            bottom_left: 40.0,
        };
        let c = a + b;
        assert_eq!(
            c,
            Corners {
                top_left: 11.0,
                top_right: 22.0,
                bottom_right: 33.0,
                bottom_left: 44.0
            }
        );
    }

    #[test]
    fn corners_sub() {
        let a = Corners {
            top_left: 10.0_f32,
            top_right: 20.0,
            bottom_right: 30.0,
            bottom_left: 40.0,
        };
        let b = Corners {
            top_left: 1.0,
            top_right: 2.0,
            bottom_right: 3.0,
            bottom_left: 4.0,
        };
        let c = a - b;
        assert_eq!(
            c,
            Corners {
                top_left: 9.0,
                top_right: 18.0,
                bottom_right: 27.0,
                bottom_left: 36.0
            }
        );
    }

    #[test]
    fn corners_mul_scalar() {
        let a = Corners {
            top_left: 1.0_f32,
            top_right: 2.0,
            bottom_right: 3.0,
            bottom_left: 4.0,
        };
        let b = a * 3.0_f32;
        assert_eq!(
            b,
            Corners {
                top_left: 3.0,
                top_right: 6.0,
                bottom_right: 9.0,
                bottom_left: 12.0
            }
        );
    }
}
