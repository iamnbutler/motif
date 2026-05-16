//! Callback registry for element interactions.
//!
//! Stores callbacks registered during rendering and allows dispatching
//! them after input events are processed.

use crate::ElementId;
use std::collections::HashMap;

/// A click callback that can be invoked when an element is clicked.
pub type ClickCallback = Box<dyn FnMut()>;

/// Registry for element callbacks.
///
/// Callbacks are registered during rendering and cleared each frame.
/// After processing input events, call `dispatch_click` to invoke
/// the appropriate callback.
pub struct CallbackRegistry {
    click_handlers: HashMap<ElementId, ClickCallback>,
}

impl Default for CallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CallbackRegistry {
    pub fn new() -> Self {
        Self {
            click_handlers: HashMap::new(),
        }
    }

    /// Register a click handler for an element.
    pub fn on_click(&mut self, id: ElementId, callback: impl FnMut() + 'static) {
        self.click_handlers.insert(id, Box::new(callback));
    }

    /// Dispatch a click event to the registered handler.
    /// Returns true if a handler was found and invoked.
    pub fn dispatch_click(&mut self, id: ElementId) -> bool {
        if let Some(callback) = self.click_handlers.get_mut(&id) {
            callback();
            true
        } else {
            false
        }
    }

    /// Clear all registered callbacks.
    /// Call this at the start of each frame.
    pub fn clear(&mut self) {
        self.click_handlers.clear();
    }

    /// Check if a click handler is registered for an element.
    pub fn has_click_handler(&self, id: ElementId) -> bool {
        self.click_handlers.contains_key(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn registry_dispatches_click() {
        let mut registry = CallbackRegistry::new();
        let clicked = Rc::new(Cell::new(false));
        let clicked_clone = clicked.clone();

        registry.on_click(ElementId(1), move || {
            clicked_clone.set(true);
        });

        assert!(!clicked.get());
        registry.dispatch_click(ElementId(1));
        assert!(clicked.get());
    }

    #[test]
    fn registry_returns_false_for_unknown_id() {
        let mut registry = CallbackRegistry::new();
        assert!(!registry.dispatch_click(ElementId(999)));
    }

    #[test]
    fn registry_clear_removes_handlers() {
        let mut registry = CallbackRegistry::new();
        registry.on_click(ElementId(1), || {});

        assert!(registry.has_click_handler(ElementId(1)));
        registry.clear();
        assert!(!registry.has_click_handler(ElementId(1)));
    }

    #[test]
    fn registry_multiple_handlers() {
        let mut registry = CallbackRegistry::new();
        let count = Rc::new(Cell::new(0));

        let c1 = count.clone();
        registry.on_click(ElementId(1), move || c1.set(c1.get() + 1));

        let c2 = count.clone();
        registry.on_click(ElementId(2), move || c2.set(c2.get() + 10));

        registry.dispatch_click(ElementId(1));
        assert_eq!(count.get(), 1);

        registry.dispatch_click(ElementId(2));
        assert_eq!(count.get(), 11);
    }

    #[test]
    fn dispatch_click_returns_true_for_registered() {
        let mut registry = CallbackRegistry::new();
        registry.on_click(ElementId(5), || {});
        assert!(registry.dispatch_click(ElementId(5)));
    }

    #[test]
    fn default_creates_empty_registry() {
        let mut registry = CallbackRegistry::default();
        assert!(!registry.has_click_handler(ElementId(0)));
        assert!(!registry.dispatch_click(ElementId(0)));
    }

    #[test]
    fn on_click_overwrite_replaces_handler() {
        let mut registry = CallbackRegistry::new();
        let first_called = Rc::new(Cell::new(false));
        let second_called = Rc::new(Cell::new(false));

        let f = first_called.clone();
        registry.on_click(ElementId(1), move || f.set(true));

        let s = second_called.clone();
        // Register a second handler for the same id — should overwrite
        registry.on_click(ElementId(1), move || s.set(true));

        registry.dispatch_click(ElementId(1));

        // Only the second handler should have been called
        assert!(!first_called.get(), "first handler should be overwritten");
        assert!(second_called.get(), "second handler should be called");
    }

    #[test]
    fn handler_invokable_multiple_times() {
        let mut registry = CallbackRegistry::new();
        let count = Rc::new(Cell::new(0u32));

        let c = count.clone();
        registry.on_click(ElementId(7), move || c.set(c.get() + 1));

        registry.dispatch_click(ElementId(7));
        registry.dispatch_click(ElementId(7));
        registry.dispatch_click(ElementId(7));

        assert_eq!(count.get(), 3, "handler should be invokable multiple times");
    }

    #[test]
    fn dispatch_after_clear_returns_false() {
        let mut registry = CallbackRegistry::new();
        registry.on_click(ElementId(3), || {});

        assert!(registry.dispatch_click(ElementId(3)));
        registry.clear();
        assert!(!registry.dispatch_click(ElementId(3)));
    }

    #[test]
    fn dispatching_one_does_not_affect_others() {
        let mut registry = CallbackRegistry::new();
        let a_count = Rc::new(Cell::new(0u32));
        let b_count = Rc::new(Cell::new(0u32));

        let a = a_count.clone();
        registry.on_click(ElementId(10), move || a.set(a.get() + 1));

        let b = b_count.clone();
        registry.on_click(ElementId(11), move || b.set(b.get() + 1));

        // Dispatch only element 10
        registry.dispatch_click(ElementId(10));
        registry.dispatch_click(ElementId(10));

        assert_eq!(a_count.get(), 2);
        assert_eq!(b_count.get(), 0, "element 11 should not be affected");
    }

    #[test]
    fn has_click_handler_true_for_registered() {
        let mut registry = CallbackRegistry::new();
        assert!(!registry.has_click_handler(ElementId(99)));
        registry.on_click(ElementId(99), || {});
        assert!(registry.has_click_handler(ElementId(99)));
    }
}
