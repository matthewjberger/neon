//! The wire format shared across Neon's contexts.
//!
//! Three seams cross here. The page talks to the engine worker with
//! [`ClientMessage`] and [`WorkerMessage`], the rendering and scene-plugin side.
//! The page talks to the language worker with [`LangRequest`] and
//! [`LangResponse`], the compile-check and validation side. The agent bridge
//! carries [`AgentRequest`] and [`AgentResponse`] between the desktop MCP server
//! and whichever context owns the asked-for state. Everything is serde, so
//! nothing hand-marshals across a boundary.

use serde::{Deserialize, Serialize};
use serde_json::Value;

mod diff;
mod lint;
pub use diff::{DiffLine, Hunk, LineChange, diff_lines, hunks};
pub use lint::{RHAI_BUILTINS, unknown_command_calls};

/// Envelope field carrying the serialized message in every `postMessage`.
pub const MESSAGE_KEY: &str = "message";
/// Envelope field carrying the transferred `OffscreenCanvas` (on `Init` only).
pub const CANVAS_KEY: &str = "canvas";

/// Lifecycle phase of a forwarded touch contact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

/// A scene plugin as the page and worker exchange it. The page is the authority:
/// it owns the set, persists it, and syncs the whole list to the worker.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginSource {
    pub id: String,
    pub name: String,
    pub source: String,
    pub enabled: bool,
}

/// Page to engine worker.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Sent once with the `OffscreenCanvas` in the transfer list.
    Init {
        width: f32,
        height: f32,
    },
    Resize {
        width: f32,
        height: f32,
    },
    PointerMove {
        x: f32,
        y: f32,
    },
    PointerButton {
        button: u8,
        pressed: bool,
    },
    Wheel {
        delta: f32,
    },
    Touch {
        id: u64,
        phase: TouchPhase,
        x: f32,
        y: f32,
    },
    Key {
        code: String,
        pressed: bool,
        text: Option<String>,
    },
    /// A click without drag: pick and select the entity at this position.
    Pick {
        x: f32,
        y: f32,
    },
    /// Replace the whole scene-plugin set. The worker rebuilds its global
    /// scripts from the enabled plugins and resets the runtime.
    SetPlugins {
        plugins: Vec<PluginSource>,
    },
    /// Run one command built in the console, as a json `Command`.
    SubmitCommand {
        command: String,
    },
    /// Drop everything the plugins spawned and restore the base scene.
    ResetScene,
    /// Pause or resume the plugin runtime without unloading anything.
    SetRunning {
        running: bool,
    },
    /// An agent request routed to the worker (scene domain). The page relays it
    /// from the MCP bridge.
    Agent(Box<AgentRequest>),
}

/// The selected entity, reported after a pick resolves.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelectedEntity {
    pub id: u32,
    pub name: String,
}

/// One field of a command, mirrored from the api manifest for the language
/// service: the argument name, its wire type, and the dispatch role.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub type_name: String,
    pub role: String,
}

/// One command method a script can call, mirrored from the api manifest. Drives
/// the editor's highlighting, completion, hover, and reference.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandInfo {
    pub method: String,
    pub variant: String,
    pub description: String,
    pub fields: Vec<FieldInfo>,
    pub reply: String,
}

/// One standard-library helper, for completion and the reference.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StdHelper {
    pub name: String,
    pub signature: String,
    pub description: String,
    /// `commands`, `events`, or `free`: how the helper is called.
    pub receiver: String,
}

/// One standard-library module: its name, full source for the in-editor viewer,
/// and the helpers it defines.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StdModule {
    pub name: String,
    pub source: String,
    pub helpers: Vec<StdHelper>,
}

/// Severity of a diagnostic, mapped to the editor gutter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
}

/// A diagnostic on a plugin's source. Line and column are 1-based, 0 when the
/// position is unknown.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    pub message: String,
    pub line: u32,
    pub column: u32,
    pub severity: Severity,
}

/// What one console log row is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogKind {
    Command,
    Event,
    Error,
}

/// One row of the command-and-event console.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub kind: LogKind,
    pub label: String,
    pub detail: String,
}

/// Engine worker to page.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkerMessage {
    /// The renderer is up. Carries the command manifest, the command json
    /// schema, and the standard library, so the page can stand up its language
    /// service and source viewer without linking the engine.
    Ready {
        adapter: String,
        commands: Vec<CommandInfo>,
        command_schema: String,
        stdlib: Vec<StdModule>,
    },
    Stats {
        fps: f32,
        entity_count: u32,
    },
    /// The worker is rebuilding the scene from the plugin set. Drives the top
    /// progress bar. Posted around a reset or a plugin sync.
    Busy {
        active: bool,
    },
    Selected {
        detail: Option<SelectedEntity>,
    },
    /// The traffic from the last script tick, for the console.
    Report {
        entries: Vec<LogEntry>,
    },
    /// A plugin runtime error, attributed to a plugin id when known.
    PluginError {
        plugin: Option<String>,
        message: String,
    },
    /// An agent response routed back to the page, to forward to the MCP bridge.
    Agent(Box<AgentResponse>),
}

/// Page to language worker.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LangRequest {
    /// Seed the worker with the command and standard-library vocabulary so it
    /// can flag unknown calls. Sent once after the engine worker is ready.
    Init {
        commands: Vec<CommandInfo>,
        stdlib: Vec<StdModule>,
    },
    /// Compile-check and validate a plugin's source. The reply is keyed by the
    /// same request id, so a stale check can be discarded.
    Check { request_id: u32, source: String },
}

/// Language worker to page.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LangResponse {
    Ready,
    Diagnostics {
        request_id: u32,
        diagnostics: Vec<Diagnostic>,
    },
}

/// One entry in a directory listing, for the file tree.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

/// One project-search match: the file, the 1-based line, and the line's text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub path: String,
    pub line: u32,
    pub text: String,
}

/// Page to the desktop filesystem bridge. The page has no disk access, so every
/// file operation crosses this seam to the native shell.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FsRequest {
    /// Open the native folder picker and list its root.
    OpenFolder { request_id: u64 },
    /// Open a known folder by path and list its root, no dialog. Used to restore
    /// the last session.
    OpenRoot { request_id: u64, path: String },
    /// List one directory's immediate children.
    ListDir { request_id: u64, path: String },
    /// Read a file's full text.
    ReadFile { request_id: u64, path: String },
    /// Write a file's full text.
    WriteFile {
        request_id: u64,
        path: String,
        text: String,
    },
    /// Search the workspace for a substring, respecting gitignore.
    Search {
        request_id: u64,
        root: String,
        query: String,
    },
    /// Create an empty file at a path.
    CreatePath { request_id: u64, path: String },
    /// Rename or move a path.
    RenamePath {
        request_id: u64,
        from: String,
        to: String,
    },
    /// Delete a file.
    DeletePath { request_id: u64, path: String },
    /// Replace every match of a regex across the workspace, respecting gitignore.
    ReplaceAll {
        request_id: u64,
        root: String,
        query: String,
        replacement: String,
    },
}

/// Desktop filesystem bridge to the page.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FsResponse {
    Folder {
        request_id: u64,
        root: Option<String>,
        entries: Vec<DirEntry>,
    },
    Dir {
        request_id: u64,
        path: String,
        entries: Vec<DirEntry>,
    },
    File {
        request_id: u64,
        path: String,
        text: String,
    },
    Wrote {
        request_id: u64,
        path: String,
    },
    SearchResults {
        request_id: u64,
        hits: Vec<SearchHit>,
    },
    /// A path was created. `dir` and `entries` refresh the parent in the tree.
    Created {
        request_id: u64,
        path: String,
        dir: String,
        entries: Vec<DirEntry>,
    },
    /// A path was renamed. `dir` and `entries` refresh the parent in the tree.
    Renamed {
        request_id: u64,
        from: String,
        to: String,
        dir: String,
        entries: Vec<DirEntry>,
    },
    /// A path was deleted. `dir` and `entries` refresh the parent in the tree.
    Deleted {
        request_id: u64,
        path: String,
        dir: String,
        entries: Vec<DirEntry>,
    },
    /// A project-wide replace finished, touching `count` files.
    Replaced {
        request_id: u64,
        count: usize,
    },
    Error {
        request_id: u64,
        message: String,
    },
}

/// Page to the desktop language-server bridge. The page is the LSP client; the
/// desktop spawns and frames the server over stdio, choosing it by `language`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LspClientMessage {
    /// Discover and spawn the server for a workspace root uri and language family
    /// (`rust`, `typescript`, `python`, `go`, `cpp`).
    Start { root_uri: String, language: String },
    /// Forward one JSON-RPC message to the server's stdin.
    Rpc { json: String },
    /// Stop the server.
    Stop,
}

/// Desktop language-server bridge to the page.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LspServerMessage {
    /// The server spawned and its stdio is bridged.
    Started,
    /// One JSON-RPC message from the server's stdout.
    Rpc { json: String },
    /// A log line from the server's stderr or the bridge itself, for the LSP
    /// log panel.
    Log { line: String },
    /// Discovery, spawn, or relay failed.
    Error { message: String },
    /// The server process exited.
    Exited { code: Option<i32> },
}

/// Page to the desktop git bridge. Git runs natively in the shell, so the page
/// asks for a file's working-tree diff against HEAD and gets back the changed
/// line numbers to mark in the gutter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GitClientMessage {
    /// Compute the diff of a file against HEAD.
    DiffFile { request_id: u64, path: String },
    /// List the working-tree status of a repo.
    Status { request_id: u64, root: String },
    /// Stage a path.
    Stage {
        request_id: u64,
        root: String,
        path: String,
    },
    /// Unstage a path.
    Unstage {
        request_id: u64,
        root: String,
        path: String,
    },
    /// Commit the staged changes with a message.
    Commit {
        request_id: u64,
        root: String,
        message: String,
    },
}

/// One changed path in a repo's status.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitFile {
    pub path: String,
    /// Whether the change is staged (in the index).
    pub staged: bool,
    /// The git status letter (`M`, `A`, `D`, `?`, ...).
    pub status: String,
}

/// The kind of change on a gutter line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitChange {
    Added,
    Modified,
    Removed,
}

/// Desktop git bridge to the page: a file's changed lines, 1-based. `Removed`
/// marks the line below a deletion.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GitServerMessage {
    Diff {
        request_id: u64,
        path: String,
        changes: Vec<(u32, GitChange)>,
    },
    /// The repo's branch and changed files.
    Status {
        request_id: u64,
        branch: String,
        files: Vec<GitFile>,
    },
    /// A stage, unstage, or commit finished; the page re-reads status.
    Done { request_id: u64 },
}

/// One highlighted run of source: a half-open UTF-8 byte range over the
/// request's `text` and the CSS class the page paints it with. The bridge emits
/// the runs in order and may leave gaps, which the page renders as plain text.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HighlightSpan {
    pub start: u32,
    pub end: u32,
    pub class: String,
}

/// Page to the desktop syntax-highlight bridge. Tree-sitter and its grammars are
/// C, so they run natively in the shell rather than in the page's wasm. The page
/// sends a buffer's language and source and gets back token spans, the same
/// request/response shape every other desktop bridge uses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HighlightClientMessage {
    /// Highlight one buffer. The reply carries the same `request_id`, so the page
    /// can drop a stale response after a newer edit.
    Highlight {
        request_id: u32,
        language: String,
        text: String,
    },
}

/// Desktop syntax-highlight bridge to the page.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HighlightServerMessage {
    /// The token spans for a request. Empty when the language has no grammar, so
    /// the page falls back to its own scanner.
    Tokens {
        request_id: u32,
        spans: Vec<HighlightSpan>,
    },
}

/// Page to the desktop terminal: open a real PTY, send keystrokes, and resize.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TerminalClientMessage {
    /// Open a PTY of the given size in a working directory.
    Open { cols: u16, rows: u16, cwd: String },
    /// Raw bytes to write to the PTY (encoded keystrokes).
    Input { bytes: Vec<u8> },
    /// Resize the PTY and the emulator.
    Resize { cols: u16, rows: u16 },
}

/// One run of cells sharing a style on a terminal row.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TermSpan {
    pub text: String,
    /// CSS color, or empty for the theme default.
    pub fg: String,
    pub bg: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

/// The emulator's rendered screen: rows of styled spans plus the cursor.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TermGrid {
    pub cols: u16,
    pub rows: u16,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub cursor_visible: bool,
    pub lines: Vec<Vec<TermSpan>>,
}

/// Desktop terminal to the page.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TerminalServerMessage {
    Grid(TermGrid),
    Exited,
}

/// Correlation id matching an [`AgentResponse`] to its [`AgentRequest`].
pub type CorrelationId = u64;

/// MCP bridge to the editor. The neon agent surface spans two domains: the
/// scene (entities, screenshot, render state) answered by the engine worker, and
/// the editor (buffers, panels, tiles, plugin source, selection text) answered
/// by the page. The page routes each request to the right owner.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentRequest {
    /// Read editor-wide state: open buffers, panels, tiles, active plugin,
    /// selection, and scene summary. The page answers.
    GetEditorState { correlation_id: CorrelationId },
    /// Read a buffer's full text by id, or the active buffer when `None`.
    GetBuffer {
        correlation_id: CorrelationId,
        buffer: Option<String>,
    },
    /// Replace a buffer's text. The page applies it and re-syncs plugins if the
    /// buffer is a plugin.
    SetBuffer {
        correlation_id: CorrelationId,
        buffer: String,
        text: String,
    },
    /// Propose a new full text for a buffer for review. The page stages it as a
    /// diff the user accepts or rejects, rather than applying it outright.
    ProposeEdit {
        correlation_id: CorrelationId,
        buffer: String,
        text: String,
    },
    /// List the plugins with their enabled state and ids. The page answers.
    ListPlugins { correlation_id: CorrelationId },
    /// The scripting API: every command (method, fields, reply) and every
    /// standard-library helper (name, signature, receiver). The page answers, so
    /// the agent can learn the API without reading source.
    GetApiReference { correlation_id: CorrelationId },
    /// The recent console traffic: the commands, events, and errors from the last
    /// ticks. The page answers, so the agent can see runtime errors and output.
    GetConsole { correlation_id: CorrelationId },
    /// Create, update, or toggle a plugin. The page answers and re-syncs.
    EditPlugin {
        correlation_id: CorrelationId,
        plugin: PluginSource,
    },
    /// Run one api `Command` as json against the scene. The worker answers.
    RunCommand {
        correlation_id: CorrelationId,
        command: String,
    },
    /// Query entities carrying every named component. The worker answers.
    QueryScene {
        correlation_id: CorrelationId,
        components: Vec<String>,
    },
    /// Capture the rendered viewport as a base64 PNG. The worker answers.
    Screenshot {
        correlation_id: CorrelationId,
        max_dimension: Option<u32>,
    },
}

/// The editor or worker's answer to an [`AgentRequest`], carried back to the MCP
/// bridge and on to Claude.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentResponse {
    EditorState {
        correlation_id: CorrelationId,
        state: Value,
    },
    Buffer {
        correlation_id: CorrelationId,
        text: String,
    },
    Plugins {
        correlation_id: CorrelationId,
        plugins: Vec<PluginSource>,
    },
    /// The scripting API reference: commands and standard-library helpers.
    Reference {
        correlation_id: CorrelationId,
        reference: Value,
    },
    /// The recent console traffic.
    Console {
        correlation_id: CorrelationId,
        entries: Vec<LogEntry>,
    },
    /// The diagnostics for a buffer after an edit: empty means it compiled clean.
    Diagnostics {
        correlation_id: CorrelationId,
        diagnostics: Vec<Diagnostic>,
    },
    Ok {
        correlation_id: CorrelationId,
    },
    Scene {
        correlation_id: CorrelationId,
        result: Value,
    },
    Screenshot {
        correlation_id: CorrelationId,
        width: u32,
        height: u32,
        png_base64: String,
    },
    Error {
        correlation_id: CorrelationId,
        message: String,
    },
}

/// The correlation id of a request, so a relay can match its reply without
/// restating the variant list at every routing site.
pub fn request_correlation(request: &AgentRequest) -> CorrelationId {
    match request {
        AgentRequest::GetEditorState { correlation_id }
        | AgentRequest::GetBuffer { correlation_id, .. }
        | AgentRequest::SetBuffer { correlation_id, .. }
        | AgentRequest::ProposeEdit { correlation_id, .. }
        | AgentRequest::ListPlugins { correlation_id }
        | AgentRequest::GetApiReference { correlation_id }
        | AgentRequest::GetConsole { correlation_id }
        | AgentRequest::EditPlugin { correlation_id, .. }
        | AgentRequest::RunCommand { correlation_id, .. }
        | AgentRequest::QueryScene { correlation_id, .. }
        | AgentRequest::Screenshot { correlation_id, .. } => *correlation_id,
    }
}

/// The correlation id of a response, the counterpart to [`request_correlation`].
pub fn response_correlation(response: &AgentResponse) -> CorrelationId {
    match response {
        AgentResponse::EditorState { correlation_id, .. }
        | AgentResponse::Buffer { correlation_id, .. }
        | AgentResponse::Plugins { correlation_id, .. }
        | AgentResponse::Reference { correlation_id, .. }
        | AgentResponse::Console { correlation_id, .. }
        | AgentResponse::Diagnostics { correlation_id, .. }
        | AgentResponse::Ok { correlation_id }
        | AgentResponse::Scene { correlation_id, .. }
        | AgentResponse::Screenshot { correlation_id, .. }
        | AgentResponse::Error { correlation_id, .. } => *correlation_id,
    }
}

/// Whether a request is answered by the engine worker (the scene domain) rather
/// than the page (the editor domain). The page routes on this, and the worker
/// rejects anything that is not scene-domain.
pub fn is_scene_request(request: &AgentRequest) -> bool {
    matches!(
        request,
        AgentRequest::RunCommand { .. }
            | AgentRequest::QueryScene { .. }
            | AgentRequest::Screenshot { .. }
    )
}
