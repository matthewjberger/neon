//! The worker's scene state, carried across frames by the offscreen driver. Plain
//! data plus the script runtime. The engine `State` trait is the one piece of
//! plumbing the offscreen loop needs, so this implements it and forwards each
//! hook to a free function in `systems/`.

use nightshade::prelude::{State, camera_controllers_system};
use nightshade_api::prelude::*;
use protocol::{LogEntry, PluginSource};

use crate::systems;

/// The scene state. Owns the [`ScriptRuntime`] that runs the plugins, the loaded
/// plugin set, the run toggle, the current selection, the base entities the
/// scene starts with (kept across a reset), and the traffic log the render loop
/// drains to the page.
pub struct Scene {
    pub runtime: ScriptRuntime,
    pub plugins: Vec<PluginSource>,
    pub running: bool,
    pub selected: Option<Entity>,
    pub base: Vec<Entity>,
    pub log: Vec<LogEntry>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            runtime: ScriptRuntime::default(),
            plugins: Vec::new(),
            running: true,
            selected: None,
            base: Vec::new(),
            log: Vec::new(),
        }
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl State for Scene {
    fn initialize(&mut self, world: &mut World) {
        systems::setup::initialize(self, world);
    }

    fn run_systems(&mut self, world: &mut World) {
        camera_controllers_system(world);
        systems::plugins::tick(self, world);
    }
}
