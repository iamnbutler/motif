//! UTF-8 / UTF-16 offset conversion utilities.
//!
//! macOS text input APIs (NSTextInput, InputHandler) use UTF-16 code unit offsets
//! because Cocoa is historically based on NSString (UTF-16). These utilities convert
//! between the byte offsets that Rust strings use and the code unit offsets that macOS
//! expects.
//!
//! # UTF-16 quick reference
//!
//! - ASCII and most Latin characters (U+0000–U+007F): 1 UTF-8 byte, 1 UTF-16 unit
//! - BMP two-byte characters (U+0080–U+07FF, e.g. é): 2 UTF-8 bytes, 1 UTF-16 unit
//! - BMP three-byte characters (U+0800–U+FFFF, e.g. €, ≠): 3 UTF-8 bytes, 1 UTF-16 unit
//! - Supplementary plane characters (U+10000–, e.g. 😀): 4 UTF-8 bytes, 2 UTF-16 units
//!   (encoded as a surrogate pair in UTF-16)

use std::ops::Range;

/// Convert a UTF-16 code unit offset to a UTF-8 byte offset in `text`.
///
/// Returns `None` if `utf16_offset` is out of bounds, or falls inside a
/// surrogate pair (i.e. does not correspond to a character boundary).
///
/// # Examples
///
/// ```
/// use motif_core::utf16_to_utf8_offset;
///
/// let text = "café";
/// assert_eq!(utf16_to_utf8_offset(text, 3), Some(3)); // before 'é'
/// assert_eq!(utf16_to_utf8_offset(text, 4), Some(5)); // after 'é' (end of string)
///
/// let emoji = "hi😀";
/// assert_eq!(utf16_to_utf8_offset(emoji, 3), None);   // inside surrogate pair
/// assert_eq!(utf16_to_utf8_offset(emoji, 4), Some(6)); // after emoji
/// ```
pub fn utf16_to_utf8_offset(text: &str, utf16_offset: usize) -> Option<usize> {
    if utf16_offset == 0 {
        return Some(0);
    }
    let mut utf16_pos: usize = 0;
    for (byte_idx, ch) in text.char_indices() {
        if utf16_pos == utf16_offset {
            return Some(byte_idx);
        }
        utf16_pos += ch.len_utf16();
        if utf16_pos > utf16_offset {
            // Target falls inside a surrogate pair — not a valid boundary.
            return None;
        }
    }
    // End-of-string is a valid boundary.
    if utf16_pos == utf16_offset {
        Some(text.len())
    } else {
        None
    }
}

/// Convert a UTF-8 byte offset in `text` to a UTF-16 code unit offset.
///
/// Returns `None` if `utf8_offset` is beyond `text.len()`, or does not fall
/// on a character boundary (i.e. is inside a multi-byte sequence).
///
/// # Examples
///
/// ```
/// use motif_core::utf8_to_utf16_offset;
///
/// let text = "café";
/// assert_eq!(utf8_to_utf16_offset(text, 3), Some(3)); // start of 'é'
/// assert_eq!(utf8_to_utf16_offset(text, 4), None);    // inside 'é'
/// assert_eq!(utf8_to_utf16_offset(text, 5), Some(4)); // end of string
///
/// let emoji = "hi😀";
/// assert_eq!(utf8_to_utf16_offset(emoji, 2), Some(2)); // before emoji
/// assert_eq!(utf8_to_utf16_offset(emoji, 6), Some(4)); // after emoji (2 UTF-16 units)
/// ```
pub fn utf8_to_utf16_offset(text: &str, utf8_offset: usize) -> Option<usize> {
    if utf8_offset > text.len() {
        return None;
    }
    if utf8_offset > 0 && !text.is_char_boundary(utf8_offset) {
        return None;
    }
    let count = text[..utf8_offset].chars().map(|c| c.len_utf16()).sum();
    Some(count)
}

/// Return the length of `text` in UTF-16 code units.
///
/// This matches the length that `NSString` (and other Cocoa APIs) report.
///
/// # Examples
///
/// ```
/// use motif_core::utf16_len;
///
/// assert_eq!(utf16_len("hello"), 5);
/// assert_eq!(utf16_len("café"),  4); // é = 1 UTF-16 unit
/// assert_eq!(utf16_len("€"),     1); // € = 1 UTF-16 unit (3 UTF-8 bytes)
/// assert_eq!(utf16_len("😀"),    2); // emoji = 2 UTF-16 units (surrogate pair)
/// ```
pub fn utf16_len(text: &str) -> usize {
    text.chars().map(|c| c.len_utf16()).sum()
}

/// Convert a UTF-16 code unit range to a UTF-8 byte range in `text`.
///
/// Returns `None` if either endpoint is invalid (see [`utf16_to_utf8_offset`]).
pub fn utf16_range_to_utf8(text: &str, range: Range<usize>) -> Option<Range<usize>> {
    let start = utf16_to_utf8_offset(text, range.start)?;
    let end = utf16_to_utf8_offset(text, range.end)?;
    Some(start..end)
}

/// Convert a UTF-8 byte range to a UTF-16 code unit range in `text`.
///
/// Returns `None` if either endpoint is invalid (see [`utf8_to_utf16_offset`]).
pub fn utf8_range_to_utf16(text: &str, range: Range<usize>) -> Option<Range<usize>> {
    let start = utf8_to_utf16_offset(text, range.start)?;
    let end = utf8_to_utf16_offset(text, range.end)?;
    Some(start..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- utf16_len ---

    #[test]
    fn utf16_len_empty() {
        assert_eq!(utf16_len(""), 0);
    }

    #[test]
    fn utf16_len_ascii() {
        assert_eq!(utf16_len("hello"), 5);
    }

    #[test]
    fn utf16_len_bmp_two_byte_utf8() {
        // é = U+00E9: 2 UTF-8 bytes, 1 UTF-16 unit
        assert_eq!(utf16_len("é"), 1);
        assert_eq!(utf16_len("café"), 4);
    }

    #[test]
    fn utf16_len_bmp_three_byte_utf8() {
        // € = U+20AC: 3 UTF-8 bytes, 1 UTF-16 unit
        assert_eq!(utf16_len("€"), 1);
        assert_eq!(utf16_len("€100"), 4);
    }

    #[test]
    fn utf16_len_supplementary_plane() {
        // 😀 = U+1F600: 4 UTF-8 bytes, 2 UTF-16 units (surrogate pair)
        assert_eq!(utf16_len("😀"), 2);
        assert_eq!(utf16_len("hi😀"), 4);
    }

    // --- utf16_to_utf8_offset ---

    #[test]
    fn utf16_to_utf8_zero_offset() {
        assert_eq!(utf16_to_utf8_offset("hello", 0), Some(0));
        assert_eq!(utf16_to_utf8_offset("", 0), Some(0));
    }

    #[test]
    fn utf16_to_utf8_ascii_offsets() {
        let text = "hello";
        for i in 0..=5 {
            assert_eq!(utf16_to_utf8_offset(text, i), Some(i));
        }
    }

    #[test]
    fn utf16_to_utf8_bmp_two_byte_utf8() {
        // "café": c(1), a(1), f(1), é(2 UTF-8 bytes, 1 UTF-16 unit)
        // UTF-16 offsets: 0→byte 0, 1→byte 1, 2→byte 2, 3→byte 3, 4→byte 5 (end)
        let text = "café";
        assert_eq!(utf16_to_utf8_offset(text, 0), Some(0));
        assert_eq!(utf16_to_utf8_offset(text, 1), Some(1));
        assert_eq!(utf16_to_utf8_offset(text, 2), Some(2));
        assert_eq!(utf16_to_utf8_offset(text, 3), Some(3));
        assert_eq!(utf16_to_utf8_offset(text, 4), Some(5));
    }

    #[test]
    fn utf16_to_utf8_bmp_three_byte_utf8() {
        // "€100": €(3 UTF-8 bytes, 1 UTF-16 unit), 1, 0, 0
        let text = "€100";
        assert_eq!(utf16_to_utf8_offset(text, 0), Some(0));
        assert_eq!(utf16_to_utf8_offset(text, 1), Some(3));
        assert_eq!(utf16_to_utf8_offset(text, 2), Some(4));
        assert_eq!(utf16_to_utf8_offset(text, 4), Some(6));
    }

    #[test]
    fn utf16_to_utf8_supplementary_plane() {
        // "hi😀": h(1), i(1), 😀(4 UTF-8 bytes, 2 UTF-16 units)
        let text = "hi😀";
        assert_eq!(utf16_to_utf8_offset(text, 0), Some(0));
        assert_eq!(utf16_to_utf8_offset(text, 1), Some(1));
        assert_eq!(utf16_to_utf8_offset(text, 2), Some(2));
        assert_eq!(utf16_to_utf8_offset(text, 3), None); // inside surrogate pair
        assert_eq!(utf16_to_utf8_offset(text, 4), Some(6)); // after emoji
    }

    #[test]
    fn utf16_to_utf8_out_of_bounds() {
        assert_eq!(utf16_to_utf8_offset("hi", 3), None);
        assert_eq!(utf16_to_utf8_offset("hi", 100), None);
    }

    #[test]
    fn utf16_to_utf8_end_of_string() {
        assert_eq!(utf16_to_utf8_offset("hello", 5), Some(5));
        assert_eq!(utf16_to_utf8_offset("", 0), Some(0));
    }

    // --- utf8_to_utf16_offset ---

    #[test]
    fn utf8_to_utf16_zero_offset() {
        assert_eq!(utf8_to_utf16_offset("hello", 0), Some(0));
        assert_eq!(utf8_to_utf16_offset("", 0), Some(0));
    }

    #[test]
    fn utf8_to_utf16_ascii_offsets() {
        let text = "hello";
        for i in 0..=5 {
            assert_eq!(utf8_to_utf16_offset(text, i), Some(i));
        }
    }

    #[test]
    fn utf8_to_utf16_bmp_two_byte_utf8() {
        // "café": é starts at byte 3, ends at byte 5 (2 UTF-8 bytes, 1 UTF-16)
        let text = "café";
        assert_eq!(utf8_to_utf16_offset(text, 0), Some(0));
        assert_eq!(utf8_to_utf16_offset(text, 1), Some(1));
        assert_eq!(utf8_to_utf16_offset(text, 2), Some(2));
        assert_eq!(utf8_to_utf16_offset(text, 3), Some(3)); // start of é
        assert_eq!(utf8_to_utf16_offset(text, 5), Some(4)); // end of string
    }

    #[test]
    fn utf8_to_utf16_bmp_two_byte_utf8_mid_char() {
        // Byte 4 is a continuation byte inside é — not a char boundary
        assert_eq!(utf8_to_utf16_offset("café", 4), None);
    }

    #[test]
    fn utf8_to_utf16_supplementary_plane() {
        // "hi😀": 😀 starts at byte 2 (4 UTF-8 bytes, 2 UTF-16 units)
        let text = "hi😀";
        assert_eq!(utf8_to_utf16_offset(text, 0), Some(0));
        assert_eq!(utf8_to_utf16_offset(text, 1), Some(1));
        assert_eq!(utf8_to_utf16_offset(text, 2), Some(2)); // before 😀
        assert_eq!(utf8_to_utf16_offset(text, 6), Some(4)); // after 😀
    }

    #[test]
    fn utf8_to_utf16_supplementary_plane_mid_char() {
        // Bytes 3, 4, 5 are continuation bytes inside 😀
        let text = "hi😀";
        assert_eq!(utf8_to_utf16_offset(text, 3), None);
        assert_eq!(utf8_to_utf16_offset(text, 4), None);
        assert_eq!(utf8_to_utf16_offset(text, 5), None);
    }

    #[test]
    fn utf8_to_utf16_out_of_bounds() {
        assert_eq!(utf8_to_utf16_offset("hi", 3), None);
        assert_eq!(utf8_to_utf16_offset("hi", 100), None);
    }

    // --- utf16_range_to_utf8 ---

    #[test]
    fn utf16_range_to_utf8_ascii() {
        let text = "hello world";
        assert_eq!(utf16_range_to_utf8(text, 6..11), Some(6..11));
    }

    #[test]
    fn utf16_range_to_utf8_with_bmp() {
        let text = "café";
        // Full string: UTF-16 0..4 → UTF-8 0..5
        assert_eq!(utf16_range_to_utf8(text, 0..4), Some(0..5));
        // Just 'é': UTF-16 3..4 → UTF-8 3..5
        assert_eq!(utf16_range_to_utf8(text, 3..4), Some(3..5));
    }

    #[test]
    fn utf16_range_to_utf8_invalid_endpoint() {
        let text = "hi😀";
        assert_eq!(utf16_range_to_utf8(text, 2..3), None); // end inside surrogate
        assert_eq!(utf16_range_to_utf8(text, 3..4), None); // start inside surrogate
    }

    // --- utf8_range_to_utf16 ---

    #[test]
    fn utf8_range_to_utf16_ascii() {
        let text = "hello world";
        assert_eq!(utf8_range_to_utf16(text, 6..11), Some(6..11));
    }

    #[test]
    fn utf8_range_to_utf16_with_bmp() {
        let text = "café";
        // Full string: UTF-8 0..5 → UTF-16 0..4
        assert_eq!(utf8_range_to_utf16(text, 0..5), Some(0..4));
        // Just 'é': UTF-8 3..5 → UTF-16 3..4
        assert_eq!(utf8_range_to_utf16(text, 3..5), Some(3..4));
    }

    #[test]
    fn utf8_range_to_utf16_invalid_endpoint() {
        // Byte 4 is inside 'é' — not a char boundary
        assert_eq!(utf8_range_to_utf16("café", 3..4), None);
    }

    // --- round-trip tests ---

    #[test]
    fn round_trip_ascii() {
        let text = "hello";
        for i in 0..=text.len() {
            let utf16 = utf8_to_utf16_offset(text, i).unwrap();
            let utf8 = utf16_to_utf8_offset(text, utf16).unwrap();
            assert_eq!(utf8, i);
        }
    }

    #[test]
    fn round_trip_mixed() {
        let text = "café 😀 €";
        let boundaries: Vec<usize> = text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect();
        for &byte_offset in &boundaries {
            let utf16 = utf8_to_utf16_offset(text, byte_offset).unwrap();
            let back = utf16_to_utf8_offset(text, utf16).unwrap();
            assert_eq!(back, byte_offset);
        }
    }
}
