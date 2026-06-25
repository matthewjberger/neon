//! All page state, grouped as signals. `Copy`, so it threads into every
//! component and closure without cloning. Plain data: no methods beyond the
//! constructor.

use leptos::prelude::*;
use protocol::{
    CommandInfo, Diagnostic, LogEntry, LogKind, PluginSource, SearchHit, SelectedEntity, StdModule,
    TermGrid,
};

mod panes;

/// The most console rows kept in [`EditorState::log`]; older rows drop as new
/// ones arrive, so an endlessly running scene cannot grow it without bound.
const LOG_LIMIT: usize = 500;

/// Which set the open buffer belongs to: scene plugins run in the engine worker,
/// editor plugins run on the page and drive the editor through key dispatch,
/// built-ins are the standard library (viewable but locked), and files are real
/// files on disk opened through the desktop filesystem bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginKind {
    Scene,
    Editor,
    Builtin,
    File,
}

/// A file opened from disk: its absolute path, current text, and whether it has
/// unsaved edits.
#[derive(Clone, PartialEq)]
pub struct FileBuffer {
    pub path: String,
    pub text: String,
    pub dirty: bool,
}

/// One node in the file tree: a directory or file, with lazily loaded children
/// and an expanded flag for directories.
#[derive(Clone, PartialEq)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub expanded: bool,
    pub children: Vec<TreeNode>,
}

/// Which view the sidebar shows, switched from the activity bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarView {
    Installed,
    Extensions,
    Files,
    Search,
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

/// A reference to an open buffer: which set it belongs to and its id (a plugin
/// id, a built-in module name, or a file path).
#[derive(Clone, PartialEq)]
pub struct BufferRef {
    pub kind: PluginKind,
    pub id: Option<String>,
}

/// One completion candidate: its display label and the text to insert.
#[derive(Clone, PartialEq)]
pub struct CompletionEntry {
    pub label: String,
    pub insert: String,
    pub detail: String,
    pub kind: String,
}

/// The completion popup: the candidates, the caret pixel anchor, and the typed
/// prefix the accepted item replaces.
#[derive(Clone, PartialEq)]
pub struct CompletionMenu {
    pub items: Vec<CompletionEntry>,
    pub x: f64,
    pub y: f64,
    pub prefix: String,
}

/// A hover card: its text and pixel anchor.
#[derive(Clone, PartialEq)]
pub struct HoverCard {
    pub text: String,
    pub x: f64,
    pub y: f64,
}

/// One jump target: the label to type, its pixel anchor, and the UTF-16 caret
/// offset to jump to.
#[derive(Clone, PartialEq)]
pub struct JumpTarget {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub offset: u32,
}

/// Active jump mode: the labeled targets and the label prefix typed so far. When
/// `awaiting_char` is set, the next keystroke chooses the character to jump to
/// and the targets are built then.
#[derive(Clone, PartialEq)]
pub struct JumpState {
    pub targets: Vec<JumpTarget>,
    pub pending: String,
    pub awaiting_char: bool,
}

/// One editor pane: a stable key, its open buffers as tabs with an active index,
/// and its flex-grow weight in the split. Plain data, held in a `Vec`, so any
/// number of panes can stack and each can hold any number of tabs.
#[derive(Clone, PartialEq)]
pub struct Pane {
    pub key: usize,
    pub tabs: Vec<BufferRef>,
    pub active: usize,
    pub flex: f32,
}

impl Pane {
    /// The buffer the active tab points to, if any.
    pub fn buffer(&self) -> Option<&BufferRef> {
        self.tabs.get(self.active)
    }
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
    /// The open editor panes, in layout order. Always at least one. Splitting
    /// appends a pane next to the focused one, closing removes the focused one.
    pub panes: RwSignal<Vec<Pane>>,
    /// The key of the focused pane.
    pub focused_key: RwSignal<usize>,
    /// Files opened from disk through the desktop filesystem bridge.
    pub files: RwSignal<Vec<FileBuffer>>,
    /// The opened workspace folder, the LSP root, if any.
    pub workspace_root: RwSignal<Option<String>>,
    /// The file tree under the workspace root.
    pub tree: RwSignal<Vec<TreeNode>>,
    /// Project search results.
    pub search_results: RwSignal<Vec<SearchHit>>,
    /// A pending jump to a file and 1-based line, applied when the file opens.
    pub goto: RwSignal<Option<(String, u32)>>,
    /// Whether the language server has been started for this session.
    pub lsp_started: RwSignal<bool>,
    /// Whether the consent toast asking to start rust-analyzer is showing.
    pub lsp_consent: RwSignal<bool>,
    /// The language-server log lines, for the LSP log panel.
    pub lsp_log: RwSignal<Vec<String>>,
    /// Whether the LSP log panel is shown.
    pub lsp_log_open: RwSignal<bool>,
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
    /// Whether the find and replace bar is open.
    pub find_open: RwSignal<bool>,
    /// The LSP completion popup, when open.
    pub completion: RwSignal<Option<CompletionMenu>>,
    /// The highlighted completion candidate.
    pub completion_index: RwSignal<usize>,
    /// The LSP hover card, when shown.
    pub hover: RwSignal<Option<HoverCard>>,
    /// Active jump mode (avy-style labeled motion), when on.
    pub jump: RwSignal<Option<JumpState>>,
    /// Whether the help and keybindings overlay is open.
    pub help_open: RwSignal<bool>,
    /// The leader menu an editor plugin published for the pending prefix, shown
    /// as the which-key panel. `None` when no leader sequence is active.
    pub leader: RwSignal<Option<LeaderMenu>>,
    /// Split orientation: true lays the panes side by side (split right), false
    /// stacks them (split below).
    pub split_vertical: RwSignal<bool>,
    /// A command id an editor plugin asked the editor to run, applied by the
    /// shell. This is how plugins dictate editor actions.
    pub command_request: RwSignal<Option<String>>,
    /// Whether the control panel is open: the master surface for dispatching any
    /// command and watching the api log.
    pub control_panel_open: RwSignal<bool>,
    /// The custom right-click menu, when open, with its anchor and items.
    pub context_menu: RwSignal<Option<ContextMenu>>,
    /// The rename prompt's current text, when the rename box is open.
    pub rename: RwSignal<Option<String>>,
    /// The titles of the code actions offered for the caret, when the picker is
    /// open. Empty means closed; the index selects the action.
    pub code_actions: RwSignal<Vec<String>>,
    /// The document symbols offered in the fuzzy symbol picker. Empty means
    /// closed; selecting one jumps to it.
    pub symbol_picker: RwSignal<Vec<SearchHit>>,
    /// Whether saving a Rust file formats it first through rust-analyzer.
    pub format_on_save: RwSignal<bool>,
    /// Every diagnostic across open files, by path, for the problems panel.
    pub problems: RwSignal<Vec<(String, Diagnostic)>>,
    /// Whether the problems panel is shown.
    pub problems_open: RwSignal<bool>,
    /// The tree path a right-click targeted, with whether it is a directory.
    pub context_target: RwSignal<Option<(String, bool)>>,
    /// The open text prompt (new file, rename, delete), when one is showing.
    pub prompt: RwSignal<Option<Prompt>>,
    /// The terminal emulator's current screen, when a PTY is open.
    pub term_grid: RwSignal<Option<TermGrid>>,
    /// A command queued to run once the PTY's shell is ready.
    pub term_pending: RwSignal<Option<String>>,
    /// Whether the terminal panel is shown.
    pub terminal_open: RwSignal<bool>,
    /// Whether the page is connected to the desktop terminal relay.
    pub term_connected: RwSignal<bool>,
    /// Extra caret offsets (UTF-16) for multi-cursor editing, beyond the
    /// textarea's own caret. Empty when not in multi-cursor mode.
    pub cursors: RwSignal<Vec<u32>>,
    /// A tick bumped when the editor scrolls, so caret overlays reposition.
    pub editor_scroll: RwSignal<u32>,
}

/// A custom right-click menu: where it sits and the commands it offers.
#[derive(Clone)]
pub struct ContextMenu {
    pub x: f64,
    pub y: f64,
    pub items: Vec<(String, crate::commands::EditorCommand)>,
}

/// What a text prompt does when confirmed.
#[derive(Clone)]
pub enum PromptAction {
    CreateFile { dir: String },
    RenameEntry { from: String },
    DeleteEntry { path: String },
}

/// A small text prompt: a title, the editable value, and the action to run.
#[derive(Clone)]
pub struct Prompt {
    pub title: String,
    pub value: String,
    pub action: PromptAction,
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
            panes: RwSignal::new(vec![Pane {
                key: 0,
                tabs: vec![BufferRef {
                    kind: PluginKind::Scene,
                    id: active,
                }],
                active: 0,
                flex: 1.0,
            }]),
            focused_key: RwSignal::new(0),
            files: RwSignal::new(Vec::new()),
            workspace_root: RwSignal::new(None),
            tree: RwSignal::new(Vec::new()),
            search_results: RwSignal::new(Vec::new()),
            goto: RwSignal::new(None),
            lsp_started: RwSignal::new(false),
            lsp_consent: RwSignal::new(false),
            lsp_log: RwSignal::new(Vec::new()),
            lsp_log_open: RwSignal::new(false),
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
            find_open: RwSignal::new(false),
            completion: RwSignal::new(None),
            completion_index: RwSignal::new(0),
            hover: RwSignal::new(None),
            jump: RwSignal::new(None),
            help_open: RwSignal::new(false),
            leader: RwSignal::new(None),
            split_vertical: RwSignal::new(true),
            command_request: RwSignal::new(None),
            control_panel_open: RwSignal::new(false),
            context_menu: RwSignal::new(None),
            rename: RwSignal::new(None),
            code_actions: RwSignal::new(Vec::new()),
            symbol_picker: RwSignal::new(Vec::new()),
            format_on_save: RwSignal::new(true),
            problems: RwSignal::new(Vec::new()),
            problems_open: RwSignal::new(false),
            context_target: RwSignal::new(None),
            prompt: RwSignal::new(None),
            term_grid: RwSignal::new(None),
            term_pending: RwSignal::new(None),
            terminal_open: RwSignal::new(false),
            term_connected: RwSignal::new(false),
            cursors: RwSignal::new(Vec::new()),
            editor_scroll: RwSignal::new(0),
        }
    }

    /// Appends entries to the unified console log, dropping the oldest rows past
    /// [`LOG_LIMIT`]. Every writer (the worker report, plugin errors, and editor
    /// commands) goes through here, so the cap is one number.
    pub fn record_log(&self, entries: impl IntoIterator<Item = LogEntry>) {
        self.log.update(|log| {
            log.extend(entries);
            let overflow = log.len().saturating_sub(LOG_LIMIT);
            if overflow > 0 {
                log.drain(0..overflow);
            }
        });
    }

    /// Records one api call or event into the unified log the console and the
    /// control panel both read.
    pub fn log_api(&self, kind: LogKind, label: impl Into<String>, detail: impl Into<String>) {
        self.record_log([LogEntry {
            kind,
            label: label.into(),
            detail: detail.into(),
        }]);
    }

    /// A buffer's source by kind and id, from the scene set, the editor set, the
    /// read-only standard library, or an open file.
    pub fn buffer_source(&self, kind: PluginKind, id: &Option<String>) -> String {
        match kind {
            PluginKind::Builtin => self.stdlib.with(|modules| {
                modules
                    .iter()
                    .find(|module| Some(&module.name) == id.as_ref())
                    .map(|module| module.source.clone())
                    .unwrap_or_default()
            }),
            PluginKind::File => self.files.with(|files| {
                files
                    .iter()
                    .find(|file| Some(&file.path) == id.as_ref())
                    .map(|file| file.text.clone())
                    .unwrap_or_default()
            }),
            _ => self.editable_set(kind).with(|plugins| {
                plugins
                    .iter()
                    .find(|plugin| Some(&plugin.id) == id.as_ref())
                    .map(|plugin| plugin.source.clone())
                    .unwrap_or_default()
            }),
        }
    }

    /// A buffer's display name by kind and id.
    pub fn buffer_name(&self, kind: PluginKind, id: &Option<String>) -> String {
        match kind {
            PluginKind::Builtin => id.clone().unwrap_or_default(),
            PluginKind::File => id
                .as_ref()
                .map(|path| basename(path).to_string())
                .unwrap_or_default(),
            _ => self.editable_set(kind).with(|plugins| {
                plugins
                    .iter()
                    .find(|plugin| Some(&plugin.id) == id.as_ref())
                    .map(|plugin| plugin.name.clone())
                    .unwrap_or_default()
            }),
        }
    }

    /// Writes a buffer's text into the right store. Files mark dirty; plugins
    /// update their source. Built-ins are read-only and ignored.
    pub fn set_buffer_text(&self, kind: PluginKind, id: &Option<String>, text: String) {
        let Some(key) = id.as_ref() else {
            return;
        };
        let old = self.buffer_source(kind, id);
        if old == text {
            return;
        }
        crate::undo::record(kind, id, &old);
        match kind {
            PluginKind::Builtin => {}
            PluginKind::File => self.files.update(|files| {
                if let Some(file) = files.iter_mut().find(|file| &file.path == key) {
                    file.text = text;
                    file.dirty = true;
                }
            }),
            _ => self.editable_set(kind).update(|plugins| {
                if let Some(plugin) = plugins.iter_mut().find(|plugin| &plugin.id == key) {
                    plugin.source = text;
                }
            }),
        }
    }

    /// Whether a file buffer has unsaved edits.
    pub fn is_dirty(&self, kind: PluginKind, id: &Option<String>) -> bool {
        if kind != PluginKind::File {
            return false;
        }
        self.files.with(|files| {
            files
                .iter()
                .find(|file| Some(&file.path) == id.as_ref())
                .map(|file| file.dirty)
                .unwrap_or(false)
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
}

/// Whether a buffer kind is read-only.
pub fn kind_readonly(kind: PluginKind) -> bool {
    kind == PluginKind::Builtin
}

/// The final path component of a file path, for the tab and status bar.
pub fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// A language id from a file's extension, for highlighting and LSP routing.
pub fn language_for_path(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "rs" => "rust",
        "toml" => "toml",
        "md" | "markdown" => "markdown",
        "json" => "json",
        "rhai" => "rhai",
        "js" | "mjs" => "javascript",
        "ts" => "typescript",
        "wgsl" => "wgsl",
        "css" => "css",
        "html" => "html",
        _ => "plaintext",
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
