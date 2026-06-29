//! The MCP tool catalog: the name, description, and JSON input schema for
//! every tool the agent bridge exposes. Pure data, split from `agent.rs` so
//! the bridge logic and the tool surface read separately.

use serde_json::{Value, json};

pub(super) fn tool_definitions() -> Vec<Value> {
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
            "get_api_reference",
            "The scripting API: every command (method name, fields, reply type) and every standard-library helper (name, signature, receiver). Call this first to learn what you can write in a plugin. You cannot read the source files; this is the API.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "get_console",
            "The recent console traffic: the commands a plugin ran, the events it received, and any runtime errors. Call this after editing to see whether a plugin errored at run time.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "set_buffer",
            "Replace a plugin's rhai source. The scene re-runs the plugins with the new source. Returns diagnostics: ok is true when it compiled clean, otherwise the syntax errors and unknown-command warnings.",
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
            "propose_edit",
            "Propose a new full rhai source for a plugin without applying it. The user reviews it as an accept/reject diff in the editor. Returns diagnostics for the proposed source: ok is true when it compiled clean, otherwise the syntax errors and unknown-command warnings.",
            json!({
                "type": "object",
                "properties": {
                    "buffer": { "type": "string", "description": "plugin id" },
                    "text": { "type": "string", "description": "proposed rhai source" }
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
            "Create or update a plugin. Pass an existing id to update it, or a new id to create one. The scene re-runs the plugins. Returns diagnostics for the new source: ok is true when it compiled clean, otherwise the syntax errors and unknown-command warnings.",
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
