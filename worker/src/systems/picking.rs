//! Synchronous ray pick at a pixel, the facade's `entity_under_cursor`. Moves the
//! cursor there first, then selects the result and syncs the engine's outline.

use crate::state::Scene;
use nightshade::prelude::input_inject_cursor_moved;
use nightshade_api::prelude::*;
use protocol::SelectedEntity;

pub fn pick(scene: &mut Scene, world: &mut World, x: f32, y: f32) -> Option<SelectedEntity> {
    input_inject_cursor_moved(world, Vec2::new(x.max(0.0), y.max(0.0)));
    let entity = entity_under_cursor(world);
    select(scene, world, entity);
    entity.map(|entity| SelectedEntity {
        id: entity.id,
        name: world
            .core
            .get_name(entity)
            .map(|name| name.0.clone())
            .unwrap_or_default(),
    })
}

pub fn select(scene: &mut Scene, world: &mut World, entity: Option<Entity>) {
    scene.selected = entity;
    world
        .resources
        .editor_selection
        .bounding_volume_selected_entity = entity;
    world.resources.editor_selection.selected_entities = entity.into_iter().collect();
}
