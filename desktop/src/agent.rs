//! The Claude bridge: an MCP endpoint over local HTTP in front of a websocket
//! relay to the page. It turns each `tools/call` into an `AgentRequest`, sends
//! it to the page, and matches the `AgentResponse` by correlation id. It holds
//! no editor state; the buffers live on the page and the scene in the worker.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use futures_util::{SinkExt, StreamExt};
use protocol::{AgentRequest, AgentResponse, CorrelationId, PluginSource};
use serde_json::{Value, json};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;

const WS_ADDR: &str = "127.0.0.1:8789";
const MCP_ADDR: &str = "127.0.0.1:8790";
const REQUEST_TIMEOUT_SECS: u64 = 30;

struct Shared {
    next_correlation: AtomicU64,
    pending: Mutex<HashMap<CorrelationId, oneshot::Sender<AgentResponse>>>,
    page_tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
}

impl Shared {
    fn new() -> Self {
        Self {
            next_correlation: AtomicU64::new(1),
            pending: Mutex::new(HashMap::new()),
            page_tx: Mutex::new(None),
        }
    }

    fn correlation(&self) -> CorrelationId {
        self.next_correlation.fetch_add(1, Ordering::Relaxed)
    }
}

static STARTED: AtomicBool = AtomicBool::new(false);

/// Starts the bridge on a background thread: a tokio runtime hosting the page
/// relay websocket, and a blocking HTTP loop serving MCP. Idempotent.
pub fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                log(&format!("failed to start the agent runtime: {error}"));
                return;
            }
        };
        let shared = Arc::new(Shared::new());
        let ws_shared = shared.clone();
        runtime.spawn(async move {
            run_ws_server(ws_shared).await;
        });
        run_mcp_server(shared, runtime.handle().clone());
    });
}

async fn run_ws_server(shared: Arc<Shared>) {
    let listener = match tokio::net::TcpListener::bind(WS_ADDR).await {
        Ok(listener) => listener,
        Err(error) => {
            log(&format!("failed to bind {WS_ADDR}: {error}"));
            return;
        }
    };
    log(&format!("websocket relay listening on ws://{WS_ADDR}"));
    loop {
        let Ok((stream, _addr)) = listener.accept().await else {
            continue;
        };
        let conn_shared = shared.clone();
        tokio::spawn(async move {
            handle_page(conn_shared, stream).await;
        });
    }
}

async fn handle_page(shared: Arc<Shared>, stream: tokio::net::TcpStream) {
    let websocket = match tokio_tungstenite::accept_async(stream).await {
        Ok(websocket) => websocket,
        Err(error) => {
            log(&format!("websocket handshake failed: {error}"));
            return;
        }
    };
    log("editor page connected");
    let (mut sink, mut source) = websocket.split();

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();
    *shared.page_tx.lock().await = Some(out_tx);

    let writer = tokio::spawn(async move {
        while let Some(text) = out_rx.recv().await {
            if sink.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    });

    while let Some(message) = source.next().await {
        let Ok(message) = message else {
            break;
        };
        let text = match message {
            Message::Text(text) => text,
            Message::Close(_) => break,
            _ => continue,
        };
        let Ok(response) = serde_json::from_str::<AgentResponse>(&text) else {
            log(&format!("unparseable response from page: {text}"));
            continue;
        };
        route_response(&shared, response).await;
    }

    *shared.page_tx.lock().await = None;
    writer.abort();
    log("editor page disconnected");
}

async fn route_response(shared: &Arc<Shared>, response: AgentResponse) {
    let correlation_id = response_correlation(&response);
    let sender = shared.pending.lock().await.remove(&correlation_id);
    if let Some(sender) = sender {
        let _ = sender.send(response);
    }
}

fn response_correlation(response: &AgentResponse) -> CorrelationId {
    match response {
        AgentResponse::EditorState { correlation_id, .. }
        | AgentResponse::Buffer { correlation_id, .. }
        | AgentResponse::Plugins { correlation_id, .. }
        | AgentResponse::Ok { correlation_id }
        | AgentResponse::Scene { correlation_id, .. }
        | AgentResponse::Screenshot { correlation_id, .. }
        | AgentResponse::Error { correlation_id, .. } => *correlation_id,
    }
}

async fn send_request(
    shared: &Arc<Shared>,
    request: AgentRequest,
) -> Result<AgentResponse, String> {
    let correlation_id = request_correlation(&request);
    let (tx, rx) = oneshot::channel();
    shared.pending.lock().await.insert(correlation_id, tx);

    let text = serde_json::to_string(&request).map_err(|error| error.to_string())?;
    {
        let guard = shared.page_tx.lock().await;
        let Some(sender) = guard.as_ref() else {
            shared.pending.lock().await.remove(&correlation_id);
            return Err("editor page is not connected".to_string());
        };
        if sender.send(text).is_err() {
            shared.pending.lock().await.remove(&correlation_id);
            return Err("editor page relay is closed".to_string());
        }
    }

    let timeout = tokio::time::Duration::from_secs(REQUEST_TIMEOUT_SECS);
    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(_)) => Err("response channel dropped".to_string()),
        Err(_) => {
            shared.pending.lock().await.remove(&correlation_id);
            Err("timed out waiting for the editor".to_string())
        }
    }
}

fn request_correlation(request: &AgentRequest) -> CorrelationId {
    match request {
        AgentRequest::GetEditorState { correlation_id }
        | AgentRequest::GetBuffer { correlation_id, .. }
        | AgentRequest::SetBuffer { correlation_id, .. }
        | AgentRequest::ListPlugins { correlation_id }
        | AgentRequest::EditPlugin { correlation_id, .. }
        | AgentRequest::RunCommand { correlation_id, .. }
        | AgentRequest::QueryScene { correlation_id, .. }
        | AgentRequest::Screenshot { correlation_id, .. } => *correlation_id,
    }
}

fn run_mcp_server(shared: Arc<Shared>, handle: tokio::runtime::Handle) {
    let server = match tiny_http::Server::http(MCP_ADDR) {
        Ok(server) => server,
        Err(error) => {
            log(&format!("failed to bind {MCP_ADDR}: {error}"));
            return;
        }
    };
    log(&format!("mcp endpoint listening on http://{MCP_ADDR}/mcp"));
    for request in server.incoming_requests() {
        let request_shared = shared.clone();
        let request_handle = handle.clone();
        std::thread::spawn(move || {
            handle_mcp_request(request_shared, request_handle, request);
        });
    }
}

fn handle_mcp_request(
    shared: Arc<Shared>,
    handle: tokio::runtime::Handle,
    mut request: tiny_http::Request,
) {
    if *request.method() != tiny_http::Method::Post {
        let _ = request.respond(tiny_http::Response::empty(405));
        return;
    }
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        let _ = request.respond(tiny_http::Response::empty(400));
        return;
    }
    let Ok(message) = serde_json::from_str::<Value>(&body) else {
        let _ = request.respond(tiny_http::Response::empty(400));
        return;
    };
    let id = message.get("id").cloned();
    let method = message
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let params = message.get("params").cloned().unwrap_or(Value::Null);

    let response = handle.block_on(dispatch(&shared, &method, params, id));
    match response {
        Some(value) => {
            let header =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .expect("static header is valid");
            let _ = request
                .respond(tiny_http::Response::from_string(value.to_string()).with_header(header));
        }
        None => {
            let _ = request.respond(tiny_http::Response::empty(202));
        }
    }
}

async fn dispatch(
    shared: &Arc<Shared>,
    method: &str,
    params: Value,
    id: Option<Value>,
) -> Option<Value> {
    match method {
        "initialize" => {
            let version = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("2025-03-26")
                .to_string();
            Some(rpc_result(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "neon", "version": "0.1.0" }
                }),
            ))
        }
        "notifications/initialized" => None,
        "ping" => Some(rpc_result(id, json!({}))),
        "tools/list" => Some(rpc_result(id, json!({ "tools": tool_definitions() }))),
        "tools/call" => Some(handle_tool_call(shared, params, id).await),
        _ => Some(rpc_error(id, -32601, &format!("method not found: {method}"))),
    }
}

async fn handle_tool_call(shared: &Arc<Shared>, params: Value, id: Option<Value>) -> Value {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    if name == "screenshot" {
        return match screenshot_tool(shared, arguments).await {
            Ok(content) => rpc_result(id, json!({ "content": content, "isError": false })),
            Err(error) => rpc_result(
                id,
                json!({ "content": [{ "type": "text", "text": error }], "isError": true }),
            ),
        };
    }

    match run_tool(shared, &name, arguments).await {
        Ok(text) => rpc_result(
            id,
            json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
        ),
        Err(error) => rpc_result(
            id,
            json!({ "content": [{ "type": "text", "text": error }], "isError": true }),
        ),
    }
}

async fn run_tool(shared: &Arc<Shared>, name: &str, arguments: Value) -> Result<String, String> {
    match name {
        "get_editor_state" => {
            let correlation_id = shared.correlation();
            let response =
                send_request(shared, AgentRequest::GetEditorState { correlation_id }).await?;
            match response {
                AgentResponse::EditorState { state, .. } => Ok(state.to_string()),
                other => format_other(other),
            }
        }
        "get_buffer" => {
            let buffer = arguments
                .get("buffer")
                .and_then(Value::as_str)
                .map(str::to_string);
            let correlation_id = shared.correlation();
            let response = send_request(
                shared,
                AgentRequest::GetBuffer {
                    correlation_id,
                    buffer,
                },
            )
            .await?;
            match response {
                AgentResponse::Buffer { text, .. } => Ok(text),
                other => format_other(other),
            }
        }
        "set_buffer" => {
            let buffer = arguments
                .get("buffer")
                .and_then(Value::as_str)
                .ok_or("buffer is required")?
                .to_string();
            let text = arguments
                .get("text")
                .and_then(Value::as_str)
                .ok_or("text is required")?
                .to_string();
            let correlation_id = shared.correlation();
            let response = send_request(
                shared,
                AgentRequest::SetBuffer {
                    correlation_id,
                    buffer,
                    text,
                },
            )
            .await?;
            ok_result(response)
        }
        "list_plugins" => {
            let correlation_id = shared.correlation();
            let response =
                send_request(shared, AgentRequest::ListPlugins { correlation_id }).await?;
            match response {
                AgentResponse::Plugins { plugins, .. } => {
                    Ok(serde_json::to_string(&plugins).unwrap_or_default())
                }
                other => format_other(other),
            }
        }
        "edit_plugin" => {
            let plugin = PluginSource {
                id: arguments
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                name: arguments
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("Untitled")
                    .to_string(),
                source: arguments
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                enabled: arguments
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            };
            if plugin.id.is_empty() {
                return Err("id is required".to_string());
            }
            let correlation_id = shared.correlation();
            let response = send_request(
                shared,
                AgentRequest::EditPlugin {
                    correlation_id,
                    plugin,
                },
            )
            .await?;
            ok_result(response)
        }
        "run_command" => {
            let command = arguments
                .get("command")
                .ok_or("command is required")?
                .to_string();
            let correlation_id = shared.correlation();
            let response = send_request(
                shared,
                AgentRequest::RunCommand {
                    correlation_id,
                    command,
                },
            )
            .await?;
            match response {
                AgentResponse::Scene { result, .. } => Ok(result.to_string()),
                other => format_other(other),
            }
        }
        "query_scene" => {
            let components = arguments
                .get("components")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|value| value.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let correlation_id = shared.correlation();
            let response = send_request(
                shared,
                AgentRequest::QueryScene {
                    correlation_id,
                    components,
                },
            )
            .await?;
            match response {
                AgentResponse::Scene { result, .. } => Ok(result.to_string()),
                other => format_other(other),
            }
        }
        other => Err(format!("unknown tool: {other}")),
    }
}

async fn screenshot_tool(shared: &Arc<Shared>, arguments: Value) -> Result<Value, String> {
    let max_dimension = arguments
        .get("max_dimension")
        .and_then(Value::as_u64)
        .map(|value| value as u32);
    let correlation_id = shared.correlation();
    let response = send_request(
        shared,
        AgentRequest::Screenshot {
            correlation_id,
            max_dimension,
        },
    )
    .await?;
    match response {
        AgentResponse::Screenshot {
            width,
            height,
            png_base64,
            ..
        } => Ok(json!([
            { "type": "image", "data": png_base64, "mimeType": "image/png" },
            { "type": "text", "text": json!({ "width": width, "height": height }).to_string() },
        ])),
        AgentResponse::Error { message, .. } => Err(message),
        other => Ok(json!([{ "type": "text", "text": format_other(other).unwrap_or_default() }])),
    }
}

fn ok_result(response: AgentResponse) -> Result<String, String> {
    match response {
        AgentResponse::Ok { .. } => Ok(json!({ "ok": true }).to_string()),
        AgentResponse::Error { message, .. } => Err(message),
        other => format_other(other),
    }
}

fn format_other(response: AgentResponse) -> Result<String, String> {
    if let AgentResponse::Error { message, .. } = response {
        return Err(message);
    }
    Ok(serde_json::to_value(&response)
        .unwrap_or(Value::Null)
        .to_string())
}

fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "get_editor_state",
            "Read the editor state: open plugins, the active plugin, the current scene selection, the running flag, and entity count. Small and cheap.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "get_buffer",
            "Read a plugin's full rhai source by id, or the active plugin when buffer is omitted.",
            json!({
                "type": "object",
                "properties": { "buffer": { "type": "string", "description": "plugin id, or omit for the active plugin" } }
            }),
        ),
        tool(
            "set_buffer",
            "Replace a plugin's rhai source. The scene re-runs the plugins with the new source.",
            json!({
                "type": "object",
                "properties": {
                    "buffer": { "type": "string", "description": "plugin id" },
                    "text": { "type": "string", "description": "new rhai source" }
                },
                "required": ["buffer", "text"]
            }),
        ),
        tool(
            "list_plugins",
            "List every plugin with its id, name, source, and enabled flag.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "edit_plugin",
            "Create or update a plugin. Pass an existing id to update it, or a new id to create one. The scene re-runs the plugins.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "name": { "type": "string" },
                    "source": { "type": "string", "description": "rhai source with on_start and/or on_tick" },
                    "enabled": { "type": "boolean" }
                },
                "required": ["id", "source"]
            }),
        ),
        tool(
            "run_command",
            "Run one nightshade-api Command against the live scene, as a json object like {\"SpawnCube\":{\"position\":[0,0.5,0]}}. Returns the command reply.",
            json!({
                "type": "object",
                "properties": { "command": { "type": "object", "description": "an externally tagged Command" } },
                "required": ["command"]
            }),
        ),
        tool(
            "query_scene",
            "Return the entity ids in the scene. components is reserved for filtering by component name.",
            json!({
                "type": "object",
                "properties": { "components": { "type": "array", "items": { "type": "string" } } }
            }),
        ),
        tool(
            "screenshot",
            "Capture the rendered viewport as a PNG image so you can see the scene. max_dimension caps the longer side in pixels.",
            json!({
                "type": "object",
                "properties": { "max_dimension": { "type": "integer" } }
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({ "name": name, "description": description, "inputSchema": input_schema })
}

fn rpc_result(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": result })
}

fn rpc_error(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "error": { "code": code, "message": message } })
}

fn log(message: &str) {
    eprintln!("[agent] {message}");
}
