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
pub enum SharedString {
    /// A static string literal - zero allocation cost.
    Static(&'static str),
    /// A reference-counted string - cheap to clone.
    Owned(Arc<str>),
}

impl SharedString {
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

impl Deref for SharedString {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for SharedString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for SharedString {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for SharedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for SharedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl Default for SharedString {
    fn default() -> Self {
        Self::Static("")
    }
}

impl From<&'static str> for SharedString {
    fn from(s: &'static str) -> Self {
        Self::Static(s)
    }
}

impl From<String> for SharedString {
    fn from(s: String) -> Self {
        Self::Owned(Arc::from(s))
    }
}

impl From<Arc<str>> for SharedString {
    fn from(s: Arc<str>) -> Self {
        Self::Owned(s)
    }
}

impl PartialEq<str> for SharedString {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for SharedString {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for SharedString {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_string_is_zero_cost() {
        let s = SharedString::new_static("hello");
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn owned_string_from_string() {
        let s = SharedString::from(String::from("world"));
        assert_eq!(s.as_str(), "world");
    }

    #[test]
    fn cheap_clone() {
        let s1 = SharedString::from(String::from("test"));
        let s2 = s1.clone();
        assert_eq!(s1, s2);
    }

    #[test]
    fn equality_with_str() {
        let s = SharedString::new_static("hello");
        assert_eq!(s, "hello");
    }

    #[test]
    fn default_is_empty() {
        let s = SharedString::default();
        assert_eq!(s.as_str(), "");
    }
}
