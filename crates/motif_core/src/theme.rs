//! Theme trait and default implementation for consistent visual styling.
//!
//! The [`Theme`] trait defines a palette of semantic color tokens used across
//! motif's built-in elements. [`DefaultTheme`] provides values that match the
//! hardcoded defaults in those elements, making it easy to create a custom
//! theme that diverges only where you want.
//!
//! # Usage
//!
//! ```ignore
//! use motif_core::theme::{Theme, DefaultTheme};
//!
//! struct MyTheme;
//!
//! impl Theme for MyTheme {
//!     fn accent(&self) -> Srgba { Srgba::new(0.6, 0.1, 0.8, 1.0) }
//!     // … override only what you need; delegate the rest to DefaultTheme
//! }
//! ```

use palette::Srgba;

/// A set of semantic color tokens that drive the appearance of motif elements.
///
/// Implement this trait to create a custom theme. Every method has a default
/// implementation that delegates to [`DefaultTheme`], so you only need to
/// override the tokens you want to change.
///
/// # Token categories
///
/// | Category   | Tokens                                                    |
/// |------------|-----------------------------------------------------------|
/// | Accent     | [`accent`](Self::accent), [`accent_hover`](Self::accent_hover), [`accent_press`](Self::accent_press) |
/// | Surface    | [`background`](Self::background), [`surface`](Self::surface) |
/// | Text       | [`text_primary`](Self::text_primary), [`text_secondary`](Self::text_secondary), [`text_placeholder`](Self::text_placeholder), [`text_on_accent`](Self::text_on_accent) |
/// | Border     | [`border`](Self::border), [`border_focus`](Self::border_focus), [`border_active`](Self::border_active) |
/// | Selection  | [`selection`](Self::selection) |
pub trait Theme {
    // -----------------------------------------------------------------------
    // Accent / brand
    // -----------------------------------------------------------------------

    /// Primary brand/accent color. Used for focused borders, check indicators,
    /// and primary buttons.
    fn accent(&self) -> Srgba {
        DefaultTheme.accent()
    }

    /// Accent color when hovered.
    fn accent_hover(&self) -> Srgba {
        DefaultTheme.accent_hover()
    }

    /// Accent color when pressed.
    fn accent_press(&self) -> Srgba {
        DefaultTheme.accent_press()
    }

    // -----------------------------------------------------------------------
    // Surfaces
    // -----------------------------------------------------------------------

    /// Page / window background color.
    fn background(&self) -> Srgba {
        DefaultTheme.background()
    }

    /// Card / panel surface color (one level above background).
    fn surface(&self) -> Srgba {
        DefaultTheme.surface()
    }

    // -----------------------------------------------------------------------
    // Text
    // -----------------------------------------------------------------------

    /// Default body text color.
    fn text_primary(&self) -> Srgba {
        DefaultTheme.text_primary()
    }

    /// Muted / secondary text color.
    fn text_secondary(&self) -> Srgba {
        DefaultTheme.text_secondary()
    }

    /// Placeholder text color.
    fn text_placeholder(&self) -> Srgba {
        DefaultTheme.text_placeholder()
    }

    /// Text color on top of an accent-colored background (e.g. button label).
    fn text_on_accent(&self) -> Srgba {
        DefaultTheme.text_on_accent()
    }

    // -----------------------------------------------------------------------
    // Borders
    // -----------------------------------------------------------------------

    /// Default border color for inputs and containers.
    fn border(&self) -> Srgba {
        DefaultTheme.border()
    }

    /// Border color when the element has keyboard focus.
    fn border_focus(&self) -> Srgba {
        DefaultTheme.border_focus()
    }

    /// Border color when the element is hovered or pressed (interactive feedback).
    fn border_active(&self) -> Srgba {
        DefaultTheme.border_active()
    }

    // -----------------------------------------------------------------------
    // Selection
    // -----------------------------------------------------------------------

    /// Text selection highlight color (semi-transparent).
    fn selection(&self) -> Srgba {
        DefaultTheme.selection()
    }
}

// ---------------------------------------------------------------------------
// DefaultTheme
// ---------------------------------------------------------------------------

/// The built-in default theme.
///
/// Color values match the hardcoded defaults in motif's built-in elements,
/// giving you a consistent starting point without needing to wire up a custom
/// theme.
pub struct DefaultTheme;

impl Theme for DefaultTheme {
    fn accent(&self) -> Srgba {
        Srgba::new(0.2, 0.4, 0.8, 1.0)
    }

    fn accent_hover(&self) -> Srgba {
        Srgba::new(0.3, 0.5, 0.9, 1.0)
    }

    fn accent_press(&self) -> Srgba {
        Srgba::new(0.15, 0.3, 0.6, 1.0)
    }

    fn background(&self) -> Srgba {
        Srgba::new(1.0, 1.0, 1.0, 1.0)
    }

    fn surface(&self) -> Srgba {
        Srgba::new(0.96, 0.96, 0.96, 1.0)
    }

    fn text_primary(&self) -> Srgba {
        Srgba::new(0.0, 0.0, 0.0, 1.0)
    }

    fn text_secondary(&self) -> Srgba {
        Srgba::new(0.6, 0.6, 0.6, 1.0)
    }

    fn text_placeholder(&self) -> Srgba {
        Srgba::new(0.6, 0.6, 0.6, 1.0)
    }

    fn text_on_accent(&self) -> Srgba {
        Srgba::new(1.0, 1.0, 1.0, 1.0)
    }

    fn border(&self) -> Srgba {
        Srgba::new(0.7, 0.7, 0.7, 1.0)
    }

    fn border_focus(&self) -> Srgba {
        Srgba::new(0.2, 0.4, 0.8, 1.0)
    }

    fn border_active(&self) -> Srgba {
        Srgba::new(0.4, 0.4, 0.4, 1.0)
    }

    fn selection(&self) -> Srgba {
        Srgba::new(0.3, 0.5, 0.9, 0.3)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: Srgba, b: Srgba) -> bool {
        let eps = 1e-5_f32;
        (a.red - b.red).abs() < eps
            && (a.green - b.green).abs() < eps
            && (a.blue - b.blue).abs() < eps
            && (a.alpha - b.alpha).abs() < eps
    }

    #[test]
    fn default_theme_accent_matches_element_defaults() {
        let t = DefaultTheme;
        // Matches button background, text_input focus border, checkbox check indicator
        assert!(approx_eq(t.accent(), Srgba::new(0.2, 0.4, 0.8, 1.0)));
    }

    #[test]
    fn default_theme_accent_hover() {
        let t = DefaultTheme;
        assert!(approx_eq(t.accent_hover(), Srgba::new(0.3, 0.5, 0.9, 1.0)));
    }

    #[test]
    fn default_theme_accent_press() {
        let t = DefaultTheme;
        assert!(approx_eq(t.accent_press(), Srgba::new(0.15, 0.3, 0.6, 1.0)));
    }

    #[test]
    fn default_theme_background_is_white() {
        let t = DefaultTheme;
        assert!(approx_eq(t.background(), Srgba::new(1.0, 1.0, 1.0, 1.0)));
    }

    #[test]
    fn default_theme_surface_is_light_gray() {
        let t = DefaultTheme;
        assert!(approx_eq(t.surface(), Srgba::new(0.96, 0.96, 0.96, 1.0)));
    }

    #[test]
    fn default_theme_text_primary_is_black() {
        let t = DefaultTheme;
        assert!(approx_eq(t.text_primary(), Srgba::new(0.0, 0.0, 0.0, 1.0)));
    }

    #[test]
    fn default_theme_text_secondary_and_placeholder_match() {
        let t = DefaultTheme;
        // Both are the same muted gray in the default theme
        assert!(approx_eq(t.text_secondary(), t.text_placeholder()));
    }

    #[test]
    fn default_theme_text_on_accent_is_white() {
        let t = DefaultTheme;
        assert!(approx_eq(
            t.text_on_accent(),
            Srgba::new(1.0, 1.0, 1.0, 1.0)
        ));
    }

    #[test]
    fn default_theme_border_focus_matches_accent() {
        let t = DefaultTheme;
        // Focus border uses the same accent color
        assert!(approx_eq(t.border_focus(), t.accent()));
    }

    #[test]
    fn default_theme_selection_is_semi_transparent() {
        let t = DefaultTheme;
        let s = t.selection();
        assert!(s.alpha < 1.0, "selection should be semi-transparent");
    }

    #[test]
    fn custom_theme_can_override_accent() {
        struct RedTheme;

        impl Theme for RedTheme {
            fn accent(&self) -> Srgba {
                Srgba::new(0.8, 0.1, 0.1, 1.0)
            }
        }

        let t = RedTheme;
        assert!(approx_eq(t.accent(), Srgba::new(0.8, 0.1, 0.1, 1.0)));
        // Non-overridden tokens fall back to DefaultTheme
        assert!(approx_eq(t.background(), DefaultTheme.background()));
    }

    #[test]
    fn trait_object_works() {
        let theme: &dyn Theme = &DefaultTheme;
        // Should be callable through a trait object without issue
        let _ = theme.accent();
        let _ = theme.background();
        let _ = theme.text_primary();
    }
}
