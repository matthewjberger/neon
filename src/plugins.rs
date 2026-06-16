//! The plugin model and its persistence. A plugin is a [`PluginSource`]: an id,
//! a name, rhai source, and an enabled flag. The page owns the set, persists it
//! to local storage, and syncs it to the worker. Theme selection persists here
//! too, since it is the other piece of page-local preference.

use protocol::PluginSource;

const PLUGINS_KEY: &str = "neon.plugins";

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
            enabled: true,
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
