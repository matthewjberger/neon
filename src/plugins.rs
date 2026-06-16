//! The plugin model and its persistence. A plugin is a [`PluginSource`]: an id,
//! a name, rhai source, and an enabled flag. The page owns the set, persists it
//! to local storage, and syncs it to the worker. Theme selection persists here
//! too, since it is the other piece of page-local preference.

use protocol::PluginSource;

const PLUGINS_KEY: &str = "neon.plugins";
const EDITOR_PLUGINS_KEY: &str = "neon.editor_plugins";

/// The starter source a fresh plugin opens with.
pub const NEW_TEMPLATE: &str = "// A new plugin. on_start runs once, on_tick every frame.\n\
// Push api Commands to `commands`; read this frame's Events from `events`.\n\
// The standard library adds helpers like commands.cube and hsv.\n\
\n\
fn on_start() {\n    commands.cube([0.0, 0.5, 0.0], RED);\n}\n\
\n\
fn on_tick() {\n}\n";

fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|window| window.local_storage().ok().flatten())
}

/// Loads the saved plugins, or the bundled examples on first run.
pub fn load() -> Vec<PluginSource> {
    let Some(storage) = storage() else {
        return defaults();
    };
    match storage.get_item(PLUGINS_KEY).ok().flatten() {
        Some(text) => serde_json::from_str(&text).unwrap_or_else(|_| defaults()),
        None => defaults(),
    }
}

/// Persists the plugin set.
pub fn save(plugins: &[PluginSource]) {
    if let Some(storage) = storage()
        && let Ok(text) = serde_json::to_string(plugins)
    {
        let _ = storage.set_item(PLUGINS_KEY, &text);
    }
}

/// A fresh plugin with a unique id and the starter template.
pub fn new_plugin(name: &str) -> PluginSource {
    let id = format!("p{}", js_sys::Date::now() as u64);
    PluginSource {
        id,
        name: name.to_string(),
        source: NEW_TEMPLATE.to_string(),
        enabled: true,
    }
}

/// The plugins a first run ships with, to show the shape and the std library.
pub fn defaults() -> Vec<PluginSource> {
    vec![
        PluginSource {
            id: "template".to_string(),
            name: "Template".to_string(),
            source: "// TEMPLATE: how to write a Neon plugin.\n\
//\n\
// A plugin is rhai with two optional hooks:\n\
//   on_start()  runs once when the plugin loads\n\
//   on_tick()   runs every frame\n\
//\n\
// Push api Commands to `commands`; read this frame's Events from `events`.\n\
// Always in scope: dt, time, keys, mouse, named, tagged.\n\
// The standard library (left panel) adds helpers like commands.cube and hsv.\n\
\n\
fn on_start() {\n\
\u{20}   commands.cube([0.0, 0.5, 0.0], hsv(0.6, 0.7, 1.0));\n\
\u{20}   commands.ring(12, 3.0, [1.0, 0.5, 0.2, 1.0]);\n\
}\n\
\n\
fn on_tick() {\n\
\u{20}   if random() < 0.02 {\n\
\u{20}       commands.sphere(random_point(5.0), 0.3, random_color());\n\
\u{20}   }\n\
}\n"
                .to_string(),
            enabled: true,
        },
        PluginSource {
            id: "example-color-grid".to_string(),
            name: "Color Grid".to_string(),
            source: "fn on_start() {\n\
\u{20}   for column in 0..10 {\n\
\u{20}       for row in 0..10 {\n\
\u{20}           let sum = column + row;\n\
\u{20}           let hue = sum.to_float() / 20.0;\n\
\u{20}           let x = column.to_float() - 4.5;\n\
\u{20}           let z = row.to_float() - 4.5;\n\
\u{20}           commands.cube([x, 0.5, z], hsv(hue, 0.7, 1.0));\n\
\u{20}       }\n\
\u{20}   }\n}\n"
                .to_string(),
            enabled: false,
        },
        PluginSource {
            id: "example-confetti".to_string(),
            name: "Confetti Rain".to_string(),
            source: "fn on_tick() {\n\
\u{20}   if random() < 0.2 {\n\
\u{20}       commands.sphere(random_point(6.0), 0.2, random_color());\n\
\u{20}   }\n}\n"
                .to_string(),
            enabled: false,
        },
        PluginSource {
            id: "example-ring".to_string(),
            name: "Pulse Ring".to_string(),
            source: "fn on_start() {\n\
\u{20}   commands.ring(24, 4.0, [0.3, 0.7, 1.0, 1.0]);\n}\n"
                .to_string(),
            enabled: false,
        },
    ]
}

/// Loads the saved editor plugins, or the bundled vim and template on first run.
pub fn load_editor_plugins() -> Vec<PluginSource> {
    let Some(storage) = storage() else {
        return default_editor_plugins();
    };
    match storage.get_item(EDITOR_PLUGINS_KEY).ok().flatten() {
        Some(text) => serde_json::from_str(&text).unwrap_or_else(|_| default_editor_plugins()),
        None => default_editor_plugins(),
    }
}

/// Persists the editor-plugin set.
pub fn save_editor_plugins(plugins: &[PluginSource]) {
    if let Some(storage) = storage()
        && let Ok(text) = serde_json::to_string(plugins)
    {
        let _ = storage.set_item(EDITOR_PLUGINS_KEY, &text);
    }
}

/// The editor plugins a first run ships with: a vim keybindings layer and a
/// commented template. Both start disabled so normal typing works until enabled.
pub fn default_editor_plugins() -> Vec<PluginSource> {
    vec![
        PluginSource {
            id: "vim".to_string(),
            name: "Vim".to_string(),
            source: VIM_SOURCE.to_string(),
            enabled: false,
        },
        PluginSource {
            id: "template-editor".to_string(),
            name: "Editor Template".to_string(),
            source: EDITOR_TEMPLATE.to_string(),
            enabled: false,
        },
    ]
}

const VIM_SOURCE: &str = r#"// Vim keybindings, as an editor plugin. on_key() runs for every keystroke.
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
"#;

const EDITOR_TEMPLATE: &str = r#"// TEMPLATE editor plugin: handle keystrokes in the code editor.
//
// on_key() runs for every keystroke. In scope:
//   key                  the key name ("a", "Enter", "Escape", "ArrowLeft", ...)
//   mode                 the current editor mode label
//   ctrl, shift, alt     modifier booleans
//   ops                  push actions here
//   state                a map that persists across keystrokes
//
// Push to ops:
//   "Consume"                       stop the key from typing normally
//   #{ SetMode: "insert" }          set the mode label
//   #{ Insert: "text" }             insert text at the cursor
//   #{ Move: 1 } / #{ MoveLine: 1 } move the cursor (negative reverses)
//   "LineStart" "LineEnd" "NextWord" "PrevWord"
//   #{ DeleteForward: 1 } #{ DeleteBackward: 1 } "DeleteLine"
//   #{ SetStatus: "..." }
//
// This example inserts a divider comment on Ctrl-/ and otherwise stays out of
// the way, so normal typing is unaffected.

fn on_key() {
    if ctrl && key == "/" {
        ops.push("Consume");
        ops.push(#{ Insert: "// ----------------\n" });
    }
}
"#;
