//! Application root. Owns the shared state, the engine-bridge slot, and the
//! language-worker handle, wires the persistence and theme effects, forwards
//! global keyboard input to the engine, and composes the shell. Data-oriented:
//! the root holds slots and runs effects, it is not an object that owns the app.

use leptos::prelude::*;
use protocol::ClientMessage;
use wasm_bindgen::{JsCast, JsValue};

use crate::bridge::{self, Bridge};
use crate::components::chat::ChatPane;
use crate::components::console::Console;
use crate::components::editor_pane::EditorPane;
use crate::components::loader::Loader;
use crate::components::plugin_panel::PluginPanel;
use crate::components::reference::Reference;
use crate::components::toolbar::Toolbar;
use crate::components::viewport::Viewport;
use crate::lang;
use crate::state::EditorState;
use crate::theme;

#[component]
pub fn App() -> impl IntoView {
    if !webgpu_supported() {
        return unsupported().into_any();
    }

    let state = EditorState::new();
    let bridge = StoredValue::new_local(None::<Bridge>);
    let lang = StoredValue::new_local(Some(lang::connect(state)));

    theme::apply_theme(&state.theme.get_untracked());

    Effect::new(move |_| {
        theme::apply_theme(&state.theme.get());
    });

    Effect::new(move |_| {
        let plugins = state.plugins.get();
        crate::plugins::save(&plugins);
    });

    Effect::new(move |_| {
        let editor_plugins = state.editor_plugins.get();
        crate::plugins::save_editor_plugins(&editor_plugins);
    });

    Effect::new(move |_| {
        state.active.get();
        state.diagnostics.set(Vec::new());
    });

    let seeded = StoredValue::new(false);
    Effect::new(move |_| {
        let commands = state.commands.get();
        if commands.is_empty() || seeded.get_value() {
            return;
        }
        seeded.set_value(true);
        if let Some(lang) = lang.get_value() {
            lang::init(&lang, commands, state.stdlib.get_untracked());
        }
        if let Some(bridge) = bridge.get_value() {
            bridge::sync_plugins(&bridge, state);
        }
    });

    let _ = window_event_listener(leptos::ev::keydown, move |event| {
        if typing_in_field(&event) {
            return;
        }
        if let Some(bridge) = bridge.get_value() {
            let text = (event.key().chars().count() == 1).then(|| event.key());
            bridge::send(
                &bridge,
                &ClientMessage::Key {
                    code: event.code(),
                    pressed: true,
                    text,
                },
            );
        }
    });

    let _ = window_event_listener(leptos::ev::keyup, move |event| {
        if typing_in_field(&event) {
            return;
        }
        if let Some(bridge) = bridge.get_value() {
            bridge::send(
                &bridge,
                &ClientMessage::Key {
                    code: event.code(),
                    pressed: false,
                    text: None,
                },
            );
        }
    });

    view! {
        <div class="app-shell">
            <Toolbar bridge state />
            <div class="workspace">
                <PluginPanel bridge state />
                <EditorPane bridge lang state />
                <div
                    class="right-column"
                    style:display=move || if state.viewport_open.get() { "flex" } else { "none" }
                >
                    <Viewport bridge state />
                    <Show when=move || state.console_open.get() fallback=|| ()>
                        <Console bridge state />
                    </Show>
                </div>
            </div>
            <Reference state />
            <ChatPane state />
            <Loader state />
        </div>
    }
    .into_any()
}

fn unsupported() -> impl IntoView {
    view! {
        <div class="unsupported">
            <div class="unsupported-card">
                <h1>"WebGPU not available"</h1>
                <p>
                    "Neon runs the engine in a web worker through WebGPU. Open it in a browser with WebGPU and OffscreenCanvas-in-workers support (Chromium 113+, Firefox 141+)."
                </p>
            </div>
        </div>
    }
}

fn typing_in_field(event: &web_sys::KeyboardEvent) -> bool {
    event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
        .map(|element| {
            let tag = element.tag_name();
            tag.eq_ignore_ascii_case("input")
                || tag.eq_ignore_ascii_case("textarea")
                || tag.eq_ignore_ascii_case("select")
                || element.is_content_editable()
        })
        .unwrap_or(false)
}

fn webgpu_supported() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Ok(navigator) = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("navigator"))
    else {
        return false;
    };
    js_sys::Reflect::get(&navigator, &JsValue::from_str("gpu"))
        .map(|gpu| !gpu.is_undefined() && !gpu.is_null())
        .unwrap_or(false)
}
