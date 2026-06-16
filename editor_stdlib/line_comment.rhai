// Comment toggle: Ctrl+/ comments or uncomments the current line with "//", the
// usual editor shortcut. It toggles, so a second press removes the marker.

fn on_key() {
    if ctrl && key == "/" {
        ops.push("Consume");
        ops.push(#{ ToggleComment: "//" });
    }
}
