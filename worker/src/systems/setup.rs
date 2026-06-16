//! Builds the base scene through the facade: sky, grid, sun, an orbit camera, the
//! immediate-draw pools, physics, and retained UI so script-driven screen text
//! shows. The selection outline is an engine setting the facade does not expose,
//! so it is set directly. The entities present after this are the scene's base,
//! kept across a reset.

use crate::state::Scene;
use nightshade::prelude::{load_procedural_textures, spawn_sun};
use nightshade_api::prelude::*;

pub fn initialize(scene: &mut Scene, world: &mut World) {
    if let Some((width, height)) = world.resources.window.cached_viewport_size {
        world.resources.window.active_viewport_rect =
            Some(nightshade::ecs::window::resources::ViewportRect {
                x: 0.0,
                y: 0.0,
                width: width as f32,
                height: height as f32,
            });
    }

    set_background(world, Background::Nebula);
    show_grid(world, true);
    world.resources.debug_draw.selection_outline_enabled = true;
    world.resources.debug_draw.selection_outline_color = [1.0, 0.5, 0.15, 1.0];
    world.resources.physics.enabled = true;
    world.resources.user_interface.enabled = true;
    world.resources.retained_ui.enabled = true;

    load_procedural_textures(world);
    spawn_sun(world);
    orbit_camera(world, vec3(0.0, 0.5, 0.0), 8.0);
    initialize_draw_pools(world);

    scene.base = live_entities(world);
}

/// Every live entity right now, the snapshot a reset restores to.
pub fn live_entities(world: &World) -> Vec<Entity> {
    let mut entities = Vec::new();
    world
        .core
        .query()
        .iter(|entity, _, _| entities.push(entity));
    entities
}
