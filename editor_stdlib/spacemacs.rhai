// Spacemacs-style keybindings: vim modal editing plus an SPC leader that runs
// editor commands. The leader menus are where window management lives.
//
//   SPC SPC        command palette (M-x)
//   SPC w v / s    split the editor
//   SPC w d        close the split
//   SPC w w        focus the other pane
//   SPC t p/c/r/a  toggle preview / console / reference / Claude
//   SPC b b        open a buffer (palette)
//   SPC p n        new plugin
//   :              command palette
//
// Normal mode: hjkl move, i/a/A/o insert, x delete, dd delete line, 0/$ ends,
// w/b words, Escape leaves insert. Edit this live to change anything.

fn on_key() {
    if mode == "insert" {
        if key == "Escape" {
            ops.push("Consume");
            ops.push(#{ SetMode: "normal" });
            ops.push(#{ Move: -1 });
        }
        return;
    }

    let pending = if "pending" in state { state.pending } else { "" };

    if pending == "SPC" {
        ops.push("Consume");
        ops.push(#{ SetStatus: "" });
        state.pending = "";
        if key == " " { ops.push(#{ RunCommand: "open-palette" }); }
        else if key == "?" { ops.push(#{ RunCommand: "open-help" }); }
        else if key == "w" { state.pending = "SPC w"; ops.push(#{ SetStatus: "SPC w-" }); }
        else if key == "t" { state.pending = "SPC t"; ops.push(#{ SetStatus: "SPC t-" }); }
        else if key == "b" { state.pending = "SPC b"; ops.push(#{ SetStatus: "SPC b-" }); }
        else if key == "p" { state.pending = "SPC p"; ops.push(#{ SetStatus: "SPC p-" }); }
        return;
    }
    if pending == "SPC w" {
        ops.push("Consume");
        ops.push(#{ SetStatus: "" });
        state.pending = "";
        if key == "v" || key == "s" { ops.push(#{ RunCommand: "split-editor" }); }
        else if key == "d" { ops.push(#{ RunCommand: "close-split" }); }
        else if key == "w" { ops.push(#{ RunCommand: "focus-other" }); }
        return;
    }
    if pending == "SPC t" {
        ops.push("Consume");
        ops.push(#{ SetStatus: "" });
        state.pending = "";
        if key == "p" { ops.push(#{ RunCommand: "toggle-preview" }); }
        else if key == "c" { ops.push(#{ RunCommand: "toggle-console" }); }
        else if key == "r" { ops.push(#{ RunCommand: "toggle-reference" }); }
        else if key == "a" { ops.push(#{ RunCommand: "toggle-chat" }); }
        return;
    }
    if pending == "SPC b" {
        ops.push("Consume");
        ops.push(#{ SetStatus: "" });
        state.pending = "";
        if key == "b" { ops.push(#{ RunCommand: "open-palette" }); }
        return;
    }
    if pending == "SPC p" {
        ops.push("Consume");
        ops.push(#{ SetStatus: "" });
        state.pending = "";
        if key == "n" { ops.push(#{ RunCommand: "new-plugin" }); }
        return;
    }
    if pending == "d" {
        ops.push("Consume");
        state.pending = "";
        if key == "d" { ops.push("DeleteLine"); }
        return;
    }

    ops.push("Consume");
    if key == " " { state.pending = "SPC"; ops.push(#{ SetStatus: "SPC-" }); }
    else if key == ":" { ops.push(#{ RunCommand: "open-palette" }); }
    else if key == "i" { ops.push(#{ SetMode: "insert" }); }
    else if key == "a" { ops.push(#{ Move: 1 }); ops.push(#{ SetMode: "insert" }); }
    else if key == "A" { ops.push("LineEnd"); ops.push(#{ SetMode: "insert" }); }
    else if key == "o" { ops.push("LineEnd"); ops.push(#{ Insert: "\n" }); ops.push(#{ SetMode: "insert" }); }
    else if key == "h" || key == "ArrowLeft" { ops.push(#{ Move: -1 }); }
    else if key == "l" || key == "ArrowRight" { ops.push(#{ Move: 1 }); }
    else if key == "j" || key == "ArrowDown" { ops.push(#{ MoveLine: 1 }); }
    else if key == "k" || key == "ArrowUp" { ops.push(#{ MoveLine: -1 }); }
    else if key == "0" { ops.push("LineStart"); }
    else if key == "$" { ops.push("LineEnd"); }
    else if key == "w" { ops.push("NextWord"); }
    else if key == "b" { ops.push("PrevWord"); }
    else if key == "x" { ops.push(#{ DeleteForward: 1 }); }
    else if key == "d" { state.pending = "d"; }
}
