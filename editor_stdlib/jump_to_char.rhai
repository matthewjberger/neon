// Jump to character, the vim f motion: in normal mode press f then a character
// to jump the caret to its next occurrence on the line. Pairs with a modal
// layer (Vim or Spacemacs).

fn on_key() {
    if mode == "insert" {
        return;
    }
    let pending = if "find_pending" in state { state.find_pending } else { false };
    if pending {
        state.find_pending = false;
        ops.push("Consume");
        ops.push(#{ SetStatus: "" });
        if key.len() == 1 {
            ops.push(#{ FindChar: key });
        }
        return;
    }
    if key == "f" && !ctrl && !alt {
        ops.push("Consume");
        state.find_pending = true;
        ops.push(#{ SetStatus: "f-" });
    }
}
