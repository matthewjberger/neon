# Panes and Tabs

A pane holds many buffers as tabs. Panes split right or below and resize, and
each edits independently.

## Tabs

Each pane has a tab strip. A buffer is a scene plugin, an editor plugin, a
read-only standard-library module, or a file from disk. Click a tab to focus it,
the close button to close it, and drag a tab to reorder it within the pane.

Tab commands, reachable from the leader (`SPC b`) or the palette:

- Close tab, next tab, previous tab.

## Splits

Split the focused pane right or below, close a split, focus another, and balance
all splits to equal width. The window commands live under `SPC w`:

- `v` or `/` split right, `s` or `-` split below.
- `d` or `x` close split.
- `h` and `k` focus previous, `l` and `j` focus next, `w` focus the other pane.
- `=` balance.

## Sessions

The opened folder and the open files are saved to local storage and reopened on
launch (`session.rs`), so a restart drops you back where you were.
