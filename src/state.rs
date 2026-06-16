//! All page state, grouped as signals. `Copy`, so it threads into every
//! component and closure without cloning. Plain data: no methods beyond the
//! constructor.

use leptos::prelude::*;
use protocol::{CommandInfo, Diagnostic, LogEntry, PluginSource, SelectedEntity, StdModule};

/// Which set the open buffer belongs to: scene plugins run in the engine worker,
/// editor plugins run on the page and drive the editor through key dispatch, and
/// built-ins are the standard library, viewable but locked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginKind {
    Scene,
    Editor,
    Builtin,
}

/// Which view the sidebar shows, switched from the activity bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarView {
    Installed,
    Extensions,
}

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
    /// Which set the open buffer belongs to.
    pub active_kind: RwSignal<PluginKind>,
    /// Editor plugins: rhai that handles keystrokes through the Editor API. Run
    /// on the page, never sent to the worker.
    pub editor_plugins: RwSignal<Vec<PluginSource>>,
    /// The current editor input mode an editor plugin owns (e.g. vim's normal or
    /// insert), shown in the toolbar.
    pub editor_mode: RwSignal<String>,
    /// A transient status line an editor plugin can set.
    pub status: RwSignal<String>,
    /// Whether the 3D preview pane is shown. Hiding it gives the editor the full
    /// width, so neon works as a plain code editor.
    pub viewport_open: RwSignal<bool>,
    /// Whether the console pane is shown.
    pub console_open: RwSignal<bool>,
    /// Which view the sidebar shows.
    pub sidebar_view: RwSignal<SidebarView>,
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
            active_kind: RwSignal::new(PluginKind::Scene),
            editor_plugins: RwSignal::new(crate::plugins::load_editor_plugins()),
            editor_mode: RwSignal::new("normal".to_string()),
            status: RwSignal::new(String::new()),
            viewport_open: RwSignal::new(true),
            console_open: RwSignal::new(true),
            sidebar_view: RwSignal::new(SidebarView::Installed),
            log: RwSignal::new(Vec::new()),
            diagnostics: RwSignal::new(Vec::new()),
            running: RwSignal::new(true),
            chat_open: RwSignal::new(false),
            reference_open: RwSignal::new(false),
            grabbing: RwSignal::new(false),
            theme: RwSignal::new(crate::theme::stored_theme()),
        }
    }

    /// The active buffer's source, or empty when none is open. Reads from the
    /// scene set, the editor set, or the read-only standard library.
    pub fn active_source(&self) -> String {
        let active = self.active.get();
        if self.active_kind.get() == PluginKind::Builtin {
            return self.stdlib.with(|modules| {
                modules
                    .iter()
                    .find(|module| Some(&module.name) == active.as_ref())
                    .map(|module| module.source.clone())
                    .unwrap_or_default()
            });
        }
        let signal = self.active_signal();
        signal.with(|plugins| {
            plugins
                .iter()
                .find(|plugin| Some(&plugin.id) == active.as_ref())
                .map(|plugin| plugin.source.clone())
                .unwrap_or_default()
        })
    }

    /// The active buffer's display name.
    pub fn active_name(&self) -> String {
        let active = self.active.get();
        if self.active_kind.get() == PluginKind::Builtin {
            return active.unwrap_or_default();
        }
        let signal = self.active_signal();
        signal.with(|plugins| {
            plugins
                .iter()
                .find(|plugin| Some(&plugin.id) == active.as_ref())
                .map(|plugin| plugin.name.clone())
                .unwrap_or_default()
        })
    }

    /// The signal backing the active editable set. Built-ins are read-only, so
    /// this falls back to the scene set for them and is never written.
    pub fn active_signal(&self) -> RwSignal<Vec<PluginSource>> {
        match self.active_kind.get() {
            PluginKind::Editor => self.editor_plugins,
            _ => self.plugins,
        }
    }

    /// Whether the active buffer is a locked built-in.
    pub fn active_readonly(&self) -> bool {
        self.active_kind.get() == PluginKind::Builtin
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
