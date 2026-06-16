// Smart Home: Home jumps to the first non-whitespace character, and again to the
// true line start, the way most editors do.

fn on_key() {
    if key == "Home" && !ctrl && !alt {
        ops.push("Consume");
        ops.push("SmartLineStart");
    }
}
