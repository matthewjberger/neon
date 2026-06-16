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
            source: "// Falling confetti. Tracks drops in `state` and draws them each frame\n\
// with immediate-mode draw_sphere, so they actually fall.\n\
fn on_tick() {\n\
\u{20}   if !(\"drops\" in state) { state.drops = []; }\n\
\u{20}   if random() < 0.4 {\n\
\u{20}       state.drops.push(#{ x: random_range(-6.0, 6.0), y: 9.0, z: random_range(-6.0, 6.0), c: random_color() });\n\
\u{20}   }\n\
\u{20}   let alive = [];\n\
\u{20}   for drop in state.drops {\n\
\u{20}       drop.y -= dt * 5.0;\n\
\u{20}       if drop.y > 0.0 {\n\
\u{20}           commands.draw_sphere([drop.x, drop.y, drop.z], 0.2, drop.c);\n\
\u{20}           alive.push(drop);\n\
\u{20}       }\n\
\u{20}   }\n\
\u{20}   state.drops = alive;\n}\n"
                .to_string(),
            enabled: false,
        },
        PluginSource {
            id: "example-ring".to_string(),
            name: "Pulse Ring".to_string(),
            source: "// Animated with immediate-mode draw: redrawn every frame so the ring\n\
// breathes with time. draw_ shapes need no entity tracking.\n\
fn on_tick() {\n\
\u{20}   let count = 24;\n\
\u{20}   let radius = 4.0 + sin(time * 2.0) * 0.7;\n\
\u{20}   let step = 6.2831853 / count.to_float();\n\
\u{20}   for index in 0..count {\n\
\u{20}       let angle = index.to_float() * step;\n\
\u{20}       let hue = index.to_float() / count.to_float();\n\
\u{20}       commands.draw_sphere([cos(angle) * radius, 1.0, sin(angle) * radius], 0.3, hsv(hue, 0.7, 1.0));\n\
\u{20}   }\n}\n"
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

const VIM_SOURCE: &str = include_str!("../editor_stdlib/vim.rhai");

const EDITOR_TEMPLATE: &str = include_str!("../editor_stdlib/editor_template.rhai");
