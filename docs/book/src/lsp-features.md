# Diagnostics, Completion, Hover

The everyday language features, all anchored at the caret with the canvas-measured
font geometry from `caret.rs`.

## Diagnostics

rust-analyzer's `publishDiagnostics` become the diagnostics strip under the
editor, with the line, column, and message. The strip shows the diagnostics for
the focused file. The rhai language worker drives plugin diagnostics in parallel,
so a scene plugin and a Rust file each get the right checker.

## Completion

Completion fires as you type, debounced. The popup shows three columns: the kind
(mapped to a short tag like `fn`, `struct`, `field`), the label, and the detail
(the type or signature). Move with the arrow keys, accept with Enter or a click,
which replaces the typed prefix. Accepting records undo like any edit.

## Hover

Ask for hover on demand:

- `K` in normal mode, or `SPC h h`, describes the symbol at the caret.
- Hovering the pointer over a symbol shows the same card.

The card renders the server's hover markup. It clears when you move on.

## Signature help

While you type a call, neon requests signature help after an open paren or a
comma and shows the active signature in the hover card, clearing it on the
closing paren. `SPC h s` triggers it on demand.

## Overlay sync

Open files sync to the server with `didOpen` and `didChange` using the overlay
model, so the server sees your unsaved edits. That is why completion and hover
reflect what is on screen, not what is on disk. See
[How the LSP Bridge Works](lsp-bridge.md).
