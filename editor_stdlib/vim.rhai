// Vim keybindings, as an editor plugin. on_key() runs for every keystroke.
// In scope: key, mode, ctrl, shift, alt, ops (push actions), state (persists).
// Edit this live to change the bindings.

fn on_key() {
    if mode == "insert" {
        if key == "Escape" {
            ops.push("Consume");
            ops.push(#{ SetMode: "normal" });
            ops.push(#{ Move: -1 });
        }
        return;
    }

    // Normal mode consumes every key so it never types into the buffer.
    ops.push("Consume");

    let pending = if "pending" in state { state.pending } else { "" };
    if pending == "d" {
        state.pending = "";
        if key == "d" { ops.push("DeleteLine"); }
        return;
    }

    if key == "i" { ops.push(#{ SetMode: "insert" }); }
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
    else if key == "d" { state.pending = "d"; ops.push(#{ SetStatus: "d-" }); }
}
