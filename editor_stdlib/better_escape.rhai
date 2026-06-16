// Better escape: type "jk" in insert mode to leave it, the familiar way to keep
// your hands on the home row. Pairs with the Vim or Spacemacs layer.

fn on_key() {
    if mode != "insert" {
        state.escape_pending = false;
        return;
    }
    let pending = if "escape_pending" in state { state.escape_pending } else { false };
    if pending {
        state.escape_pending = false;
        if key == "k" {
            ops.push("Consume");
            ops.push(#{ DeleteBackward: 1 });
            ops.push(#{ SetMode: "normal" });
            ops.push(#{ Move: -1 });
        }
        return;
    }
    if key == "j" {
        ops.push("Consume");
        ops.push(#{ Insert: "j" });
        state.escape_pending = true;
    }
}
