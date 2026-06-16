// Join lines: Ctrl+J pulls the next line up onto the current one.

fn on_key() {
    if ctrl && key == "j" {
        ops.push("Consume");
        ops.push("JoinLines");
    }
}
