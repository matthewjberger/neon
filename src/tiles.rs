//! The tile registry: one home for the content a pane tab can hold. A tile is
//! a buffer or one of the editor's panels (the 3D view, console, terminal,
//! reference), so all of them are content you open into a pane rather than fixed
//! chrome. Adding a kind of tile is this file (the variant, its title, and how
//! it renders) plus an open command.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::bridge::Bridge;
use crate::components::console::Console;
use crate::components::reference::Reference;
use crate::components::terminal::Terminal;
use crate::components::viewport::Viewport;
use crate::state::{BufferRef, EditorState};

/// The engine-bridge slot threaded into the panel tiles that drive the scene.
pub type BridgeSlot = StoredValue<Option<Bridge>, LocalStorage>;

/// What a pane tab holds: a text buffer, or one of the editor's panels.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum TileContent {
    Buffer(BufferRef),
    Viewport,
    Console,
    Terminal,
    Reference,
}

impl TileContent {
    /// The buffer this tile holds, or `None` for a panel tile.
    pub fn as_buffer(&self) -> Option<&BufferRef> {
        match self {
            TileContent::Buffer(buffer) => Some(buffer),
            _ => None,
        }
    }

    /// The tab's display name.
    pub fn title(&self, state: &EditorState) -> String {
        match self {
            TileContent::Buffer(buffer) => state.buffer_name(buffer.kind, &buffer.id),
            TileContent::Viewport => "3D View".to_string(),
            TileContent::Console => "Console".to_string(),
            TileContent::Terminal => "Terminal".to_string(),
            TileContent::Reference => "Reference".to_string(),
        }
    }
}

/// Renders a pane's body for a panel tile. The editor surface handles buffer
/// tiles itself, so this covers the panels and the empty placeholder.
pub fn body(content: Option<TileContent>, bridge: BridgeSlot, state: EditorState) -> AnyView {
    match content {
        Some(TileContent::Viewport) => view! { <Viewport bridge state /> }.into_any(),
        Some(TileContent::Console) => view! { <Console bridge state /> }.into_any(),
        Some(TileContent::Terminal) => view! { <Terminal state /> }.into_any(),
        Some(TileContent::Reference) => view! { <Reference state /> }.into_any(),
        _ => view! { <div class="editor-empty">"Open a buffer to edit"</div> }.into_any(),
    }
}
