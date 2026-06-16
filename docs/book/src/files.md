# Files and the Workspace

A pane buffer can be a file from disk, not only a plugin. Disk access runs
through the filesystem bridge in the desktop shell, so open a folder and edit
your project directly in the desktop build.

## Opening a folder

Open a folder from the File menu, the leader (`SPC f f`), or the file tree
header. The tree loads lazily: directories expand on click and fetch their
children, files open in the focused pane on click.

A file buffer carries its path, text, and dirty flag in `state.files`, and the
status bar shows its language by extension.

## Saving

- `SPC f s` saves the focused file.
- `SPC f S` saves every dirty file.

## Project search

The Search view (`SPC /` or `SPC s p`) runs a query across the workspace on the
desktop with ripgrep's own engine: the `grep` line searcher over an `ignore`
parallel walk that respects `.gitignore`. The query is a smart-case regex,
case-insensitive until you type an uppercase letter, and a query that is not
valid regex matches literally. Each hit is a file and line. Click it to open the
file and jump to the line. References and multi-location go-to results land in
this same panel.

## The filesystem bridge

The page has no disk access, so `desktop/src/fs.rs` runs every file operation
natively: the folder picker, directory listing, file read and write, and the
ripgrep-engine search.
The page client (`src/fs.rs`) sends `FsRequest`s and applies each `FsResponse` to
the tree, the open buffers, and the search results.
