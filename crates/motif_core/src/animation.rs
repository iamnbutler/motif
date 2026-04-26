//! Animation primitives for time-bounded progress tracking.
//!
//! Provides [`Animation`], a lightweight struct that tracks elapsed time
//! against a fixed duration, returning a normalized progress value in
//! `0.0..=1.0`. Combine with the included easing functions to produce
//! smooth, non-linear motion.
//!
//! # Example
//!
//! ```rust,no_run
//! use std::time::Duration;
//! use motif_core::animation::{Animation, ease_in_out};
//!
//! // Start a 300 ms fade-in.
//! let anim = Animation::new(Duration::from_millis(300));
//!
//! // In your paint loop:
//! let alpha = ease_in_out(anim.progress());
//! // Use alpha to set opacity, scale, color, etc.
//! ```

use std::time::{Duration, Instant};

// ============================================================================
// Animation
// ============================================================================

/// A time-bounded animation that tracks progress from `0.0` to `1.0`.
///
/// Create with [`Animation::new`] (starts at the current instant) or
/// [`Animation::with_start`] (explicit start time, useful for testing).
///
/// [`Animation::progress`] returns a clamped linear `t` value. Pass it
/// through an easing function ([`linear`], [`ease_in`], [`ease_out`],
/// [`ease_in_out`]) to obtain a curved output.
#[derive(Debug, Clone)]
pub struct Animation {
    start: Instant,
    duration: Duration,
}

impl Animation {
    /// Create a new animation that starts immediately.
    pub fn new(duration: Duration) -> Self {
        Self {
            start: Instant::now(),
            duration,
        }
    }

    /// Create an animation with an explicit start time.
    ///
    /// Useful in tests where you need deterministic progress values.
    pub fn with_start(start: Instant, duration: Duration) -> Self {
        Self { start, duration }
    }

    /// Linear progress in `0.0..=1.0`. Clamped to `1.0` after completion.
    ///
    /// A zero-duration animation always returns `1.0`.
    pub fn progress(&self) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        let elapsed = self.start.elapsed();
        (elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0)
    }

    /// How long the animation has been running, capped at `duration`.
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed().min(self.duration)
    }

    /// How much time remains before the animation completes.
    ///
    /// Returns [`Duration::ZERO`] once the animation has finished.
    pub fn remaining(&self) -> Duration {
        self.duration.saturating_sub(self.start.elapsed())
    }

    /// Returns `true` if the animation has reached or passed its duration.
    pub fn is_finished(&self) -> bool {
        self.start.elapsed() >= self.duration
    }

    /// Restart the animation from the current instant.
    pub fn restart(&mut self) {
        self.start = Instant::now();
    }
}

// ============================================================================
// Easing functions
// ============================================================================

/// Linear easing — passes `t` through unchanged.
///
/// `f(0.0) = 0.0`, `f(1.0) = 1.0`
#[inline]
pub fn linear(t: f32) -> f32 {
    t
}

/// Ease-in: slow start, fast end. Uses a cubic curve.
///
/// `f(0.0) = 0.0`, `f(0.5) ≈ 0.125`, `f(1.0) = 1.0`
#[inline]
pub fn ease_in(t: f32) -> f32 {
    t * t * t
}

/// Ease-out: fast start, slow end. Uses a cubic curve.
///
/// `f(0.0) = 0.0`, `f(0.5) ≈ 0.875`, `f(1.0) = 1.0`
#[inline]
pub fn ease_out(t: f32) -> f32 {
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

/// Ease-in-out: slow start and end, fast middle. Smooth cubic curve.
///
/// `f(0.0) = 0.0`, `f(0.5) = 0.5`, `f(1.0) = 1.0`
#[inline]
pub fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let inv = -2.0 * t + 2.0;
        1.0 - inv * inv * inv / 2.0
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    // --- Animation::progress ---

    #[test]
    fn progress_near_zero_at_start() {
        let anim = Animation::new(Duration::from_secs(100));
        assert!(anim.progress() < 0.01, "progress should be near 0 at start");
    }

    #[test]
    fn progress_is_one_when_finished() {
        let start = Instant::now() - Duration::from_secs(10);
        let anim = Animation::with_start(start, Duration::from_secs(1));
        assert_eq!(anim.progress(), 1.0);
    }

    #[test]
    fn progress_clamped_past_one() {
        // Animation ran 100× its duration — must not exceed 1.0.
        let start = Instant::now() - Duration::from_secs(100);
        let anim = Animation::with_start(start, Duration::from_secs(1));
        assert_eq!(anim.progress(), 1.0);
    }

    #[test]
    fn progress_is_one_for_zero_duration() {
        let anim = Animation::new(Duration::ZERO);
        assert_eq!(anim.progress(), 1.0);
    }

    // --- Animation::is_finished ---

    #[test]
    fn is_finished_after_duration() {
        let start = Instant::now() - Duration::from_secs(10);
        let anim = Animation::with_start(start, Duration::from_secs(1));
        assert!(anim.is_finished());
    }

    #[test]
    fn not_finished_when_new() {
        let anim = Animation::new(Duration::from_secs(100));
        assert!(!anim.is_finished());
    }

    // --- Animation::elapsed ---

    #[test]
    fn elapsed_at_least_sleep_duration() {
        let start = Instant::now() - Duration::from_millis(50);
        let anim = Animation::with_start(start, Duration::from_secs(10));
        assert!(
            anim.elapsed() >= Duration::from_millis(50),
            "elapsed should be >= offset"
        );
    }

    #[test]
    fn elapsed_capped_at_duration() {
        let start = Instant::now() - Duration::from_secs(100);
        let anim = Animation::with_start(start, Duration::from_secs(1));
        assert_eq!(anim.elapsed(), Duration::from_secs(1));
    }

    // --- Animation::remaining ---

    #[test]
    fn remaining_zero_when_finished() {
        let start = Instant::now() - Duration::from_secs(10);
        let anim = Animation::with_start(start, Duration::from_secs(1));
        assert_eq!(anim.remaining(), Duration::ZERO);
    }

    #[test]
    fn remaining_close_to_full_when_new() {
        let anim = Animation::new(Duration::from_secs(100));
        assert!(
            anim.remaining() > Duration::from_secs(99),
            "remaining should be close to full duration"
        );
    }

    // --- Easing functions ---

    #[test]
    fn linear_endpoints() {
        assert_eq!(linear(0.0), 0.0);
        assert_eq!(linear(1.0), 1.0);
        assert_eq!(linear(0.5), 0.5);
    }

    #[test]
    fn ease_in_endpoints() {
        assert_eq!(ease_in(0.0), 0.0);
        assert_eq!(ease_in(1.0), 1.0);
        // ease_in is slow at the start, so midpoint < 0.5
        assert!(ease_in(0.5) < 0.5);
    }

    #[test]
    fn ease_out_endpoints() {
        assert_eq!(ease_out(0.0), 0.0);
        assert_eq!(ease_out(1.0), 1.0);
        // ease_out is fast at the start, so midpoint > 0.5
        assert!(ease_out(0.5) > 0.5);
    }

    #[test]
    fn ease_in_out_endpoints_and_midpoint() {
        assert_eq!(ease_in_out(0.0), 0.0);
        assert_eq!(ease_in_out(1.0), 1.0);
        // Symmetric — midpoint must be exactly 0.5
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn ease_in_out_monotone() {
        // Verify the curve is strictly increasing across 10 sample points.
        let mut prev = ease_in_out(0.0);
        for i in 1..=10 {
            let t = i as f32 / 10.0;
            let cur = ease_in_out(t);
            assert!(cur >= prev, "ease_in_out should be monotone at t={t}");
            prev = cur;
        }
    }

    // --- Animation::restart ---

    #[test]
    fn restart_resets_progress() {
        let start = Instant::now() - Duration::from_secs(10);
        let mut anim = Animation::with_start(start, Duration::from_secs(1));
        assert!(anim.is_finished());
        anim.restart();
        assert!(!anim.is_finished());
        assert!(anim.progress() < 0.1);
    }
}
