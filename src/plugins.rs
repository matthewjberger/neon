//! The plugin model and its persistence. A plugin is a [`PluginSource`]: an id,
//! a name, rhai source, and an enabled flag. The page owns the installed set,
//! persists it to local storage, and syncs scene plugins to the worker.
//!
//! Bundled plugins live in [`catalog`], the source the Extensions manager
//! installs from. The installed sets (scene and editor) are the workspace.

use protocol::PluginSource;

use crate::state::PluginKind;

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

/// One bundled plugin the Extensions manager can install.
pub struct CatalogEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: PluginKind,
    pub description: &'static str,
    pub source: &'static str,
}

/// Every bundled plugin: the scene examples and the editor plugins.
pub fn catalog() -> Vec<CatalogEntry> {
    vec![
        CatalogEntry {
            id: "template",
            name: "Template",
            kind: PluginKind::Scene,
            description: "A commented starter: on_start, on_tick, commands, and the std library.",
            source: TEMPLATE_SCENE,
        },
        CatalogEntry {
            id: "example-color-grid",
            name: "Color Grid",
            kind: PluginKind::Scene,
            description: "A 10 by 10 grid of cubes colored across the hue wheel.",
            source: COLOR_GRID,
        },
        CatalogEntry {
            id: "example-confetti",
            name: "Confetti Rain",
            kind: PluginKind::Scene,
            description: "Falling confetti, drawn each frame so it actually rains.",
            source: CONFETTI,
        },
        CatalogEntry {
            id: "example-ring",
            name: "Pulse Ring",
            kind: PluginKind::Scene,
            description: "A ring of spheres that breathes with time.",
            source: PULSE_RING,
        },
        CatalogEntry {
            id: "vim",
            name: "Vim",
            kind: PluginKind::Editor,
            description: "Vim normal and insert keybindings for the code editor.",
            source: VIM_SOURCE,
        },
        CatalogEntry {
            id: "template-editor",
            name: "Editor Template",
            kind: PluginKind::Editor,
            description: "A starter editor plugin showing the on_key API.",
            source: EDITOR_TEMPLATE,
        },
    ]
}

/// An installed, enabled plugin from a catalog entry.
pub fn entry_to_plugin(entry: &CatalogEntry) -> PluginSource {
    PluginSource {
        id: entry.id.to_string(),
        name: entry.name.to_string(),
        source: entry.source.to_string(),
        enabled: true,
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|window| window.local_storage().ok().flatten())
}

/// Loads the installed scene plugins, or the first-run set.
pub fn load() -> Vec<PluginSource> {
    let Some(storage) = storage() else {
        return defaults();
    };
    match storage.get_item(PLUGINS_KEY).ok().flatten() {
        Some(text) => serde_json::from_str(&text).unwrap_or_else(|_| defaults()),
        None => defaults(),
    }
}

/// Persists the scene-plugin set.
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

/// First run installs just the Template scene plugin; the rest are in the
/// Extensions manager to install.
pub fn defaults() -> Vec<PluginSource> {
    catalog()
        .iter()
        .filter(|entry| entry.id == "template")
        .map(entry_to_plugin)
        .collect()
}

/// Loads the installed editor plugins, or the first-run set (none).
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

/// Editor plugins start uninstalled; vim and the template are in the manager.
pub fn default_editor_plugins() -> Vec<PluginSource> {
    Vec::new()
}

const VIM_SOURCE: &str = include_str!("../editor_stdlib/vim.rhai");
const EDITOR_TEMPLATE: &str = include_str!("../editor_stdlib/editor_template.rhai");

const TEMPLATE_SCENE: &str = r#"// TEMPLATE: how to write a Neon plugin.
//
// A plugin is rhai with two optional hooks:
//   on_start()  runs once when the plugin loads
//   on_tick()   runs every frame
//
// Push api Commands to `commands`; read this frame's Events from `events`.
// Always in scope: dt, time, keys, mouse, named, tagged.
// The standard library adds helpers like commands.cube and hsv.

fn on_start() {
    commands.cube([0.0, 0.5, 0.0], hsv(0.6, 0.7, 1.0));
    commands.ring(12, 3.0, [1.0, 0.5, 0.2, 1.0]);
}

fn on_tick() {
    if random() < 0.02 {
        commands.sphere(random_point(5.0), 0.3, random_color());
    }
}
"#;

const COLOR_GRID: &str = r#"fn on_start() {
    for column in 0..10 {
        for row in 0..10 {
            let sum = column + row;
            let hue = sum.to_float() / 20.0;
            let x = column.to_float() - 4.5;
            let z = row.to_float() - 4.5;
            commands.cube([x, 0.5, z], hsv(hue, 0.7, 1.0));
        }
    }
}
"#;

const CONFETTI: &str = r#"// Falling confetti. Tracks drops in `state` and draws them each frame
// with immediate-mode draw_sphere, so they actually fall.
fn on_tick() {
    if !("drops" in state) { state.drops = []; }
    if random() < 0.4 {
        state.drops.push(#{ x: random_range(-6.0, 6.0), y: 9.0, z: random_range(-6.0, 6.0), c: random_color() });
    }
    let alive = [];
    for drop in state.drops {
        drop.y -= dt * 5.0;
        if drop.y > 0.0 {
            commands.draw_sphere([drop.x, drop.y, drop.z], 0.2, drop.c);
            alive.push(drop);
        }
    }
    state.drops = alive;
}
"#;

const PULSE_RING: &str = r#"// Animated with immediate-mode draw: redrawn every frame so the ring
// breathes with time. draw_ shapes need no entity tracking.
fn on_tick() {
    let count = 24;
    let radius = 4.0 + sin(time * 2.0) * 0.7;
    let step = 6.2831853 / count.to_float();
    for index in 0..count {
        let angle = index.to_float() * step;
        let hue = index.to_float() / count.to_float();
        commands.draw_sphere([cos(angle) * radius, 1.0, sin(angle) * radius], 0.3, hsv(hue, 0.7, 1.0));
    }
}
"#;
