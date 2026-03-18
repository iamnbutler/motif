//! Callback registry for element interactions.
//!
//! Stores callbacks registered during rendering and allows dispatching
//! them after input events are processed.

use crate::ElementId;
use std::collections::HashMap;

/// A click callback that can be invoked when an element is clicked.
pub type ClickCallback = Box<dyn FnMut()>;

/// A text change callback that receives the new text value.
///
/// Called when a text input's content changes (e.g. the user types).
pub type TextChangeCallback = Box<dyn FnMut(&str)>;

/// A blur callback invoked when an element loses keyboard focus.
///
/// Useful for "save on blur" patterns: the callback receives the final
/// text value so the application can commit it without storing extra state.
pub type BlurCallback = Box<dyn FnMut(&str)>;

/// A focus callback invoked when an element gains keyboard focus.
pub type FocusCallback = Box<dyn FnMut()>;

/// Registry for element callbacks.
///
/// Callbacks are registered during rendering and cleared each frame.
/// After processing input events, call the appropriate `dispatch_*`
/// method to invoke registered callbacks.
///
/// # Text input lifecycle
///
/// ```text
/// User clicks on TextInput
///     → dispatch_focus(id)         ← FocusCallback fires
/// User types
///     → dispatch_text_change(id, new_value)  ← TextChangeCallback fires
/// User presses Escape / Tab / clicks elsewhere
///     → dispatch_blur(id, final_value)       ← BlurCallback fires (save here)
/// ```
pub struct CallbackRegistry {
    click_handlers: HashMap<ElementId, ClickCallback>,
    text_change_handlers: HashMap<ElementId, TextChangeCallback>,
    blur_handlers: HashMap<ElementId, BlurCallback>,
    focus_handlers: HashMap<ElementId, FocusCallback>,
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
            text_change_handlers: HashMap::new(),
            blur_handlers: HashMap::new(),
            focus_handlers: HashMap::new(),
        }
    }

    // ── Click ────────────────────────────────────────────────────────────────

    /// Register a click handler for an element.
    pub fn on_click(&mut self, id: ElementId, callback: impl FnMut() + 'static) {
        self.click_handlers.insert(id, Box::new(callback));
    }

    /// Dispatch a click event to the registered handler.
    /// Returns `true` if a handler was found and invoked.
    pub fn dispatch_click(&mut self, id: ElementId) -> bool {
        if let Some(callback) = self.click_handlers.get_mut(&id) {
            callback();
            true
        } else {
            false
        }
    }

    /// Check if a click handler is registered for an element.
    pub fn has_click_handler(&self, id: ElementId) -> bool {
        self.click_handlers.contains_key(&id)
    }

    // ── Text change ──────────────────────────────────────────────────────────

    /// Register a text-change handler for a text input element.
    ///
    /// The callback receives the new text value after each edit.
    pub fn on_text_change(&mut self, id: ElementId, callback: impl FnMut(&str) + 'static) {
        self.text_change_handlers.insert(id, Box::new(callback));
    }

    /// Dispatch a text-change event to the registered handler.
    ///
    /// Call this whenever the text input's value changes (after processing a
    /// key event through `TextEditState`). Returns `true` if a handler was
    /// found and invoked.
    pub fn dispatch_text_change(&mut self, id: ElementId, new_value: &str) -> bool {
        if let Some(callback) = self.text_change_handlers.get_mut(&id) {
            callback(new_value);
            true
        } else {
            false
        }
    }

    /// Check if a text-change handler is registered for an element.
    pub fn has_text_change_handler(&self, id: ElementId) -> bool {
        self.text_change_handlers.contains_key(&id)
    }

    // ── Blur ─────────────────────────────────────────────────────────────────

    /// Register a blur handler for a text input element.
    ///
    /// The callback receives the final text value when focus leaves the
    /// element. Use this to commit edits ("save on blur").
    pub fn on_blur(&mut self, id: ElementId, callback: impl FnMut(&str) + 'static) {
        self.blur_handlers.insert(id, Box::new(callback));
    }

    /// Dispatch a blur event to the registered handler.
    ///
    /// Call this when a text input loses focus (e.g. `HandleKeyResult::Blur`,
    /// focus moving to another element, or a click outside). The `final_value`
    /// argument is the committed text at the moment focus was lost.
    ///
    /// Returns `true` if a handler was found and invoked.
    pub fn dispatch_blur(&mut self, id: ElementId, final_value: &str) -> bool {
        if let Some(callback) = self.blur_handlers.get_mut(&id) {
            callback(final_value);
            true
        } else {
            false
        }
    }

    /// Check if a blur handler is registered for an element.
    pub fn has_blur_handler(&self, id: ElementId) -> bool {
        self.blur_handlers.contains_key(&id)
    }

    // ── Focus ────────────────────────────────────────────────────────────────

    /// Register a focus handler for a text input element.
    ///
    /// The callback fires when the element gains keyboard focus.
    pub fn on_focus(&mut self, id: ElementId, callback: impl FnMut() + 'static) {
        self.focus_handlers.insert(id, Box::new(callback));
    }

    /// Dispatch a focus event to the registered handler.
    ///
    /// Call this when a text input gains focus (e.g. user clicks on it).
    /// Returns `true` if a handler was found and invoked.
    pub fn dispatch_focus(&mut self, id: ElementId) -> bool {
        if let Some(callback) = self.focus_handlers.get_mut(&id) {
            callback();
            true
        } else {
            false
        }
    }

    /// Check if a focus handler is registered for an element.
    pub fn has_focus_handler(&self, id: ElementId) -> bool {
        self.focus_handlers.contains_key(&id)
    }

    // ── Lifecycle ────────────────────────────────────────────────────────────

    /// Clear all registered callbacks.
    ///
    /// Call this at the start of each frame so that callbacks registered
    /// during `render()` are fresh for the current frame.
    pub fn clear(&mut self) {
        self.click_handlers.clear();
        self.text_change_handlers.clear();
        self.blur_handlers.clear();
        self.focus_handlers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    // ── Click tests ──────────────────────────────────────────────────────────

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

    // ── Text change tests ────────────────────────────────────────────────────

    #[test]
    fn registry_dispatches_text_change() {
        let mut registry = CallbackRegistry::new();
        let last_value = Rc::new(RefCell::new(String::new()));
        let lv = last_value.clone();

        registry.on_text_change(ElementId(10), move |v| {
            *lv.borrow_mut() = v.to_owned();
        });

        assert!(registry.dispatch_text_change(ElementId(10), "hello"));
        assert_eq!(*last_value.borrow(), "hello");

        assert!(registry.dispatch_text_change(ElementId(10), "hello world"));
        assert_eq!(*last_value.borrow(), "hello world");
    }

    #[test]
    fn text_change_returns_false_for_unknown_id() {
        let mut registry = CallbackRegistry::new();
        assert!(!registry.dispatch_text_change(ElementId(99), "text"));
    }

    #[test]
    fn has_text_change_handler() {
        let mut registry = CallbackRegistry::new();
        assert!(!registry.has_text_change_handler(ElementId(1)));
        registry.on_text_change(ElementId(1), |_| {});
        assert!(registry.has_text_change_handler(ElementId(1)));
    }

    #[test]
    fn text_change_cleared_by_clear() {
        let mut registry = CallbackRegistry::new();
        registry.on_text_change(ElementId(1), |_| {});
        assert!(registry.has_text_change_handler(ElementId(1)));
        registry.clear();
        assert!(!registry.has_text_change_handler(ElementId(1)));
    }

    // ── Blur tests ───────────────────────────────────────────────────────────

    #[test]
    fn registry_dispatches_blur_with_final_value() {
        let mut registry = CallbackRegistry::new();
        let saved = Rc::new(RefCell::new(String::new()));
        let s = saved.clone();

        registry.on_blur(ElementId(20), move |v| {
            *s.borrow_mut() = v.to_owned();
        });

        assert!(registry.dispatch_blur(ElementId(20), "committed text"));
        assert_eq!(*saved.borrow(), "committed text");
    }

    #[test]
    fn blur_returns_false_for_unknown_id() {
        let mut registry = CallbackRegistry::new();
        assert!(!registry.dispatch_blur(ElementId(99), "value"));
    }

    #[test]
    fn has_blur_handler() {
        let mut registry = CallbackRegistry::new();
        assert!(!registry.has_blur_handler(ElementId(5)));
        registry.on_blur(ElementId(5), |_| {});
        assert!(registry.has_blur_handler(ElementId(5)));
    }

    #[test]
    fn blur_cleared_by_clear() {
        let mut registry = CallbackRegistry::new();
        registry.on_blur(ElementId(5), |_| {});
        assert!(registry.has_blur_handler(ElementId(5)));
        registry.clear();
        assert!(!registry.has_blur_handler(ElementId(5)));
    }

    #[test]
    fn blur_callback_can_distinguish_empty_vs_nonempty() {
        let mut registry = CallbackRegistry::new();
        let values = Rc::new(RefCell::new(Vec::new()));
        let v = values.clone();

        registry.on_blur(ElementId(1), move |val| v.borrow_mut().push(val.to_owned()));

        registry.dispatch_blur(ElementId(1), "");
        registry.dispatch_blur(ElementId(1), "some text");
        assert_eq!(*values.borrow(), vec!["", "some text"]);
    }

    // ── Focus tests ──────────────────────────────────────────────────────────

    #[test]
    fn registry_dispatches_focus() {
        let mut registry = CallbackRegistry::new();
        let focused = Rc::new(Cell::new(false));
        let f = focused.clone();

        registry.on_focus(ElementId(30), move || f.set(true));

        assert!(!focused.get());
        assert!(registry.dispatch_focus(ElementId(30)));
        assert!(focused.get());
    }

    #[test]
    fn focus_returns_false_for_unknown_id() {
        let mut registry = CallbackRegistry::new();
        assert!(!registry.dispatch_focus(ElementId(99)));
    }

    #[test]
    fn has_focus_handler() {
        let mut registry = CallbackRegistry::new();
        assert!(!registry.has_focus_handler(ElementId(7)));
        registry.on_focus(ElementId(7), || {});
        assert!(registry.has_focus_handler(ElementId(7)));
    }

    #[test]
    fn focus_cleared_by_clear() {
        let mut registry = CallbackRegistry::new();
        registry.on_focus(ElementId(7), || {});
        assert!(registry.has_focus_handler(ElementId(7)));
        registry.clear();
        assert!(!registry.has_focus_handler(ElementId(7)));
    }

    // ── Cross-handler tests ──────────────────────────────────────────────────

    #[test]
    fn clear_removes_all_handler_types() {
        let mut registry = CallbackRegistry::new();
        let id = ElementId(42);

        registry.on_click(id, || {});
        registry.on_text_change(id, |_| {});
        registry.on_blur(id, |_| {});
        registry.on_focus(id, || {});

        assert!(registry.has_click_handler(id));
        assert!(registry.has_text_change_handler(id));
        assert!(registry.has_blur_handler(id));
        assert!(registry.has_focus_handler(id));

        registry.clear();

        assert!(!registry.has_click_handler(id));
        assert!(!registry.has_text_change_handler(id));
        assert!(!registry.has_blur_handler(id));
        assert!(!registry.has_focus_handler(id));
    }

    #[test]
    fn different_ids_do_not_interfere() {
        let mut registry = CallbackRegistry::new();
        let a_blurred = Rc::new(RefCell::new(String::new()));
        let b_blurred = Rc::new(RefCell::new(String::new()));

        let a = a_blurred.clone();
        registry.on_blur(ElementId(1), move |v| *a.borrow_mut() = v.to_owned());

        let b = b_blurred.clone();
        registry.on_blur(ElementId(2), move |v| *b.borrow_mut() = v.to_owned());

        registry.dispatch_blur(ElementId(1), "first");
        registry.dispatch_blur(ElementId(2), "second");

        assert_eq!(*a_blurred.borrow(), "first");
        assert_eq!(*b_blurred.borrow(), "second");
    }
}
