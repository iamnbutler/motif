//! Animation primitives for the Motif UI framework.
//!
//! Provides an [`EasingFn`] type, common easing functions, a [`Lerp`] trait
//! for smooth value interpolation, and a [`Tween<T>`] animator that drives a
//! value between two endpoints over a fixed [`Duration`].
//!
//! # Quick Start
//!
//! ```no_run
//! use std::time::Duration;
//! use motif_core::animation::{Tween, ease_out_cubic};
//!
//! // Slide opacity from 0.0 → 1.0 over 300 ms with ease-out-cubic
//! let tween = Tween::new(0.0f32, 1.0f32, Duration::from_millis(300), ease_out_cubic);
//!
//! // In your render loop, query the current value given elapsed time
//! let opacity: f32 = tween.value_at(Duration::from_millis(150));
//! ```
//!
//! # Design
//!
//! [`Tween<T>`] is a **pure value calculator** — it holds no mutable state and
//! does not track wall-clock time itself.  The caller passes an `elapsed`
//! [`Duration`] and receives the interpolated value.  This makes tweens easy
//! to test, cache, and compose into higher-level animation drivers without
//! hidden side-effects.

use std::time::Duration;

// ── EasingFn ──────────────────────────────────────────────────────────────────

/// A normalised easing function: maps `t` ∈ \[0.0, 1.0\] to a progress value.
///
/// The output is typically in \[0.0, 1.0\] but may exceed this range for
/// overshoot effects such as elastic or back easings.
pub type EasingFn = fn(f64) -> f64;

// ── Built-in easing functions ─────────────────────────────────────────────────

/// Constant velocity — no acceleration or deceleration.
pub fn linear(t: f64) -> f64 {
    t
}

/// Quadratic ease-in — accelerates from zero velocity.
pub fn ease_in_quad(t: f64) -> f64 {
    t * t
}

/// Quadratic ease-out — decelerates to zero velocity.
pub fn ease_out_quad(t: f64) -> f64 {
    t * (2.0 - t)
}

/// Quadratic ease-in-out — accelerates then decelerates.
pub fn ease_in_out_quad(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

/// Cubic ease-in — stronger acceleration than quadratic.
pub fn ease_in_cubic(t: f64) -> f64 {
    t * t * t
}

/// Cubic ease-out — stronger deceleration than quadratic.
pub fn ease_out_cubic(t: f64) -> f64 {
    let u = t - 1.0;
    u * u * u + 1.0
}

/// Cubic ease-in-out.
pub fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let u = 2.0 * t - 2.0;
        0.5 * u * u * u + 1.0
    }
}

/// Sinusoidal ease-in — gentle, smooth acceleration.
pub fn ease_in_sine(t: f64) -> f64 {
    1.0 - (t * std::f64::consts::FRAC_PI_2).cos()
}

/// Sinusoidal ease-out — gentle, smooth deceleration.
pub fn ease_out_sine(t: f64) -> f64 {
    (t * std::f64::consts::FRAC_PI_2).sin()
}

/// Sinusoidal ease-in-out.
pub fn ease_in_out_sine(t: f64) -> f64 {
    -((std::f64::consts::PI * t).cos() - 1.0) / 2.0
}

/// Exponential ease-in — dramatic acceleration from near zero.
pub fn ease_in_expo(t: f64) -> f64 {
    if t == 0.0 {
        0.0
    } else {
        (2.0_f64).powf(10.0 * t - 10.0)
    }
}

/// Exponential ease-out — dramatic deceleration to near zero.
pub fn ease_out_expo(t: f64) -> f64 {
    if t == 1.0 {
        1.0
    } else {
        1.0 - (2.0_f64).powf(-10.0 * t)
    }
}

/// Exponential ease-in-out.
pub fn ease_in_out_expo(t: f64) -> f64 {
    if t == 0.0 {
        return 0.0;
    }
    if t == 1.0 {
        return 1.0;
    }
    if t < 0.5 {
        (2.0_f64).powf(20.0 * t - 10.0) / 2.0
    } else {
        (2.0 - (2.0_f64).powf(-20.0 * t + 10.0)) / 2.0
    }
}

// ── Lerp ──────────────────────────────────────────────────────────────────────

/// Linear interpolation between two values of the same type.
///
/// `t` is a normalised factor in \[0.0, 1.0\].  The contract is:
/// - `lerp(self, other, 0.0)` == `self`
/// - `lerp(self, other, 1.0)` == `other`
pub trait Lerp: Sized {
    /// Returns the value at normalised time `t` between `self` and `other`.
    fn lerp(self, other: Self, t: f64) -> Self;
}

impl Lerp for f32 {
    fn lerp(self, other: Self, t: f64) -> Self {
        self + (other - self) * t as f32
    }
}

impl Lerp for f64 {
    fn lerp(self, other: Self, t: f64) -> Self {
        self + (other - self) * t
    }
}

// ── Tween ─────────────────────────────────────────────────────────────────────

/// A stateless value animator from `start` to `end` over `duration`.
///
/// Pass an `elapsed` [`Duration`] to [`value_at`](Tween::value_at) and
/// receive the current interpolated value.  `Tween` does not track time
/// internally — that is the responsibility of the caller.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// use motif_core::animation::{Tween, ease_out_cubic};
///
/// let tween = Tween::new(0.0f32, 200.0f32, Duration::from_millis(400), ease_out_cubic);
///
/// assert_eq!(tween.value_at(Duration::ZERO), 0.0);
/// assert_eq!(tween.value_at(Duration::from_millis(400)), 200.0);
/// assert!(tween.is_complete(Duration::from_millis(400)));
/// ```
#[derive(Clone, Debug)]
pub struct Tween<T: Lerp + Clone> {
    start: T,
    end: T,
    duration: Duration,
    easing: EasingFn,
}

impl<T: Lerp + Clone> Tween<T> {
    /// Creates a new tween that moves from `start` to `end` over `duration`
    /// using `easing` to shape the progress curve.
    pub fn new(start: T, end: T, duration: Duration, easing: EasingFn) -> Self {
        Self {
            start,
            end,
            duration,
            easing,
        }
    }

    /// Returns the interpolated value at `elapsed` time.
    ///
    /// - If `elapsed` is zero, returns `start`.
    /// - If `elapsed >= duration` (or `duration` is zero), returns `end`.
    /// - Otherwise applies the easing function to the normalised time.
    pub fn value_at(&self, elapsed: Duration) -> T {
        if self.duration.is_zero() || elapsed >= self.duration {
            return self.end.clone();
        }
        if elapsed.is_zero() {
            return self.start.clone();
        }
        let t = elapsed.as_secs_f64() / self.duration.as_secs_f64();
        let eased = (self.easing)(t.clamp(0.0, 1.0));
        self.start.clone().lerp(self.end.clone(), eased)
    }

    /// Returns `true` once `elapsed >= duration`.
    pub fn is_complete(&self, elapsed: Duration) -> bool {
        elapsed >= self.duration
    }

    /// Returns a new tween that runs in reverse (`end` → `start`), with the
    /// same duration and easing.
    pub fn reversed(self) -> Self {
        Self {
            start: self.end,
            end: self.start,
            duration: self.duration,
            easing: self.easing,
        }
    }

    /// Returns the total duration of this tween.
    pub fn duration(&self) -> Duration {
        self.duration
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── linear ────────────────────────────────────────────────────────────────

    #[test]
    fn linear_at_zero() {
        assert_eq!(linear(0.0), 0.0);
    }

    #[test]
    fn linear_at_one() {
        assert_eq!(linear(1.0), 1.0);
    }

    #[test]
    fn linear_at_midpoint() {
        assert!((linear(0.5) - 0.5).abs() < 1e-12);
    }

    // ── ease_in_quad / ease_out_quad ──────────────────────────────────────────

    #[test]
    fn ease_in_quad_slower_than_linear_at_midpoint() {
        // Ease-in starts slow → should be below linear at t=0.5
        assert!(ease_in_quad(0.5) < 0.5);
    }

    #[test]
    fn ease_out_quad_faster_than_linear_at_midpoint() {
        // Ease-out starts fast → should be above linear at t=0.5
        assert!(ease_out_quad(0.5) > 0.5);
    }

    #[test]
    fn ease_in_out_quad_symmetric() {
        let a = ease_in_out_quad(0.25);
        let b = 1.0 - ease_in_out_quad(0.75);
        assert!((a - b).abs() < 1e-12);
    }

    // ── ease_out_cubic ────────────────────────────────────────────────────────

    #[test]
    fn ease_out_cubic_boundary_zero() {
        assert!((ease_out_cubic(0.0) - 0.0).abs() < 1e-12);
    }

    #[test]
    fn ease_out_cubic_boundary_one() {
        assert!((ease_out_cubic(1.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn ease_out_cubic_ahead_of_linear_at_midpoint() {
        assert!(ease_out_cubic(0.5) > 0.5);
    }

    // ── ease_in_out_cubic ─────────────────────────────────────────────────────

    #[test]
    fn ease_in_out_cubic_symmetric() {
        let a = ease_in_out_cubic(0.25);
        let b = 1.0 - ease_in_out_cubic(0.75);
        assert!((a - b).abs() < 1e-12);
    }

    // ── expo easings ──────────────────────────────────────────────────────────

    #[test]
    fn ease_in_expo_at_zero() {
        assert_eq!(ease_in_expo(0.0), 0.0);
    }

    #[test]
    fn ease_out_expo_at_one() {
        assert_eq!(ease_out_expo(1.0), 1.0);
    }

    #[test]
    fn ease_in_out_expo_at_zero_and_one() {
        assert_eq!(ease_in_out_expo(0.0), 0.0);
        assert_eq!(ease_in_out_expo(1.0), 1.0);
    }

    // ── Lerp ─────────────────────────────────────────────────────────────────

    #[test]
    fn lerp_f32_at_zero_returns_start() {
        assert!((0.0f32.lerp(100.0, 0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn lerp_f32_at_one_returns_end() {
        assert!((0.0f32.lerp(100.0, 1.0) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn lerp_f32_midpoint() {
        assert!((0.0f32.lerp(100.0, 0.5) - 50.0).abs() < 1e-4);
    }

    #[test]
    fn lerp_f64_midpoint() {
        assert!((10.0f64.lerp(20.0, 0.5) - 15.0).abs() < 1e-12);
    }

    // ── Tween ─────────────────────────────────────────────────────────────────

    #[test]
    fn tween_at_zero_returns_start() {
        let t = Tween::new(0.0f32, 100.0, Duration::from_millis(200), linear);
        assert!((t.value_at(Duration::ZERO) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn tween_at_duration_returns_end() {
        let t = Tween::new(0.0f32, 100.0, Duration::from_millis(200), linear);
        assert!((t.value_at(Duration::from_millis(200)) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn tween_past_duration_clamps_to_end() {
        let t = Tween::new(0.0f32, 100.0, Duration::from_millis(200), linear);
        assert!((t.value_at(Duration::from_millis(999)) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn tween_midpoint_with_linear_easing() {
        let t = Tween::new(0.0f32, 100.0, Duration::from_millis(200), linear);
        assert!((t.value_at(Duration::from_millis(100)) - 50.0).abs() < 1e-4);
    }

    #[test]
    fn tween_not_complete_before_duration() {
        let t = Tween::new(0.0f32, 100.0, Duration::from_millis(200), linear);
        assert!(!t.is_complete(Duration::from_millis(199)));
    }

    #[test]
    fn tween_complete_at_duration() {
        let t = Tween::new(0.0f32, 100.0, Duration::from_millis(200), linear);
        assert!(t.is_complete(Duration::from_millis(200)));
    }

    #[test]
    fn tween_reversed_swaps_endpoints() {
        let t = Tween::new(0.0f32, 100.0, Duration::from_millis(200), linear);
        let rev = t.reversed();
        assert!((rev.value_at(Duration::ZERO) - 100.0).abs() < 1e-6);
        assert!((rev.value_at(Duration::from_millis(200)) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn tween_zero_duration_returns_end() {
        let t = Tween::new(0.0f32, 100.0, Duration::ZERO, linear);
        assert!((t.value_at(Duration::ZERO) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn tween_ease_out_cubic_ahead_of_linear_at_midpoint() {
        let eased = Tween::new(0.0f32, 100.0, Duration::from_millis(200), ease_out_cubic);
        let lin = Tween::new(0.0f32, 100.0, Duration::from_millis(200), linear);
        let mid = Duration::from_millis(100);
        // ease-out is fast then slow → should be ahead of linear at midpoint
        assert!(eased.value_at(mid) > lin.value_at(mid));
    }

    #[test]
    fn tween_duration_accessor() {
        let t = Tween::new(0.0f32, 1.0, Duration::from_millis(300), linear);
        assert_eq!(t.duration(), Duration::from_millis(300));
    }
}
