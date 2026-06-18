//! Pure text measurements over a buffer's value and a UTF-16 caret offset: the
//! word and the prefix at the caret, the line and character of an offset, and
//! the caret's pixel anchor. No LSP state, so the request and response code
//! reads against plain helpers.

use web_sys::HtmlTextAreaElement;

/// The identifier the caret sits in or on the edge of, or empty if none.
pub(super) fn word_at(value: &str, caret: u32) -> String {
    let mut offset = 0;
    let mut current = String::new();
    let mut current_start = 0;
    for unit in value.chars() {
        let width = unit.len_utf16() as u32;
        if unit.is_alphanumeric() || unit == '_' {
            if current.is_empty() {
                current_start = offset;
            }
            current.push(unit);
        } else {
            if !current.is_empty() && caret >= current_start && caret <= offset {
                return current;
            }
            current.clear();
        }
        offset += width;
    }
    if !current.is_empty() && caret >= current_start && caret <= offset {
        return current;
    }
    String::new()
}

/// The zero-based line and UTF-16 character of a UTF-16 caret offset.
pub(super) fn line_character(value: &str, caret: u32) -> (u32, u32) {
    let mut line = 0;
    let mut column = 0;
    let mut seen = 0;
    for character in value.chars() {
        if seen >= caret {
            break;
        }
        let width = character.len_utf16() as u32;
        if character == '\n' {
            line += 1;
            column = 0;
        } else {
            column += width;
        }
        seen += width;
    }
    (line, column)
}

/// The identifier characters immediately before the caret, the prefix a
/// completion replaces.
pub(super) fn word_prefix(value: &str, caret: u32) -> String {
    let mut seen = 0;
    let mut word = String::new();
    for character in value.chars() {
        if seen >= caret {
            break;
        }
        if character.is_alphanumeric() || character == '_' {
            word.push(character);
        } else {
            word.clear();
        }
        seen += character.len_utf16() as u32;
    }
    word
}

/// The pixel point just below a line and character, where a popup is anchored.
pub(super) fn caret_pixel(element: &HtmlTextAreaElement, line: u32, column: u32) -> (f64, f64) {
    let (x, top) = crate::caret::cell(element, line, column);
    (x, top + crate::caret::line_height(element))
}
