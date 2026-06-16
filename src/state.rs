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

/// One row in the leader menu: the key to press and what it does. A label that
/// starts with `+` is a submenu the key opens.
#[derive(Clone, PartialEq)]
pub struct LeaderItem {
    pub key: String,
    pub label: String,
}

/// The which-key menu an editor plugin publishes for the current leader prefix.
/// The page renders it as the bottom panel while the prefix is pending.
#[derive(Clone, PartialEq)]
pub struct LeaderMenu {
    pub title: String,
    pub items: Vec<LeaderItem>,
}

#[derive(Clone, Copy)]
pub struct EditorState {
    pub ready: RwSignal<bool>,
    /// Whether the worker is rebuilding the scene, for the top progress bar.
    pub busy: RwSignal<bool>,
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
    /// Whether the command palette is open.
    pub palette_open: RwSignal<bool>,
    /// Whether the help and keybindings overlay is open.
    pub help_open: RwSignal<bool>,
    /// The leader menu an editor plugin published for the pending prefix, shown
    /// as the which-key panel. `None` when no leader sequence is active.
    pub leader: RwSignal<Option<LeaderMenu>>,
    /// Whether the editor is split into two panes.
    pub split: RwSignal<bool>,
    /// Split orientation: true lays the panes side by side (split right), false
    /// stacks them (split below).
    pub split_vertical: RwSignal<bool>,
    /// The secondary pane's open buffer when split.
    pub secondary: RwSignal<Option<String>>,
    pub secondary_kind: RwSignal<PluginKind>,
    /// Which pane is focused: false primary, true secondary.
    pub focus_secondary: RwSignal<bool>,
    /// A command id an editor plugin asked the editor to run, applied by the
    /// shell. This is how plugins dictate editor actions.
    pub command_request: RwSignal<Option<String>>,
}

impl EditorState {
    pub fn new() -> Self {
        let plugins = crate::plugins::load();
        let active = plugins.first().map(|plugin| plugin.id.clone());
        Self {
            ready: RwSignal::new(false),
            busy: RwSignal::new(false),
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
            palette_open: RwSignal::new(false),
            help_open: RwSignal::new(false),
            leader: RwSignal::new(None),
            split: RwSignal::new(false),
            split_vertical: RwSignal::new(true),
            secondary: RwSignal::new(None),
            secondary_kind: RwSignal::new(PluginKind::Scene),
            focus_secondary: RwSignal::new(false),
            command_request: RwSignal::new(None),
        }
    }

    /// A buffer's source by kind and id, from the scene set, the editor set, or
    /// the read-only standard library.
    pub fn buffer_source(&self, kind: PluginKind, id: &Option<String>) -> String {
        if kind == PluginKind::Builtin {
            return self.stdlib.with(|modules| {
                modules
                    .iter()
                    .find(|module| Some(&module.name) == id.as_ref())
                    .map(|module| module.source.clone())
                    .unwrap_or_default()
            });
        }
        self.editable_set(kind).with(|plugins| {
            plugins
                .iter()
                .find(|plugin| Some(&plugin.id) == id.as_ref())
                .map(|plugin| plugin.source.clone())
                .unwrap_or_default()
        })
    }

    /// A buffer's display name by kind and id.
    pub fn buffer_name(&self, kind: PluginKind, id: &Option<String>) -> String {
        if kind == PluginKind::Builtin {
            return id.clone().unwrap_or_default();
        }
        self.editable_set(kind).with(|plugins| {
            plugins
                .iter()
                .find(|plugin| Some(&plugin.id) == id.as_ref())
                .map(|plugin| plugin.name.clone())
                .unwrap_or_default()
        })
    }

    /// The editable set for a kind. Built-ins fall back to the scene set and are
    /// never written.
    pub fn editable_set(&self, kind: PluginKind) -> RwSignal<Vec<PluginSource>> {
        if kind == PluginKind::Editor {
            self.editor_plugins
        } else {
            self.plugins
        }
    }

    /// The primary pane's source, used by the agent relay.
    pub fn active_source(&self) -> String {
        self.buffer_source(self.active_kind.get(), &self.active.get())
    }
}

/// Whether a buffer kind is read-only.
pub fn kind_readonly(kind: PluginKind) -> bool {
    kind == PluginKind::Builtin
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
