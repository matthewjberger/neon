# Editor Plugins

An editor plugin extends the editor itself. It has one hook, `on_key`, which runs
for every keystroke and pushes ops the host applies.

```rhai
fn on_key() {
    if ctrl && key == "/" {
        ops.push("Consume");
        ops.push(#{ Insert: "// ----------------\n" });
    }
}
```

## What is in scope

| Name | What it is |
| --- | --- |
| `key` | the key name (`"a"`, `"Enter"`, `"Escape"`, `"ArrowLeft"`) |
| `mode` | the current mode label (`"normal"`, `"insert"`, or your own) |
| `ctrl` `shift` `alt` | modifier booleans |
| `ops` | the array you push actions to |
| `state` | a map that persists across keystrokes |

## Ops

The host owns the buffer text, so the ops do real editing. Push a plain string
for an action with no payload, or a one-key map for an action with one:

```rhai
ops.push("Consume");                 // stop the key typing normally
ops.push(#{ SetMode: "insert" });
ops.push(#{ Insert: "text" });
ops.push(#{ Move: 1 });              // negative reverses
ops.push("LineEnd");
ops.push("DuplicateLine");
ops.push(#{ ToggleComment: "//" });
```

The full set is in the [Editor Op Reference](reference-ops.md). It covers caret
motion, edits, the case and line transforms, mode and status, and the menu and
command controls below.

## Persistent state

`state` is a map that survives between keystrokes, so a plugin can build
multi-key sequences. The Spacemacs leader uses it to remember the pending prefix:

```rhai
fn on_key() {
    let pending = if "pending" in state { state.pending } else { "" };
    if pending == "g" {
        ops.push("Consume");
        state.pending = "";
        if key == "d" { ops.push(#{ RunCommand: "go-to-definition" }); }
        return;
    }
    if key == "g" { ops.push("Consume"); state.pending = "g"; }
}
```

## Driving the rest of the editor

`RunCommand` runs any entry in the command registry by id, so a plugin reaches
every editor action, not just buffer edits:

```rhai
ops.push(#{ RunCommand: "format-document" });
ops.push(#{ RunCommand: "split-right" });
ops.push("OpenPalette");
```

## Which-key menus

`ShowMenu` publishes the leader menu, the bottom panel that lists the next keys.
`HideMenu` clears it. The keymap and its menu live together in the plugin:

```rhai
fn file_menu() {
    #{ title: "+Files", items: [
        #{ key: "s", label: "Save" },
        #{ key: "f", label: "Open folder" },
    ] };
}

fn on_key() {
    ops.push(#{ ShowMenu: file_menu() });
}
```

## Modifier guards

A modal layer should leave modified chords to the non-modal plugins. The shipped
layers return early on `ctrl` or `alt` in normal mode, and handle `Escape` in
insert mode, so Emacs keys and the like stack cleanly on top.

## Only one modal layer

Vim and Spacemacs each claim the keyboard in normal mode, so enabling one
disables the other. The non-modal plugins (auto pairs, better escape, line tools,
and so on) stack freely. See [The Plugin Manager](plugin-manager.md).
