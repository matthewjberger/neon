# Find and Replace

The find bar acts on the focused buffer. Open it with `Ctrl+F`, the leader
(`SPC s s`), or vim `/` in normal mode.

## Buffer find

Type a query to find matches in the current buffer and step through them. Replace
swaps the matches in place. Find runs against the focused textarea
(`find.rs`), and every replacement goes through the same `set_buffer_text` path as
any other edit, so it records undo.

## Project search

Find is buffer-local. To search the whole workspace, use the Search view
(`SPC /` or `SPC s p`), which runs on the desktop and respects `.gitignore`. See
[Files and the Workspace](files.md).
