# Avy-style Jumps

Neon has Spacemacs and avy style jumps: the editor labels targets on screen over
a dimmed buffer, and you type a label to move the caret there.

## The three jumps

- `SPC j w` labels every word start on screen.
- `SPC j l` labels every line.
- `SPC j j` labels every occurrence of a character you then type.

Each visible target gets a colored label. Type the label and the caret jumps to
it. Labels are a single key while there are few targets, and grow to two keys
when there are many.

## How it works

`jump.rs` computes the targets within the scrolled-in line range, draws the
labels over a dim overlay, and moves the caret to the one whose label you type.
The labels and the LSP popups share the caret geometry in `caret.rs`, which
measures the font advance with a canvas, so a label sits exactly on its target.

Targets are limited to the lines currently scrolled into view, which keeps the
label set small and the keys short.
