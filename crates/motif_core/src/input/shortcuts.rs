//! App-level keyboard shortcut dispatch.
//!
//! Provides a registry for binding key combinations to application-level
//! callbacks. This is distinct from [`super::bindings`] (text-editing actions)
//! — `ShortcutRegistry` is for app-wide commands like "Save", "New", or
//! "Toggle sidebar".
//!
//! # Example
//!
//! ```
//! use motif_core::input::{KeyboardShortcut, ShortcutRegistry};
//! use winit::keyboard::ModifiersState;
//!
//! let mut registry = ShortcutRegistry::new();
//!
//! // Register a platform-native "Save" shortcut (⌘S on macOS, Ctrl+S elsewhere).
//! let save_id = registry.register(KeyboardShortcut::cmd('s'), || {
//!     // save the document
//! });
//!
//! // Later, dispatch incoming key events from the event loop:
//! // if registry.dispatch(&key, &modifiers) { /* shortcut fired */ }
//!
//! // Remove the shortcut when the window closes:
//! registry.unregister(save_id);
//! ```

use winit::keyboard::{Key, ModifiersState, NamedKey};

/// A keyboard shortcut: a specific key plus a required set of modifier keys.
///
/// Create shortcuts with [`KeyboardShortcut::cmd`] for the most common case,
/// or with [`KeyboardShortcut::new_char`] / [`KeyboardShortcut::new_named`] for
/// full control.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyboardShortcut {
    /// The logical key to match.
    pub key: Key,
    /// Required modifier keys. Modifiers must match exactly — e.g. registering
    /// `SUPER` will not fire when `SUPER | SHIFT` is held (register separately
    /// for the shifted variant).
    pub modifiers: ModifiersState,
}

impl KeyboardShortcut {
    /// Create a shortcut from a character key with explicit modifiers.
    ///
    /// `c` is compared case-insensitively, so `'S'` and `'s'` are equivalent.
    pub fn new_char(c: char, modifiers: ModifiersState) -> Self {
        // Store lowercase to simplify matching.
        let lower: String = c.to_lowercase().collect();
        Self {
            key: Key::Character(lower.into()),
            modifiers,
        }
    }

    /// Create a shortcut from a [`NamedKey`] with explicit modifiers.
    pub fn new_named(key: NamedKey, modifiers: ModifiersState) -> Self {
        Self {
            key: Key::Named(key),
            modifiers,
        }
    }

    /// Create a platform-native command shortcut.
    ///
    /// Uses `⌘` (Super) on macOS and `Ctrl` on all other platforms.
    /// This covers the most common app-level shortcut pattern.
    pub fn cmd(c: char) -> Self {
        #[cfg(target_os = "macos")]
        let mods = ModifiersState::SUPER;
        #[cfg(not(target_os = "macos"))]
        let mods = ModifiersState::CONTROL;
        Self::new_char(c, mods)
    }

    /// Create a platform-native command+shift shortcut.
    ///
    /// Uses `⌘⇧` on macOS and `Ctrl+Shift` elsewhere.
    pub fn cmd_shift(c: char) -> Self {
        #[cfg(target_os = "macos")]
        let mods = ModifiersState::SUPER | ModifiersState::SHIFT;
        #[cfg(not(target_os = "macos"))]
        let mods = ModifiersState::CONTROL | ModifiersState::SHIFT;
        Self::new_char(c, mods)
    }

    /// Returns `true` if this shortcut matches the given key and modifiers.
    ///
    /// Character keys are matched case-insensitively. Modifiers must match
    /// exactly (no subset or superset matching).
    pub fn matches(&self, key: &Key, modifiers: &ModifiersState) -> bool {
        if *modifiers != self.modifiers {
            return false;
        }
        match (&self.key, key) {
            (Key::Character(a), Key::Character(b)) => {
                // Both stored lowercase (new_char normalises); compare directly.
                // Use eq_ignore_ascii_case as a safety net for externally
                // constructed shortcuts that skip new_char normalisation.
                (**a).eq_ignore_ascii_case(&**b)
            }
            _ => self.key == *key,
        }
    }
}

// ─── Registry ────────────────────────────────────────────────────────────────

/// Opaque identifier for a registered shortcut.
///
/// Returned by [`ShortcutRegistry::register`]; pass it to
/// [`ShortcutRegistry::unregister`] to remove the shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShortcutId(u64);

struct Entry {
    id: ShortcutId,
    shortcut: KeyboardShortcut,
    callback: Box<dyn Fn() + Send + Sync>,
}

/// Registry of app-level keyboard shortcuts.
///
/// Shortcuts are matched in registration order; the **first** match wins.
/// Designed for a small number of bindings (tens, not thousands) — linear
/// scan is fine at this scale.
///
/// # Thread safety
///
/// `ShortcutRegistry` is not `Sync` itself, but callbacks must be
/// `Send + Sync + 'static`.
#[derive(Default)]
pub struct ShortcutRegistry {
    entries: Vec<Entry>,
    next_id: u64,
}

impl ShortcutRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a keyboard shortcut with a callback.
    ///
    /// Returns a [`ShortcutId`] that can be used to remove the shortcut later.
    pub fn register(
        &mut self,
        shortcut: KeyboardShortcut,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> ShortcutId {
        let id = ShortcutId(self.next_id);
        self.next_id += 1;
        self.entries.push(Entry {
            id,
            shortcut,
            callback: Box::new(callback),
        });
        id
    }

    /// Unregister a previously registered shortcut by its [`ShortcutId`].
    ///
    /// No-op if the `id` is not found (e.g. already removed).
    pub fn unregister(&mut self, id: ShortcutId) {
        self.entries.retain(|e| e.id != id);
    }

    /// Dispatch a key-press event.
    ///
    /// Finds the first registered shortcut that matches `(key, modifiers)`,
    /// calls its callback, and returns `true`. Returns `false` if no shortcut
    /// matched.
    ///
    /// Call this for every `InputEvent::Key` where `state == ElementState::Pressed`.
    pub fn dispatch(&self, key: &Key, modifiers: &ModifiersState) -> bool {
        for entry in &self.entries {
            if entry.shortcut.matches(key, modifiers) {
                (entry.callback)();
                return true;
            }
        }
        false
    }

    /// Returns the number of registered shortcuts.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if no shortcuts are registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use winit::keyboard::{Key, ModifiersState, NamedKey};

    fn no_mods() -> ModifiersState {
        ModifiersState::empty()
    }

    fn ctrl() -> ModifiersState {
        ModifiersState::CONTROL
    }

    fn ctrl_shift() -> ModifiersState {
        ModifiersState::CONTROL | ModifiersState::SHIFT
    }

    // ── KeyboardShortcut ──────────────────────────────────────────────────────

    #[test]
    fn char_shortcut_matches_key() {
        let s = KeyboardShortcut::new_char('s', ctrl());
        assert!(s.matches(&Key::Character("s".into()), &ctrl()));
    }

    #[test]
    fn char_shortcut_case_insensitive() {
        let s = KeyboardShortcut::new_char('s', ctrl());
        // 'S' (shifted) should match the same shortcut
        assert!(s.matches(&Key::Character("S".into()), &ctrl()));
    }

    #[test]
    fn char_shortcut_wrong_modifiers_no_match() {
        let s = KeyboardShortcut::new_char('s', ctrl());
        assert!(!s.matches(&Key::Character("s".into()), &no_mods()));
        assert!(!s.matches(&Key::Character("s".into()), &ctrl_shift()));
    }

    #[test]
    fn char_shortcut_wrong_key_no_match() {
        let s = KeyboardShortcut::new_char('s', ctrl());
        assert!(!s.matches(&Key::Character("a".into()), &ctrl()));
    }

    #[test]
    fn named_shortcut_matches() {
        let s = KeyboardShortcut::new_named(NamedKey::Escape, no_mods());
        assert!(s.matches(&Key::Named(NamedKey::Escape), &no_mods()));
    }

    #[test]
    fn named_shortcut_wrong_key_no_match() {
        let s = KeyboardShortcut::new_named(NamedKey::Escape, no_mods());
        assert!(!s.matches(&Key::Named(NamedKey::Enter), &no_mods()));
    }

    #[test]
    fn cmd_shortcut_uses_platform_modifier() {
        let s = KeyboardShortcut::cmd('n');
        #[cfg(target_os = "macos")]
        assert_eq!(s.modifiers, ModifiersState::SUPER);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(s.modifiers, ModifiersState::CONTROL);
    }

    #[test]
    fn cmd_shift_shortcut_uses_platform_modifiers() {
        let s = KeyboardShortcut::cmd_shift('z');
        #[cfg(target_os = "macos")]
        assert_eq!(s.modifiers, ModifiersState::SUPER | ModifiersState::SHIFT);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(s.modifiers, ModifiersState::CONTROL | ModifiersState::SHIFT);
    }

    // ── ShortcutRegistry ──────────────────────────────────────────────────────

    #[test]
    fn registry_starts_empty() {
        let r = ShortcutRegistry::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn register_increases_len() {
        let mut r = ShortcutRegistry::new();
        r.register(KeyboardShortcut::new_char('s', ctrl()), || {});
        assert_eq!(r.len(), 1);
        r.register(KeyboardShortcut::new_char('z', ctrl()), || {});
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn dispatch_fires_matching_shortcut() {
        let fired = Arc::new(Mutex::new(false));
        let fired_clone = fired.clone();

        let mut r = ShortcutRegistry::new();
        r.register(KeyboardShortcut::new_char('s', ctrl()), move || {
            *fired_clone.lock().unwrap() = true;
        });

        let matched = r.dispatch(&Key::Character("s".into()), &ctrl());
        assert!(matched);
        assert!(*fired.lock().unwrap());
    }

    #[test]
    fn dispatch_returns_false_when_no_match() {
        let mut r = ShortcutRegistry::new();
        r.register(KeyboardShortcut::new_char('s', ctrl()), || {});

        // Wrong key
        assert!(!r.dispatch(&Key::Character("a".into()), &ctrl()));
        // Wrong modifiers
        assert!(!r.dispatch(&Key::Character("s".into()), &no_mods()));
    }

    #[test]
    fn dispatch_first_match_wins() {
        let count = Arc::new(Mutex::new(0u32));
        let c1 = count.clone();
        let c2 = count.clone();

        let mut r = ShortcutRegistry::new();
        // Register two identical shortcuts
        r.register(KeyboardShortcut::new_char('s', ctrl()), move || {
            *c1.lock().unwrap() += 1;
        });
        r.register(KeyboardShortcut::new_char('s', ctrl()), move || {
            *c2.lock().unwrap() += 10;
        });

        r.dispatch(&Key::Character("s".into()), &ctrl());
        // Only the first callback should fire
        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[test]
    fn unregister_removes_shortcut() {
        let fired = Arc::new(Mutex::new(false));
        let fired_clone = fired.clone();

        let mut r = ShortcutRegistry::new();
        let id = r.register(KeyboardShortcut::new_char('s', ctrl()), move || {
            *fired_clone.lock().unwrap() = true;
        });

        r.unregister(id);
        assert!(r.is_empty());

        let matched = r.dispatch(&Key::Character("s".into()), &ctrl());
        assert!(!matched);
        assert!(!*fired.lock().unwrap());
    }

    #[test]
    fn unregister_unknown_id_is_noop() {
        let mut r = ShortcutRegistry::new();
        r.register(KeyboardShortcut::new_char('a', ctrl()), || {});
        // Unregister a non-existent ID
        r.unregister(ShortcutId(999));
        // Original shortcut should still be there
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn multiple_shortcuts_all_registered() {
        let mut r = ShortcutRegistry::new();
        r.register(KeyboardShortcut::cmd('n'), || {});
        r.register(KeyboardShortcut::cmd('s'), || {});
        r.register(KeyboardShortcut::cmd_shift('z'), || {});
        r.register(
            KeyboardShortcut::new_named(NamedKey::Escape, no_mods()),
            || {},
        );
        assert_eq!(r.len(), 4);
    }
}
