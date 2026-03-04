//! Focus management for keyboard input routing.
//!
//! Provides `FocusHandle` for elements that can receive keyboard focus,
//! and `FocusState` for tracking the currently focused element.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};

/// Unique identifier for a focusable element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusId(pub(crate) u64);

/// Global counter for generating unique focus IDs.
static NEXT_FOCUS_ID: AtomicU64 = AtomicU64::new(1);

fn next_focus_id() -> FocusId {
    FocusId(NEXT_FOCUS_ID.fetch_add(1, Ordering::Relaxed))
}

/// Events emitted when focus changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusEvent {
    /// Element gained focus.
    Focus { id: FocusId },
    /// Element lost focus.
    Blur { id: FocusId },
}

/// Shared state for a focus handle. When all handles are dropped,
/// the focus is considered invalid.
struct FocusHandleInner {
    id: FocusId,
}

/// A handle to a focusable element.
///
/// Create via `FocusHandle::new()`. The handle can be cloned and shared.
/// When all clones are dropped, the focus ID becomes invalid.
#[derive(Clone)]
pub struct FocusHandle {
    inner: Arc<FocusHandleInner>,
}

impl std::fmt::Debug for FocusHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FocusHandle")
            .field("id", &self.inner.id)
            .finish()
    }
}

impl FocusHandle {
    /// Create a new focus handle with a unique ID.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(FocusHandleInner {
                id: next_focus_id(),
            }),
        }
    }

    /// Get the focus ID for this handle.
    pub fn id(&self) -> FocusId {
        self.inner.id
    }

    /// Check if this handle is currently focused.
    pub fn is_focused(&self, state: &FocusState) -> bool {
        state.focused == Some(self.inner.id)
    }

    /// Request focus for this handle.
    ///
    /// Returns the previous focus ID if focus changed, or None if already focused.
    pub fn focus(&self, state: &mut FocusState) -> Option<FocusId> {
        let prev = state.focused;
        if prev != Some(self.inner.id) {
            state.focused = Some(self.inner.id);
            state.pending_events.push(FocusEvent::Focus { id: self.inner.id });
            if let Some(prev_id) = prev {
                state.pending_events.push(FocusEvent::Blur { id: prev_id });
            }
            prev
        } else {
            None
        }
    }

    /// Create a weak reference to this handle.
    pub fn downgrade(&self) -> WeakFocusHandle {
        WeakFocusHandle {
            inner: Arc::downgrade(&self.inner),
        }
    }
}

impl Default for FocusHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for FocusHandle {
    fn eq(&self, other: &Self) -> bool {
        self.inner.id == other.inner.id
    }
}

impl Eq for FocusHandle {}

/// A weak reference to a focus handle.
///
/// Can be upgraded to a `FocusHandle` if the original handle still exists.
#[derive(Clone)]
pub struct WeakFocusHandle {
    inner: Weak<FocusHandleInner>,
}

impl WeakFocusHandle {
    /// Try to upgrade to a strong handle.
    pub fn upgrade(&self) -> Option<FocusHandle> {
        self.inner.upgrade().map(|inner| FocusHandle { inner })
    }

    /// Get the focus ID, if the handle is still valid.
    pub fn id(&self) -> Option<FocusId> {
        self.inner.upgrade().map(|inner| inner.id)
    }
}

/// Tracks focus state for a window.
#[derive(Debug, Default)]
pub struct FocusState {
    /// Currently focused element, if any.
    focused: Option<FocusId>,
    /// Pending focus events to be processed.
    pending_events: Vec<FocusEvent>,
}

impl FocusState {
    /// Create a new focus state with nothing focused.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the currently focused element ID.
    pub fn focused(&self) -> Option<FocusId> {
        self.focused
    }

    /// Check if a specific ID is focused.
    pub fn is_focused(&self, id: FocusId) -> bool {
        self.focused == Some(id)
    }

    /// Remove focus from the current element.
    ///
    /// Returns the previously focused ID, if any.
    pub fn blur(&mut self) -> Option<FocusId> {
        let prev = self.focused.take();
        if let Some(prev_id) = prev {
            self.pending_events.push(FocusEvent::Blur { id: prev_id });
        }
        prev
    }

    /// Take all pending focus events.
    pub fn take_events(&mut self) -> Vec<FocusEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Number of pending events.
    pub fn event_count(&self) -> usize {
        self.pending_events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_handle_has_unique_id() {
        let h1 = FocusHandle::new();
        let h2 = FocusHandle::new();
        assert_ne!(h1.id(), h2.id());
    }

    #[test]
    fn focus_handle_clone_shares_id() {
        let h1 = FocusHandle::new();
        let h2 = h1.clone();
        assert_eq!(h1.id(), h2.id());
        assert_eq!(h1, h2);
    }

    #[test]
    fn focus_state_starts_unfocused() {
        let state = FocusState::new();
        assert!(state.focused().is_none());
    }

    #[test]
    fn focus_sets_focused_element() {
        let mut state = FocusState::new();
        let handle = FocusHandle::new();

        handle.focus(&mut state);

        assert!(handle.is_focused(&state));
        assert_eq!(state.focused(), Some(handle.id()));
    }

    #[test]
    fn focus_emits_focus_event() {
        let mut state = FocusState::new();
        let handle = FocusHandle::new();

        handle.focus(&mut state);

        let events = state.take_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], FocusEvent::Focus { id: handle.id() });
    }

    #[test]
    fn focus_change_emits_blur_and_focus() {
        let mut state = FocusState::new();
        let h1 = FocusHandle::new();
        let h2 = FocusHandle::new();

        h1.focus(&mut state);
        state.take_events(); // Clear first focus event

        h2.focus(&mut state);

        let events = state.take_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], FocusEvent::Focus { id: h2.id() });
        assert_eq!(events[1], FocusEvent::Blur { id: h1.id() });
    }

    #[test]
    fn focus_same_element_is_noop() {
        let mut state = FocusState::new();
        let handle = FocusHandle::new();

        handle.focus(&mut state);
        state.take_events();

        let result = handle.focus(&mut state);

        assert!(result.is_none());
        assert_eq!(state.event_count(), 0);
    }

    #[test]
    fn blur_clears_focus() {
        let mut state = FocusState::new();
        let handle = FocusHandle::new();

        handle.focus(&mut state);
        state.take_events();

        let prev = state.blur();

        assert_eq!(prev, Some(handle.id()));
        assert!(state.focused().is_none());
        assert!(!handle.is_focused(&state));
    }

    #[test]
    fn blur_emits_blur_event() {
        let mut state = FocusState::new();
        let handle = FocusHandle::new();

        handle.focus(&mut state);
        state.take_events();
        state.blur();

        let events = state.take_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], FocusEvent::Blur { id: handle.id() });
    }

    #[test]
    fn blur_when_unfocused_is_noop() {
        let mut state = FocusState::new();

        let prev = state.blur();

        assert!(prev.is_none());
        assert_eq!(state.event_count(), 0);
    }

    #[test]
    fn weak_handle_upgrades_when_strong_exists() {
        let handle = FocusHandle::new();
        let weak = handle.downgrade();

        let upgraded = weak.upgrade();
        assert!(upgraded.is_some());
        assert_eq!(upgraded.unwrap().id(), handle.id());
    }

    #[test]
    fn weak_handle_fails_when_strong_dropped() {
        let weak = {
            let handle = FocusHandle::new();
            handle.downgrade()
        };

        assert!(weak.upgrade().is_none());
        assert!(weak.id().is_none());
    }
}
