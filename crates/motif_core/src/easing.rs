//! Common easing functions for animations and transitions.
//!
//! Each function maps a normalized time `t ∈ [0.0, 1.0]` to a modified progress
//! value in the same range. Compose with [`lerp`] from the geometry module to
//! produce smooth interpolated values.
//!
//! # Example
//!
//! ```ignore
//! // Animate a position using cubic ease-out:
//! let animated_x = lerp(start_x, end_x, ease_out_cubic(t));
//! ```
//!
//! ## Naming convention
//!
//! - `ease_in_*`: slow start, fast end
//! - `ease_out_*`: fast start, slow end
//! - `ease_in_out_*`: slow start and end, fast middle

/// Linear — no easing; `t` passes through unchanged.
#[inline]
pub fn ease_linear(t: f32) -> f32 {
    t
}

// ── Quadratic ────────────────────────────────────────────────────────────────

/// Ease-in quadratic: slow start, accelerates (t²).
#[inline]
pub fn ease_in_quad(t: f32) -> f32 {
    t * t
}

/// Ease-out quadratic: fast start, decelerates (1-(1-t)²).
#[inline]
pub fn ease_out_quad(t: f32) -> f32 {
    let inv = 1.0 - t;
    1.0 - inv * inv
}

/// Ease-in-out quadratic: slow start and end, fast middle.
#[inline]
pub fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        let inv = -2.0 * t + 2.0;
        1.0 - inv * inv / 2.0
    }
}

// ── Cubic ────────────────────────────────────────────────────────────────────

/// Ease-in cubic: slow start, sharp acceleration (t³).
#[inline]
pub fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

/// Ease-out cubic: fast start, sharp deceleration (1-(1-t)³).
#[inline]
pub fn ease_out_cubic(t: f32) -> f32 {
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

/// Ease-in-out cubic: slow start and end, sharp peak.
#[inline]
pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let inv = -2.0 * t + 2.0;
        1.0 - inv * inv * inv / 2.0
    }
}

// ── Sinusoidal ───────────────────────────────────────────────────────────────

/// Ease-in sine: gentle slow start (based on cos).
#[inline]
pub fn ease_in_sine(t: f32) -> f32 {
    1.0 - (t * std::f32::consts::FRAC_PI_2).cos()
}

/// Ease-out sine: gentle fast start (based on sin).
#[inline]
pub fn ease_out_sine(t: f32) -> f32 {
    (t * std::f32::consts::FRAC_PI_2).sin()
}

/// Ease-in-out sine: smooth S-curve.
#[inline]
pub fn ease_in_out_sine(t: f32) -> f32 {
    -((std::f32::consts::PI * t).cos() - 1.0) / 2.0
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    // Every easing function must map 0.0 → 0.0 and 1.0 → 1.0.
    macro_rules! test_boundaries {
        ($name:ident, $fn:ident) => {
            #[test]
            fn $name() {
                assert!(
                    approx_eq($fn(0.0), 0.0),
                    "{} at t=0 should be 0",
                    stringify!($fn)
                );
                assert!(
                    approx_eq($fn(1.0), 1.0),
                    "{} at t=1 should be 1",
                    stringify!($fn)
                );
            }
        };
    }

    test_boundaries!(linear_boundaries, ease_linear);
    test_boundaries!(in_quad_boundaries, ease_in_quad);
    test_boundaries!(out_quad_boundaries, ease_out_quad);
    test_boundaries!(in_out_quad_boundaries, ease_in_out_quad);
    test_boundaries!(in_cubic_boundaries, ease_in_cubic);
    test_boundaries!(out_cubic_boundaries, ease_out_cubic);
    test_boundaries!(in_out_cubic_boundaries, ease_in_out_cubic);
    test_boundaries!(in_sine_boundaries, ease_in_sine);
    test_boundaries!(out_sine_boundaries, ease_out_sine);
    test_boundaries!(in_out_sine_boundaries, ease_in_out_sine);

    // Midpoint checks — in-* is slow start (value < 0.5 at t=0.5),
    // out-* is fast start (value > 0.5 at t=0.5),
    // in-out-* is symmetric (value == 0.5 at t=0.5).

    #[test]
    fn in_quad_midpoint_slow_start() {
        // 0.5² = 0.25 — progress lags behind time
        assert!(approx_eq(ease_in_quad(0.5), 0.25));
    }

    #[test]
    fn out_quad_midpoint_fast_start() {
        // 1-(0.5)² = 0.75 — progress leads time
        assert!(approx_eq(ease_out_quad(0.5), 0.75));
    }

    #[test]
    fn in_out_quad_midpoint_symmetric() {
        assert!(approx_eq(ease_in_out_quad(0.5), 0.5));
    }

    #[test]
    fn in_cubic_midpoint_very_slow_start() {
        // 0.5³ = 0.125
        assert!(approx_eq(ease_in_cubic(0.5), 0.125));
    }

    #[test]
    fn out_cubic_midpoint_very_fast_start() {
        // 1-(0.5)³ = 0.875
        assert!(approx_eq(ease_out_cubic(0.5), 0.875));
    }

    #[test]
    fn in_out_cubic_midpoint_symmetric() {
        assert!(approx_eq(ease_in_out_cubic(0.5), 0.5));
    }

    #[test]
    fn in_out_sine_midpoint_symmetric() {
        // -(cos(π/2) - 1) / 2 = -(-1)/2 = 0.5
        assert!(approx_eq(ease_in_out_sine(0.5), 0.5));
    }

    #[test]
    fn in_sine_slower_than_linear() {
        // ease-in should be below the diagonal at t=0.5
        assert!(ease_in_sine(0.5) < 0.5);
    }

    #[test]
    fn out_sine_faster_than_linear() {
        // ease-out should be above the diagonal at t=0.5
        assert!(ease_out_sine(0.5) > 0.5);
    }
}
