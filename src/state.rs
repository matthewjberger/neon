//! All page state, grouped as signals. `Copy`, so it threads into every
//! component and closure without cloning. Plain data: no methods beyond the
//! constructor.

use leptos::prelude::*;
use protocol::{CommandInfo, Diagnostic, LogEntry, PluginSource, SelectedEntity, StdModule};

#[derive(Clone, Copy)]
pub struct EditorState {
    pub ready: RwSignal<bool>,
    pub adapter: RwSignal<String>,
    pub fps: RwSignal<f32>,
    pub entity_count: RwSignal<u32>,
    pub selected: RwSignal<Option<SelectedEntity>>,
    /// The api command vocabulary, from the worker's Ready, for highlighting,
    /// the reference, and the console.
    pub commands: RwSignal<Vec<CommandInfo>>,
    /// The standard library modules, for the source viewer and reference.
    pub stdlib: RwSignal<Vec<StdModule>>,
    /// The authored plugins. The page owns this set and syncs it to the worker.
    pub plugins: RwSignal<Vec<PluginSource>>,
    /// The id of the plugin open in the editor, if any.
    pub active: RwSignal<Option<String>>,
    pub log: RwSignal<Vec<LogEntry>>,
    /// Diagnostics for the active plugin, from the language worker.
    pub diagnostics: RwSignal<Vec<Diagnostic>>,
    pub running: RwSignal<bool>,
    pub chat_open: RwSignal<bool>,
    pub reference_open: RwSignal<bool>,
    pub grabbing: RwSignal<bool>,
    /// The active theme id, applied to the document root as `data-theme`.
    pub theme: RwSignal<String>,
}

impl EditorState {
    pub fn new() -> Self {
        let plugins = crate::plugins::load();
        let active = plugins.first().map(|plugin| plugin.id.clone());
        Self {
            ready: RwSignal::new(false),
            adapter: RwSignal::new(String::new()),
            fps: RwSignal::new(0.0),
            entity_count: RwSignal::new(0),
            selected: RwSignal::new(None),
            commands: RwSignal::new(Vec::new()),
            stdlib: RwSignal::new(Vec::new()),
            plugins: RwSignal::new(plugins),
            active: RwSignal::new(active),
            log: RwSignal::new(Vec::new()),
            diagnostics: RwSignal::new(Vec::new()),
            running: RwSignal::new(true),
            chat_open: RwSignal::new(false),
            reference_open: RwSignal::new(false),
            grabbing: RwSignal::new(false),
            theme: RwSignal::new(crate::theme::stored_theme()),
        }
    }

    /// The active plugin's source, or empty when none is open.
    pub fn active_source(&self) -> String {
        let active = self.active.get();
        self.plugins.with(|plugins| {
            plugins
                .iter()
                .find(|plugin| Some(&plugin.id) == active.as_ref())
                .map(|plugin| plugin.source.clone())
                .unwrap_or_default()
        })
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
