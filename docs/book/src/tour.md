# A Tour of the Window

The window has a top menu bar, a left activity bar and sidebar, the editor area
in the center, and an optional 3D preview column on the right.

## Top menu bar

A thin menu bar in the editor style: File, Edit, View, Go, Run, and Help
dropdowns that dispatch editor commands, a centered command center that opens the
palette and shows the workspace name, and a compact right side with the editor
mode, renderer stats, the theme picker, and the Claude toggle.

## Activity bar and sidebar

The activity bar on the far left switches the sidebar between views:

- Installed plugins.
- The plugin manager (the catalog you install from).
- The file tree.
- Project search.

## Editor area

The center holds the editing surface: a tab strip, a line-number gutter, and the
text. Panes split right or below and resize, and each pane holds many buffers as
tabs. See [The Editing Surface](editing-surface.md) and
[Panes and Tabs](panes-tabs.md).

## Preview column

The right column hosts the 3D viewport and the console. Toggle it from the View
menu or the top bar. Scene plugins draw into the viewport, and the console shows
the command and event traffic from each tick. See
[The Engine Viewport](viewport.md).

## Panels and overlays

Several surfaces appear on demand: the command palette, the which-key leader
menu, the find bar, the rust-analyzer log, the control panel, the rename prompt,
the code-action picker, the symbol picker, and the keybindings help. Most are
reachable from the Spacemacs leader, the palette, or a right-click menu.
