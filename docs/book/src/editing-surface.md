# The Editing Surface

The editor is a native `<textarea>` with a Rust highlight layer behind it,
sharing the same box, plus a line-number gutter and a tab strip
(`components/editor_pane.rs`).

## Why a textarea

The textarea gives caret, selection, IME, clipboard, accessibility, and
scrolling for free, all proven and solid. A `<pre>` highlight layer sits exactly
behind it, scrolled in lockstep, so the text you edit and the colored text you
see line up.

Pressing Enter auto-indents: the new line carries the previous line's leading
whitespace and gains a level after a line that opens a block, so code stays
lined up as you type.

## Multi-cursor

Add carets with `Ctrl+Alt+Up` and `Ctrl+Alt+Down`, then type, backspace, or
delete and the edit lands at every caret at once, the column-editing workflow.
The extra carets render as blinking overlays at their offsets. A plain arrow
key, a click, or `Esc` collapses back to a single caret. The add-cursor and
clear actions are in the palette too.

## Highlighting

`highlight.rs` is a hand-written multi-language scanner: per-language keyword and
comment rules for rust, toml, json, javascript, and rhai. For rhai it colors the
scene command tokens straight from the live manifest, so the engine vocabulary
stays in sync with no extra wiring.

Highlighting is windowed to the lines scrolled into view plus a buffer, so the
scanner runs over the visible range rather than the whole file. Off-screen lines
render as plain text in the same positions, so the full text and the alignment
stay exact and large files stay responsive.

## Undo

Every edit goes through `state.set_buffer_text`, which records the pre-edit text
into a per-buffer undo stack (`undo.rs`) before writing. Native textarea undo is
bypassed by programmatic edits, so neon owns undo instead: `Ctrl+Z` and `Ctrl+Y`
undo every path, whether the change came from typing, a plugin op, find, or an
LSP edit. Bursts of typing coalesce into one undo step.

## Caret geometry

The on-screen position of the caret drives the completion popup, the hover card,
and the jump labels. `caret.rs` measures the font's advance with a canvas and
caches it, so those overlays anchor at the right pixel.
