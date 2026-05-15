//! Cheap-to-clone string type for UI text content.

use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

/// An immutable string that can be cheaply cloned.
///
/// Either a `&'static str` (zero-cost) or an `Arc<str>` (reference-counted).
/// Use this for text content in elements to avoid unnecessary allocations.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ArcStr {
    /// A static string literal - zero allocation cost.
    Static(&'static str),
    /// A reference-counted string - cheap to clone.
    Owned(Arc<str>),
}

impl ArcStr {
    /// Create from a static string literal.
    pub const fn new_static(s: &'static str) -> Self {
        Self::Static(s)
    }

    /// Create from a dynamic string.
    pub fn new(s: impl Into<Arc<str>>) -> Self {
        Self::Owned(s.into())
    }

    /// Get the underlying string slice.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Static(s) => s,
            Self::Owned(s) => s,
        }
    }
}

impl Deref for ArcStr {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for ArcStr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for ArcStr {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for ArcStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for ArcStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl Default for ArcStr {
    fn default() -> Self {
        Self::Static("")
    }
}

impl From<&'static str> for ArcStr {
    fn from(s: &'static str) -> Self {
        Self::Static(s)
    }
}

impl From<String> for ArcStr {
    fn from(s: String) -> Self {
        Self::Owned(Arc::from(s))
    }
}

impl From<Arc<str>> for ArcStr {
    fn from(s: Arc<str>) -> Self {
        Self::Owned(s)
    }
}

impl PartialEq<str> for ArcStr {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for ArcStr {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for ArcStr {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_string_is_zero_cost() {
        let s = ArcStr::new_static("hello");
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn owned_string_from_string() {
        let s = ArcStr::from(String::from("world"));
        assert_eq!(s.as_str(), "world");
    }

    #[test]
    fn cheap_clone() {
        let s1 = ArcStr::from(String::from("test"));
        let s2 = s1.clone();
        assert_eq!(s1, s2);
    }

    #[test]
    fn equality_with_str() {
        let s = ArcStr::new_static("hello");
        assert_eq!(s, "hello");
    }

    #[test]
    fn default_is_empty() {
        let s = ArcStr::default();
        assert_eq!(s.as_str(), "");
    }

    #[test]
    fn new_constructor_creates_owned() {
        let s = ArcStr::new("dynamic");
        assert_eq!(s.as_str(), "dynamic");
        assert!(matches!(s, ArcStr::Owned(_)));
    }

    #[test]
    fn from_arc_str() {
        let arc: Arc<str> = Arc::from("arc_backed");
        let s = ArcStr::from(arc);
        assert_eq!(s, "arc_backed");
    }

    #[test]
    fn deref_to_str() {
        let s = ArcStr::new_static("deref_me");
        let r: &str = &*s;
        assert_eq!(r, "deref_me");
    }

    #[test]
    fn display_format() {
        let s = ArcStr::new_static("display");
        assert_eq!(format!("{s}"), "display");
    }

    #[test]
    fn debug_format() {
        let s = ArcStr::new_static("debug");
        assert_eq!(format!("{s:?}"), "\"debug\"");
    }

    #[test]
    fn ordering() {
        let a = ArcStr::new_static("apple");
        let b = ArcStr::new_static("banana");
        assert!(a < b);
        assert!(b > a);
        assert_eq!(a.cmp(&a), std::cmp::Ordering::Equal);
    }

    #[test]
    fn static_and_owned_equal_same_content() {
        let static_s = ArcStr::new_static("same");
        let owned_s = ArcStr::from(String::from("same"));
        assert_eq!(static_s, owned_s);
    }

    #[test]
    fn eq_with_string() {
        let s = ArcStr::new_static("hello");
        assert_eq!(s, String::from("hello"));
    }

    #[test]
    fn as_ref_str() {
        let s = ArcStr::new_static("as_ref");
        let r: &str = s.as_ref();
        assert_eq!(r, "as_ref");
    }
}
