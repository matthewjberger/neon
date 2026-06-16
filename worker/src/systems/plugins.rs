//! The plugin runner. The page is the authority on the plugin set; this rebuilds
//! the engine's global scripts from it, prepends the standard library so its
//! helpers are in scope, and runs everything each tick through the facade's
//! `run_scripts`. The commands a plugin produces are applied as one deferred
//! batch, and the tick's traffic is buffered for the console.

use nightshade::ecs::script::components::GlobalScript;
use nightshade::prelude::despawn_recursive_immediate;
use nightshade_api::prelude::*;
use protocol::{LogEntry, LogKind, PluginSource};

use crate::state::Scene;
use crate::stdlib;

/// Replace the plugin set and replay it from a clean stage, so editing or
/// toggling a plugin never leaves the entities a prior run spawned behind.
pub fn set_plugins(scene: &mut Scene, world: &mut World, plugins: Vec<PluginSource>) {
    scene.plugins = plugins;
    reset(scene, world);
}

/// Set the run toggle without unloading anything.
pub fn set_running(scene: &mut Scene, running: bool) {
    scene.running = running;
}

/// Drop everything the plugins spawned and restore the base scene, then reload.
pub fn reset(scene: &mut Scene, world: &mut World) {
    let keep: std::collections::HashSet<Entity> = scene.base.iter().copied().collect();
    let mut current = Vec::new();
    world.core.query().iter(|entity, _, _| current.push(entity));
    for entity in current {
        if keep.contains(&entity) {
            continue;
        }
        let alive = world
            .core
            .entity_locations
            .get(entity.id)
            .is_some_and(|location| {
                location.allocated && location.generation == entity.generation
            });
        if alive {
            despawn_recursive_immediate(world, entity);
        }
    }
    world.resources.mesh_render_state.request_full_rebuild();
    rebuild(scene, world);
}

/// One tick: clear the immediate-draw pools, run the enabled plugins, apply
/// their commands, and buffer the traffic for the console.
pub fn tick(scene: &mut Scene, world: &mut World) {
    clear_draw_pools(world);
    if !scene.running {
        return;
    }
    let report = run_scripts(world, &mut scene.runtime);
    for command in &report.commands {
        let (label, detail) = describe_command(command);
        scene.log.push(LogEntry {
            kind: LogKind::Command,
            label,
            detail,
        });
    }
    for error in &report.errors {
        scene.log.push(LogEntry {
            kind: LogKind::Error,
            label: "error".to_string(),
            detail: error.clone(),
        });
    }
    for event in drain_events(world) {
        scene.log.push(LogEntry {
            kind: LogKind::Event,
            label: event_label(&event),
            detail: String::new(),
        });
    }
    let limit = 200;
    if scene.log.len() > limit {
        let excess = scene.log.len() - limit;
        scene.log.drain(0..excess);
    }
}

fn rebuild(scene: &mut Scene, world: &mut World) {
    let prelude = stdlib::prelude();
    world.resources.global_scripts.entries.clear();
    for plugin in &scene.plugins {
        if !plugin.enabled {
            continue;
        }
        let source = format!("{prelude}\n\n{}", plugin.source);
        world.resources.global_scripts.entries.push(GlobalScript {
            name: plugin.id.clone(),
            source,
            enabled: true,
        });
    }
    script_runtime_reset(&mut scene.runtime);
}

/// A command's console label (its variant) and detail (its argument json).
fn describe_command(command: &Command) -> (String, String) {
    match serde_json::to_value(command) {
        Ok(serde_json::Value::Object(map)) => match map.into_iter().next() {
            Some((variant, body)) => (variant, body.to_string()),
            None => ("command".to_string(), String::new()),
        },
        Ok(serde_json::Value::String(variant)) => (variant, String::new()),
        _ => ("command".to_string(), String::new()),
    }
}

fn event_label(event: &Event) -> String {
    match event {
        Event::Collision { started, .. } => {
            if *started { "collision started" } else { "collision ended" }.to_string()
        }
        Event::Despawned { .. } => "despawned".to_string(),
        Event::AnimationFinished { .. } => "animation finished".to_string(),
        Event::AnimationEvent { name, .. } => format!("animation event: {name}"),
        Event::NavigationArrived { .. } => "navigation arrived".to_string(),
    }
}
