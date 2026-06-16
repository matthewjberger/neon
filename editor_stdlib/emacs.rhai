// Emacs-style keybindings, as an editor plugin. Non-modal: it only acts on the
// Ctrl and Alt chords below and leaves ordinary typing alone. Ctrl+F, Ctrl+S,
// and Ctrl+Z are neon globals (find, save, undo), so this layer leaves them be.

fn on_key() {
    if ctrl {
        if key == "a" { ops.push("Consume"); ops.push("LineStart"); }
        else if key == "e" { ops.push("Consume"); ops.push("LineEnd"); }
        else if key == "b" { ops.push("Consume"); ops.push(#{ Move: -1 }); }
        else if key == "p" { ops.push("Consume"); ops.push(#{ MoveLine: -1 }); }
        else if key == "n" { ops.push("Consume"); ops.push(#{ MoveLine: 1 }); }
        else if key == "d" { ops.push("Consume"); ops.push(#{ DeleteForward: 1 }); }
        else if key == "k" { ops.push("Consume"); ops.push("DeleteToLineEnd"); }
        return;
    }
    if alt {
        if key == "f" { ops.push("Consume"); ops.push("NextWord"); }
        else if key == "b" { ops.push("Consume"); ops.push("PrevWord"); }
        else if key == "d" { ops.push("Consume"); ops.push("DeleteWordForward"); }
        else if key == "Backspace" { ops.push("Consume"); ops.push("DeleteWordBackward"); }
    }
}
