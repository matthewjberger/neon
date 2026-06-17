# Navigation and Refactoring

The heavier language features, all reachable from `SPC g`, the palette, and editor
plugins.

## Navigation

- **Go to definition** (`SPC g g`, `SPC g d`, or `gd`). Jumps to the definition.
  When a request returns many locations, they list in the symbol picker instead
  of jumping to the first.
- **Go to type definition** (`SPC g t`) and **implementation** (`SPC g i`). Same
  shape, different request.
- **Find references** (`SPC g r`). Lists every reference in the search panel,
  where a flat file-and-line list fits. Click a hit to navigate.
- **Document symbols** (`SPC s j`, `SPC j i`, `SPC g s`). Opens a fuzzy picker
  over the file's symbols. Filter as you type, Enter to jump.
- **Workspace symbols** (`SPC s J`, `SPC g S`). Searches the whole workspace for
  symbols matching the word at the caret, into the same picker.

## Refactoring

- **Rename** (`SPC g R`, `SPC x r`). Opens a prompt prefilled with the symbol.
  The returned workspace edit applies across open buffers, and any other affected
  file is opened and patched on demand.
- **Code actions** (`SPC a`, `SPC x .`). Lists the actions at the caret in a
  picker. Selecting one applies its edit or runs its command. The bridge handles
  the server's `applyEdit` request, so command-driven fixes work too.
- **Format** (`SPC j +`, `SPC j =`). Formats the whole buffer and applies the
  edits.

## Diagnostics navigation

- **Next and previous error** (`SPC e n`, `SPC e p`). Steps the caret to the next
  or previous diagnostic line in the focused file.
- **Problems** (`SPC e e`). Lists every diagnostic across open files in one
  panel; click a row to open the file and jump to the line.
- **The rust-analyzer log** (`SPC e l`, `SPC t l`). Toggles the panel that shows
  the server's output.

## Format on save

Saving a Rust file (`SPC f s`) formats it through rust-analyzer first and writes
the result. It is on by default, toggleable from the palette, and falls back to a
plain write when the server is not running.

## One vocabulary

Every one of these is an editor command, so the palette, the leader, and any
editor plugin reach the same set through `RunCommand`. The complete list is in the
[Command Reference](reference-commands.md).
