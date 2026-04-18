//! System clipboard access.
//!
//! Provides cross-platform read/write access to the system clipboard. On
//! non-macOS platforms, [`read`] always returns `None` and [`write`] is a
//! no-op — both are safe to call unconditionally.
//!
//! ## Usage
//!
//! ```ignore
//! use motif_core::clipboard;
//!
//! // Copy text to the clipboard
//! clipboard::write("hello world");
//!
//! // Paste text from the clipboard
//! if let Some(text) = clipboard::read() {
//!     println!("clipboard contains: {text}");
//! }
//! ```

// macOS implementation using NSPasteboard via objc2-app-kit
#[cfg(target_os = "macos")]
mod inner {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::NSString;

    /// Write `text` to the system clipboard.
    ///
    /// Clears any existing clipboard contents and replaces them with `text`.
    pub fn write(text: &str) {
        unsafe {
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            let s = NSString::from_str(text);
            pb.setString_forType(&s, NSPasteboardTypeString);
        }
    }

    /// Read text from the system clipboard.
    ///
    /// Returns `None` if the clipboard is empty or contains non-string data.
    pub fn read() -> Option<String> {
        unsafe {
            let pb = NSPasteboard::generalPasteboard();
            pb.stringForType(NSPasteboardTypeString)
                .map(|s| s.to_string())
        }
    }
}

// No-op stubs for all other platforms
#[cfg(not(target_os = "macos"))]
mod inner {
    /// No-op on non-macOS platforms.
    pub fn write(_text: &str) {}

    /// Always returns `None` on non-macOS platforms.
    pub fn read() -> Option<String> {
        None
    }
}

pub use inner::{read, write};
