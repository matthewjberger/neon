// Comment object: in normal mode, gcc toggles the comment on the current line,
// the familiar two-key comment motion. Pairs with a modal layer.

fn on_key() {
    if mode == "insert" {
        return;
    }
    let pending = if "comment" in state { state.comment } else { "" };
    if pending == "g" {
        state.comment = "";
        ops.push("Consume");
        if key == "c" {
            state.comment = "gc";
        }
        return;
    }
    if pending == "gc" {
        state.comment = "";
        ops.push("Consume");
        if key == "c" {
            ops.push(#{ ToggleComment: "//" });
        }
        return;
    }
    if key == "g" && !ctrl && !alt {
        ops.push("Consume");
        state.comment = "g";
    }
}
