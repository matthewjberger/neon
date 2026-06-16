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
