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
            id: "example-wave-grid",
            name: "Wave Grid",
            kind: PluginKind::Scene,
            description: "A grid of spheres rippling like water.",
            source: WAVE_GRID,
        },
        CatalogEntry {
            id: "example-orbits",
            name: "Orbits",
            kind: PluginKind::Scene,
            description: "Bodies orbiting a glowing center, a little solar system.",
            source: ORBITS,
        },
        CatalogEntry {
            id: "example-spiral",
            name: "Spiral",
            kind: PluginKind::Scene,
            description: "A rotating spiral arm climbing upward.",
            source: SPIRAL,
        },
        CatalogEntry {
            id: "example-lissajous",
            name: "Lissajous",
            kind: PluginKind::Scene,
            description: "A Lissajous curve traced in space, drifting with time.",
            source: LISSAJOUS,
        },
        CatalogEntry {
            id: "example-starfield",
            name: "Starfield",
            kind: PluginKind::Scene,
            description: "A warp starfield streaming toward the camera.",
            source: STARFIELD,
        },
        CatalogEntry {
            id: "example-helix",
            name: "Double Helix",
            kind: PluginKind::Scene,
            description: "Two strands winding around each other, rotating.",
            source: HELIX,
        },
        CatalogEntry {
            id: "example-bouncing-balls",
            name: "Bouncing Balls",
            kind: PluginKind::Scene,
            description: "Balls dropped under gravity, bouncing off the floor.",
            source: BOUNCING_BALLS,
        },
        CatalogEntry {
            id: "example-snowfall",
            name: "Snowfall",
            kind: PluginKind::Scene,
            description: "Snow that drifts sideways as it falls.",
            source: SNOWFALL,
        },
        CatalogEntry {
            id: "example-breathing-sphere",
            name: "Breathing Sphere",
            kind: PluginKind::Scene,
            description: "A single sphere easing its radius and hue with time.",
            source: BREATHING_SPHERE,
        },
        CatalogEntry {
            id: "example-fireworks",
            name: "Fireworks",
            kind: PluginKind::Scene,
            description: "Random bursts of sparks that arc out and fade.",
            source: FIREWORKS,
        },
        CatalogEntry {
            id: "spacemacs",
            name: "Spacemacs",
            kind: PluginKind::Editor,
            description: "Modal editing with an SPC leader for windows, toggles, and the palette.",
            source: SPACEMACS,
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
        CatalogEntry {
            id: "emacs",
            name: "Emacs Keys",
            kind: PluginKind::Editor,
            description: "Non-modal Ctrl and Alt motions and kills: Ctrl+A/E/B/N/P/D/K, Alt+F/B/D.",
            source: EMACS,
        },
        CatalogEntry {
            id: "auto-pairs",
            name: "Auto Pairs",
            kind: PluginKind::Editor,
            description: "Insert the matching bracket or quote and keep the caret between them.",
            source: AUTO_PAIRS,
        },
        CatalogEntry {
            id: "better-escape",
            name: "Better Escape",
            kind: PluginKind::Editor,
            description: "Type jk in insert mode to leave it, hands on the home row.",
            source: BETTER_ESCAPE,
        },
        CatalogEntry {
            id: "comment-toggle",
            name: "Comment Toggle",
            kind: PluginKind::Editor,
            description: "Ctrl+/ comments or uncomments the current line.",
            source: COMMENT_TOGGLE,
        },
        CatalogEntry {
            id: "line-tools",
            name: "Line Tools",
            kind: PluginKind::Editor,
            description: "Move (Alt+Up/Down), duplicate, delete, and indent lines, VSCode-style.",
            source: LINE_TOOLS,
        },
        CatalogEntry {
            id: "word-motions",
            name: "Word Delete",
            kind: PluginKind::Editor,
            description: "Ctrl+Backspace and Ctrl+Delete remove the word before or after the caret.",
            source: WORD_MOTIONS,
        },
        CatalogEntry {
            id: "join-lines",
            name: "Join Lines",
            kind: PluginKind::Editor,
            description: "Ctrl+J pulls the next line up onto the current one.",
            source: JOIN_LINES,
        },
        CatalogEntry {
            id: "smart-home",
            name: "Smart Home",
            kind: PluginKind::Editor,
            description: "Home jumps to the first non-whitespace character, then the line start.",
            source: SMART_HOME,
        },
        CatalogEntry {
            id: "jump-to-char",
            name: "Jump to Char",
            kind: PluginKind::Editor,
            description: "The vim f motion: press f then a character to jump to it on the line.",
            source: JUMP_TO_CHAR,
        },
        CatalogEntry {
            id: "blank-lines",
            name: "Blank Lines",
            kind: PluginKind::Editor,
            description: "In normal mode, ] space opens a line below, [ space one above.",
            source: BLANK_LINES,
        },
        CatalogEntry {
            id: "commentary",
            name: "Comment Object",
            kind: PluginKind::Editor,
            description: "The gcc motion: toggle the comment on the current line in normal mode.",
            source: COMMENTARY,
        },
        CatalogEntry {
            id: "move-lines",
            name: "Move Lines",
            kind: PluginKind::Editor,
            description: "Alt+j and Alt+k shuffle the current line down or up.",
            source: MOVE_LINES,
        },
    ]
}

/// The display categories the plugin manager groups by, in order.
pub const CATEGORIES: &[&str] = &[
    "Keybinding layers",
    "Editing",
    "Motions",
    "Comments",
    "Starters",
    "Visuals",
];

/// The group a catalog entry belongs to in the plugin manager.
pub fn category(entry: &CatalogEntry) -> &'static str {
    match entry.kind {
        PluginKind::Scene => {
            if entry.id == "template" {
                "Starters"
            } else {
                "Visuals"
            }
        }
        PluginKind::Editor => match entry.id {
            "spacemacs" | "vim" | "emacs" => "Keybinding layers",
            "template-editor" => "Starters",
            "comment-toggle" | "commentary" => "Comments",
            "jump-to-char" | "smart-home" | "blank-lines" => "Motions",
            _ => "Editing",
        },
        _ => "Editing",
    }
}

/// Whether an editor plugin is a modal keybinding layer. Only one should be
/// enabled at a time, since they each claim the whole keyboard in normal mode.
pub fn is_modal(id: &str) -> bool {
    matches!(id, "vim" | "spacemacs")
}

/// Disables every other modal layer when one is enabled, so they never stack.
pub fn enforce_modal_exclusivity(plugins: &mut [PluginSource], enabled: &str) {
    if !is_modal(enabled) {
        return;
    }
    for plugin in plugins.iter_mut() {
        if plugin.id != enabled && is_modal(&plugin.id) {
            plugin.enabled = false;
        }
    }
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

/// First run installs Spacemacs as the default keybindings; vim and the template
/// are in the manager.
pub fn default_editor_plugins() -> Vec<PluginSource> {
    catalog()
        .iter()
        .filter(|entry| entry.id == "spacemacs")
        .map(entry_to_plugin)
        .collect()
}

const SPACEMACS: &str = include_str!("../editor_stdlib/spacemacs.rhai");
const VIM_SOURCE: &str = include_str!("../editor_stdlib/vim.rhai");
const EDITOR_TEMPLATE: &str = include_str!("../editor_stdlib/editor_template.rhai");
const EMACS: &str = include_str!("../editor_stdlib/emacs.rhai");
const AUTO_PAIRS: &str = include_str!("../editor_stdlib/auto_pairs.rhai");
const BETTER_ESCAPE: &str = include_str!("../editor_stdlib/better_escape.rhai");
const COMMENT_TOGGLE: &str = include_str!("../editor_stdlib/line_comment.rhai");
const LINE_TOOLS: &str = include_str!("../editor_stdlib/line_tools.rhai");
const WORD_MOTIONS: &str = include_str!("../editor_stdlib/word_motions.rhai");
const JOIN_LINES: &str = include_str!("../editor_stdlib/join_lines.rhai");
const SMART_HOME: &str = include_str!("../editor_stdlib/smart_home.rhai");
const JUMP_TO_CHAR: &str = include_str!("../editor_stdlib/jump_to_char.rhai");
const BLANK_LINES: &str = include_str!("../editor_stdlib/blank_lines.rhai");
const COMMENTARY: &str = include_str!("../editor_stdlib/commentary.rhai");
const MOVE_LINES: &str = include_str!("../editor_stdlib/move_lines.rhai");

const WAVE_GRID: &str = include_str!("../examples/wave_grid.rhai");
const ORBITS: &str = include_str!("../examples/orbits.rhai");
const SPIRAL: &str = include_str!("../examples/spiral.rhai");
const LISSAJOUS: &str = include_str!("../examples/lissajous.rhai");
const STARFIELD: &str = include_str!("../examples/starfield.rhai");
const HELIX: &str = include_str!("../examples/helix.rhai");
const BOUNCING_BALLS: &str = include_str!("../examples/bouncing_balls.rhai");
const SNOWFALL: &str = include_str!("../examples/snowfall.rhai");
const BREATHING_SPHERE: &str = include_str!("../examples/breathing_sphere.rhai");
const FIREWORKS: &str = include_str!("../examples/fireworks.rhai");

const TEMPLATE_SCENE: &str = r#"// TEMPLATE: how to write a Neon plugin.
//
// A plugin is rhai with two optional hooks.
//   on_start()  runs once when the plugin loads
//   on_tick()   runs every frame
//
// Push api Commands to `commands` and read this frame's Events from `events`.
// Always in scope: dt, time, keys, mouse, named, tagged.
// The standard library adds helpers like commands.cube and hsv.

fn on_start() {
    commands.cube([0.0, 0.5, 0.0], hsv(0.6, 0.7, 1.0));
}

fn on_tick() {
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
