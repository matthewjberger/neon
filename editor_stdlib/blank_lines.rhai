// Blank lines: in normal mode, ] then space opens a blank line below, [ then
// space opens one above, without leaving normal mode. Pairs with a modal layer.

fn on_key() {
    if mode == "insert" {
        return;
    }
    let pending = if "bracket" in state { state.bracket } else { "" };
    if pending == "]" {
        state.bracket = "";
        ops.push("Consume");
        if key == " " {
            ops.push("LineEnd");
            ops.push(#{ Insert: "\n" });
        }
        return;
    }
    if pending == "[" {
        state.bracket = "";
        ops.push("Consume");
        if key == " " {
            ops.push("LineStart");
            ops.push(#{ Insert: "\n" });
            ops.push(#{ Move: -1 });
        }
        return;
    }
    if key == "]" && !ctrl && !alt {
        ops.push("Consume");
        state.bracket = "]";
    } else if key == "[" && !ctrl && !alt {
        ops.push("Consume");
        state.bracket = "[";
    }
}
