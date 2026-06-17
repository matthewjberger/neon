//! Neon's page: the Leptos UI on the main thread.
//!
//! Data-oriented throughout. State is a `Copy` struct of signals
//! ([`state::EditorState`]); behavior is free functions; components are plain
//! `#[component]` functions. No objects own the engine, the workers, or each
//! other.
//!
//! - `app.rs` composes the shell and forwards global input.
//! - `bridge.rs` spawns the engine worker and maps its `WorkerMessage`s to
//!   signal writes.
//! - `lang.rs` spawns the language worker and routes compile-check requests.
//! - `state.rs` is all page state as `Copy` signals.
//! - `highlight.rs` is the rhai syntax scanner the editor overlay paints with.
//! - `plugins.rs` is the plugin model and its local-storage persistence.
//! - `components/` holds the viewport, code editor, plugin panel, console,
//!   reference, toolbar, chat, and loader.

mod app;
mod bridge;
mod caret;
mod check;
mod commands;
mod complete;
mod components;
mod editor_plugins;
mod fs;
mod highlight;
mod ipc;
mod jump;
mod lang;
mod lsp;
mod plugins;
mod relay;
mod session;
mod state;
mod theme;
mod tour;
mod undo;

pub use app::App;
