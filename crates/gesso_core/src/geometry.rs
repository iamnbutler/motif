//! Core geometry primitives for gesso.

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

#[cfg(test)]
mod tests {
    use super::*;
    use glamour::Unit;

    #[test]
    fn logical_pixels_is_f32_unit() {
        // LogicalPixels should implement Unit with f32 scalar
        fn assert_unit<U: Unit<Scalar = f32>>() {}
        assert_unit::<LogicalPixels>();
    }

    #[test]
    fn device_pixels_is_f32_unit() {
        fn assert_unit<U: Unit<Scalar = f32>>() {}
        assert_unit::<DevicePixels>();
    }
}
