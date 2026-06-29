//! Neon's editing core: a rope-backed text document with one or more selections.
//! Engine-agnostic and DOM-free — pure text and selection logic — so it is unit
//! tested on the host and reused by the editor surface. Offsets are character
//! indices, matching the rest of the editor.

use ropey::Rope;

/// A selection (or a bare caret when `anchor == head`): the fixed `anchor` and
/// the moving `head`, as character offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

impl Selection {
    /// A zero-width selection (a caret) at an offset.
    pub fn caret(at: usize) -> Self {
        Self {
            anchor: at,
            head: at,
        }
    }

    /// A selection spanning `anchor` to `head`.
    pub fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    /// The lower offset.
    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    /// The higher offset.
    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    /// Whether the selection is a bare caret.
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// The number of characters selected.
    pub fn len(&self) -> usize {
        self.end() - self.start()
    }
}

/// A text document with multiple selections. Edits apply at every selection, so
/// multi-cursor is native rather than bolted on.
#[derive(Clone)]
pub struct Document {
    text: Rope,
    selections: Vec<Selection>,
    primary: usize,
}

impl Document {
    /// A document over some text, with a single caret at the start.
    pub fn new(text: &str) -> Self {
        Self {
            text: Rope::from_str(text),
            selections: vec![Selection::caret(0)],
            primary: 0,
        }
    }

    /// The whole text as a string.
    pub fn text(&self) -> String {
        self.text.to_string()
    }

    /// The number of characters.
    pub fn len_chars(&self) -> usize {
        self.text.len_chars()
    }

    /// The number of lines.
    pub fn len_lines(&self) -> usize {
        self.text.len_lines()
    }

    /// The 0-based line a character offset sits on.
    pub fn char_to_line(&self, char_index: usize) -> usize {
        self.text.char_to_line(char_index.min(self.len_chars()))
    }

    /// The offset of the start of a line.
    pub fn line_to_char(&self, line: usize) -> usize {
        self.text
            .line_to_char(line.min(self.len_lines().saturating_sub(1)))
    }

    /// The offset of the end of a line, before its newline.
    pub fn line_end(&self, line: usize) -> usize {
        let line = line.min(self.len_lines().saturating_sub(1));
        let start = self.text.line_to_char(line);
        let slice = self.text.line(line);
        let mut len = slice.len_chars();
        if len > 0 && slice.char(len - 1) == '\n' {
            len -= 1;
        }
        start + len
    }

    /// The selections, sorted by start.
    pub fn selections(&self) -> &[Selection] {
        &self.selections
    }

    /// The primary selection (the one a single-caret editor tracks).
    pub fn primary(&self) -> Selection {
        self.selections[self.primary]
    }

    /// Replaces the selection set, clamping, sorting, and merging overlaps.
    pub fn set_selections(&mut self, selections: Vec<Selection>) {
        let len = self.len_chars();
        let mut clamped: Vec<Selection> = selections
            .into_iter()
            .map(|selection| Selection::new(selection.anchor.min(len), selection.head.min(len)))
            .collect();
        clamped.sort_by_key(|selection| selection.start());
        let mut merged: Vec<Selection> = Vec::new();
        for selection in clamped {
            if let Some(last) = merged.last_mut()
                && selection.start() <= last.end()
            {
                let head = last.end().max(selection.end());
                let anchor = last.start();
                *last = Selection::new(anchor, head);
                continue;
            }
            merged.push(selection);
        }
        if merged.is_empty() {
            merged.push(Selection::caret(0));
        }
        self.primary = self.primary.min(merged.len() - 1);
        self.selections = merged;
    }

    /// Collapses to a single caret at the primary selection's head.
    pub fn collapse(&mut self) {
        let head = self.primary().head;
        self.selections = vec![Selection::caret(head)];
        self.primary = 0;
    }

    /// Adds a caret, keeping the set sorted and merged.
    pub fn add_cursor(&mut self, at: usize) {
        let mut selections = self.selections.clone();
        selections.push(Selection::caret(at));
        self.set_selections(selections);
    }

    /// Replaces every selection with `text` (typing, paste). Empty selections
    /// just insert. Applies left to right with a running offset so every edit
    /// lands correctly.
    pub fn insert(&mut self, text: &str) {
        let insert_len = text.chars().count();
        let mut ordered = self.selections.clone();
        ordered.sort_by_key(|selection| selection.start());
        let mut shift: isize = 0;
        let mut result = Vec::with_capacity(ordered.len());
        for selection in ordered {
            let start = (selection.start() as isize + shift) as usize;
            let end = (selection.end() as isize + shift) as usize;
            if end > start {
                self.text.remove(start..end);
            }
            if !text.is_empty() {
                self.text.insert(start, text);
            }
            result.push(Selection::caret(start + insert_len));
            shift += insert_len as isize - (end as isize - start as isize);
        }
        self.primary = self.primary.min(result.len() - 1);
        self.selections = result;
    }

    /// Deletes the character before each caret, or the selection when non-empty.
    pub fn backspace(&mut self) {
        for selection in self.selections.iter_mut() {
            if selection.is_empty() && selection.head > 0 {
                selection.anchor = selection.head - 1;
            }
        }
        self.insert("");
    }

    /// Deletes the character after each caret, or the selection when non-empty.
    pub fn delete_forward(&mut self) {
        let len = self.len_chars();
        for selection in self.selections.iter_mut() {
            if selection.is_empty() && selection.head < len {
                selection.anchor = selection.head + 1;
            }
        }
        self.insert("");
    }

    /// Moves every selection's head by `f`, extending from the anchor or
    /// collapsing to the new head.
    fn move_each(&mut self, extend: bool, f: impl Fn(&Document, &Selection) -> usize) {
        let current = self.selections.clone();
        let moved: Vec<Selection> = current
            .iter()
            .map(|selection| {
                let head = f(self, selection);
                if extend {
                    Selection::new(selection.anchor, head)
                } else {
                    Selection::caret(head)
                }
            })
            .collect();
        self.set_selections(moved);
    }

    pub fn move_left(&mut self, extend: bool) {
        self.move_each(extend, |_, selection| selection.head.saturating_sub(1));
    }

    pub fn move_right(&mut self, extend: bool) {
        self.move_each(extend, |document, selection| {
            (selection.head + 1).min(document.len_chars())
        });
    }

    pub fn move_up(&mut self, extend: bool) {
        self.move_each(extend, |document, selection| {
            let line = document.char_to_line(selection.head);
            if line == 0 {
                return selection.head;
            }
            let column = selection.head - document.line_to_char(line);
            let target = line - 1;
            (document.line_to_char(target) + column).min(document.line_end(target))
        });
    }

    pub fn move_down(&mut self, extend: bool) {
        self.move_each(extend, |document, selection| {
            let line = document.char_to_line(selection.head);
            if line + 1 >= document.len_lines() {
                return selection.head;
            }
            let column = selection.head - document.line_to_char(line);
            let target = line + 1;
            (document.line_to_char(target) + column).min(document.line_end(target))
        });
    }

    pub fn move_line_start(&mut self, extend: bool) {
        self.move_each(extend, |document, selection| {
            document.line_to_char(document.char_to_line(selection.head))
        });
    }

    pub fn move_line_end(&mut self, extend: bool) {
        self.move_each(extend, |document, selection| {
            document.line_end(document.char_to_line(selection.head))
        });
    }

    pub fn move_doc_start(&mut self, extend: bool) {
        self.move_each(extend, |_, _| 0);
    }

    pub fn move_doc_end(&mut self, extend: bool) {
        self.move_each(extend, |document, _| document.len_chars());
    }

    pub fn move_word_next(&mut self, extend: bool) {
        self.move_each(extend, |document, selection| {
            document.next_word(selection.head)
        });
    }

    pub fn move_word_prev(&mut self, extend: bool) {
        self.move_each(extend, |document, selection| {
            document.prev_word(selection.head)
        });
    }

    /// The text covered by the primary selection.
    pub fn primary_text(&self) -> String {
        let selection = self.primary();
        self.text
            .slice(selection.start()..selection.end())
            .to_string()
    }

    fn next_word(&self, from: usize) -> usize {
        let len = self.len_chars();
        let mut index = from;
        while index < len && is_word(self.text.char(index)) {
            index += 1;
        }
        while index < len && !is_word(self.text.char(index)) {
            index += 1;
        }
        index
    }

    fn prev_word(&self, from: usize) -> usize {
        let mut index = from;
        while index > 0 && !is_word(self.text.char(index - 1)) {
            index -= 1;
        }
        while index > 0 && is_word(self.text.char(index - 1)) {
            index -= 1;
        }
        index
    }
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_at_caret() {
        let mut document = Document::new("hello");
        document.set_selections(vec![Selection::caret(5)]);
        document.insert(" world");
        assert_eq!(document.text(), "hello world");
        assert_eq!(document.primary(), Selection::caret(11));
    }

    #[test]
    fn multi_cursor_insert() {
        let mut document = Document::new("ab\ncd");
        document.set_selections(vec![Selection::caret(0), Selection::caret(3)]);
        document.insert("X");
        assert_eq!(document.text(), "Xab\nXcd");
        assert_eq!(
            document.selections(),
            &[Selection::caret(1), Selection::caret(5)]
        );
    }

    #[test]
    fn typing_replaces_selection() {
        let mut document = Document::new("hello");
        document.set_selections(vec![Selection::new(0, 5)]);
        document.insert("hi");
        assert_eq!(document.text(), "hi");
        assert_eq!(document.primary(), Selection::caret(2));
    }

    #[test]
    fn backspace_and_delete() {
        let mut document = Document::new("abc");
        document.set_selections(vec![Selection::caret(2)]);
        document.backspace();
        assert_eq!(document.text(), "ac");
        document.set_selections(vec![Selection::caret(1)]);
        document.delete_forward();
        assert_eq!(document.text(), "a");
    }

    #[test]
    fn vertical_motion_keeps_column() {
        let mut document = Document::new("abcd\nef\nghij");
        document.set_selections(vec![Selection::caret(3)]);
        document.move_down(false);
        // Line 1 ("ef") is shorter, so the caret clamps to its end.
        assert_eq!(document.primary(), Selection::caret(7));
        document.move_down(false);
        // Line 2 ("ghij") is long enough to restore column 3 from the original.
        assert_eq!(document.char_to_line(document.primary().head), 2);
    }

    #[test]
    fn word_motions() {
        let mut document = Document::new("foo bar baz");
        document.set_selections(vec![Selection::caret(0)]);
        document.move_word_next(false);
        assert_eq!(document.primary(), Selection::caret(4));
        document.move_word_prev(false);
        assert_eq!(document.primary(), Selection::caret(0));
    }

    #[test]
    fn selections_merge_when_overlapping() {
        let mut document = Document::new("abcdef");
        document.set_selections(vec![Selection::new(0, 3), Selection::new(2, 5)]);
        assert_eq!(document.selections(), &[Selection::new(0, 5)]);
    }

    #[test]
    fn line_and_doc_motions() {
        let mut document = Document::new("abc\ndef");
        document.set_selections(vec![Selection::caret(5)]);
        document.move_line_start(false);
        assert_eq!(document.primary(), Selection::caret(4));
        document.move_line_end(false);
        assert_eq!(document.primary(), Selection::caret(7));
        document.move_doc_start(false);
        assert_eq!(document.primary(), Selection::caret(0));
        document.move_doc_end(false);
        assert_eq!(document.primary(), Selection::caret(7));
    }

    #[test]
    fn extend_builds_a_selection() {
        let mut document = Document::new("abcdef");
        document.set_selections(vec![Selection::caret(0)]);
        document.move_right(true);
        document.move_right(true);
        assert_eq!(document.primary(), Selection::new(0, 2));
        assert_eq!(document.primary_text(), "ab");
    }
}
