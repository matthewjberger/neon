# Right-click Menus

Neon suppresses the webview's default context menu and shows its own, tailored to
what you clicked.

## The menus

- **Editor surface.** Find and replace, the jumps, the palette, and split.
- **Tab.** Close tab, next and previous tab, split, close split. Right-clicking a
  tab focuses it first, so the actions apply to that tab.
- **File tree.** Open folder, project search, save all, new plugin.
- **Plugin panels.** New plugin, plugin manager, installed plugins.
- **Anywhere else.** A general menu: palette, control panel, files, search, the
  panel toggles, next theme, and help.

## How it works

The root container handles `contextmenu`, prevents the browser default, and opens
the general menu. Specific elements (the editor, a tab, a tree row, a plugin
panel) handle the event first, stop propagation, and open their own menu. Every
item is an editor command, so right-click dispatch goes through the same registry
as the palette and the leader (`components/context_menu.rs`). A transparent
backdrop closes the menu.
