//! Application root. Owns the shared state, the engine-bridge slot, and the
//! language-worker handle, wires the persistence and theme effects, forwards
//! global keyboard input to the engine, and composes the shell. Data-oriented:
//! the root holds slots and runs effects, it is not an object that owns the app.

use leptos::prelude::*;
use protocol::ClientMessage;
use wasm_bindgen::{JsCast, JsValue};

use crate::bridge::{self, Bridge};
use crate::commands;
use crate::components::activity_bar::ActivityBar;
use crate::components::chat::ChatPane;
use crate::components::console::Console;
use crate::components::context_menu::ContextMenuView;
use crate::components::control_panel::ControlPanel;
use crate::components::editor_pane::EditorPane;
use crate::components::extensions::Extensions;
use crate::components::file_tree::FileTree;
use crate::components::find::FindBar;
use crate::components::help::Help;
use crate::components::jump_overlay::JumpOverlay;
use crate::components::loader::Loader;
use crate::components::lsp_menus::{CodeActionMenu, RenamePrompt, SymbolPicker};
use crate::components::lsp_panel::{LspConsent, LspLog};
use crate::components::palette::Palette;
use crate::components::plugin_panel::PluginPanel;
use crate::components::popups::{CompletionPopup, HoverCardView};
use crate::components::reference::Reference;
use crate::components::search::SearchPanel;
use crate::components::status_bar::StatusBar;
use crate::components::toolbar::Toolbar;
use crate::components::tour::TourView;
use crate::components::viewport::Viewport;
use crate::components::which_key::WhichKey;
use crate::lang;
use crate::state::{EditorState, SidebarView};
use crate::theme;

#[component]
pub fn App() -> impl IntoView {
    if !webgpu_supported() {
        return unsupported().into_any();
    }

    let state = EditorState::new();
    let bridge = StoredValue::new_local(None::<Bridge>);
    let lang = StoredValue::new_local(Some(lang::connect(state)));

    crate::session::capture();
    crate::ipc::notify_host("enable-fs");
    crate::fs::start(state);

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
        state.workspace_root.get();
        state.files.get();
        crate::session::save(state);
    });

    Effect::new(move |_| {
        state.active_id();
        state.active_kind();
        crate::lsp::refresh_diagnostics(state);
    });

    Effect::new(move |_| {
        if let Some(id) = state.command_request.get() {
            state.command_request.set(None);
            if let Some(command) = commands::command_from_id(&id) {
                commands::run(command, state, bridge);
            }
            crate::tour::observe(state, &id);
        }
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
        if state.jump.get_untracked().is_some() {
            event.prevent_default();
            crate::jump::key(state, &event.key());
            return;
        }
        if event.ctrl_key() && event.shift_key() && event.key().eq_ignore_ascii_case("p") {
            event.prevent_default();
            state.palette_open.set(true);
            return;
        }
        if event.key() == "F1" {
            event.prevent_default();
            state.help_open.set(true);
            return;
        }
        if event.ctrl_key() && event.key().eq_ignore_ascii_case("s") {
            event.prevent_default();
            if let Some(command) = commands::command_from_id("save-file") {
                commands::run(command, state, bridge);
            }
            return;
        }
        if event.ctrl_key()
            && (event.key().eq_ignore_ascii_case("f") || event.key().eq_ignore_ascii_case("h"))
        {
            event.prevent_default();
            state.find_open.set(true);
            return;
        }
        if event.ctrl_key()
            && !event.shift_key()
            && event.key().eq_ignore_ascii_case("z")
            && !target_is_input(&event)
        {
            event.prevent_default();
            crate::undo::undo(state);
            return;
        }
        if event.ctrl_key()
            && (event.key().eq_ignore_ascii_case("y")
                || (event.shift_key() && event.key().eq_ignore_ascii_case("z")))
            && !target_is_input(&event)
        {
            event.prevent_default();
            crate::undo::redo(state);
            return;
        }
        if event.key() == "Escape" && state.help_open.get_untracked() {
            state.help_open.set(false);
            return;
        }
        if event.key() == "Escape" && state.leader.get_untracked().is_some() {
            state.leader.set(None);
        }
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

    let area_ref = NodeRef::<leptos::html::Div>::new();
    let dragging = StoredValue::new(None::<usize>);

    let _ = window_event_listener(leptos::ev::pointermove, move |event| {
        let Some(right_key) = dragging.get_value() else {
            return;
        };
        let Some(area) = area_ref.get() else {
            return;
        };
        let vertical = state.split_vertical.get_untracked();
        let size = if vertical {
            area.client_width()
        } else {
            area.client_height()
        } as f32;
        if size <= 0.0 {
            return;
        }
        let movement = if vertical {
            event.movement_x()
        } else {
            event.movement_y()
        } as f32;
        state.drag_divider(right_key, movement / size);
    });

    let _ = window_event_listener(leptos::ev::pointerup, move |_| {
        dragging.set_value(None);
    });

    let split_below = move || state.pane_count() > 1 && !state.split_vertical.get();

    view! {
        <div
            class="app-shell"
            on:contextmenu=move |event: web_sys::MouseEvent| {
                event.prevent_default();
                crate::components::context_menu::open(
                    state,
                    event.client_x() as f64,
                    event.client_y() as f64,
                    crate::components::context_menu::general_menu(),
                );
            }
        >
            <Toolbar bridge state />
            <div class="workspace">
                <ActivityBar state />
                {move || match state.sidebar_view.get() {
                    SidebarView::Installed => view! { <PluginPanel bridge state /> }.into_any(),
                    SidebarView::Extensions => view! { <Extensions bridge state /> }.into_any(),
                    SidebarView::Files => view! { <FileTree state /> }.into_any(),
                    SidebarView::Search => view! { <SearchPanel state /> }.into_any(),
                }}
                <div
                    class="editor-area"
                    class:split-below=split_below
                    node_ref=area_ref
                >
                    <For
                        each=move || state.panes.get()
                        key=|pane| pane.key
                        children=move |pane| {
                            let key = pane.key;
                            view! {
                                <Show
                                    when=move || {
                                        state
                                            .panes
                                            .with(|panes| panes.first().map(|first| first.key) != Some(key))
                                    }
                                    fallback=|| ()
                                >
                                    <div
                                        class="editor-splitter"
                                        on:pointerdown=move |event: web_sys::PointerEvent| {
                                            event.prevent_default();
                                            dragging.set_value(Some(key));
                                        }
                                    ></div>
                                </Show>
                                <EditorPane bridge lang state pane_key=key />
                            }
                        }
                    />
                </div>
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
            <StatusBar state />
            <FindBar state />
            <CompletionPopup state />
            <HoverCardView state />
            <JumpOverlay state />
            <Reference state />
            <WhichKey state />
            <LspConsent state />
            <LspLog state />
            <RenamePrompt state />
            <CodeActionMenu state />
            <SymbolPicker state />
            <ControlPanel bridge state />
            <ContextMenuView bridge state />
            <TourView state />
            <Palette bridge state />
            <Help state />
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

/// Whether the event targets a panel field (an `input` or `select`), where the
/// browser's native undo should win over the editor's. The editor surface is a
/// `textarea`, so this leaves it alone.
fn target_is_input(event: &web_sys::KeyboardEvent) -> bool {
    event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
        .map(|element| {
            let tag = element.tag_name();
            tag.eq_ignore_ascii_case("input") || tag.eq_ignore_ascii_case("select")
        })
        .unwrap_or(false)
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
