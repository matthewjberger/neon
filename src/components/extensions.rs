//! The Extensions view: the plugin manager. Lists every bundled plugin with its
//! description and an install or uninstall control, so the workspace explorer
//! only shows what you have installed.

use leptos::prelude::*;

use crate::bridge::{self, Bridge};
use crate::plugins::{catalog, entry_to_plugin};
use crate::state::{EditorState, PluginKind};

#[component]
pub fn Extensions(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    state: EditorState,
) -> impl IntoView {
    view! {
        <div class="extensions">
            <div class="panel-title">"Plugins"</div>
            <div class="extensions-list">
                {catalog()
                    .into_iter()
                    .map(|entry| {
                        let id = entry.id;
                        let kind = entry.kind;
                        let name = entry.name;
                        let description = entry.description;
                        view! {
                            <div class="extension">
                                <div class="extension-head">
                                    <span class="extension-name">{name}</span>
                                    <span class="extension-kind">{kind_label(kind)}</span>
                                </div>
                                <div class="extension-desc">{description}</div>
                                <button
                                    class="tool-button extension-action"
                                    class:installed=move || installed(state, id, kind)
                                    on:click=move |_| toggle_install(bridge, state, id, kind)
                                >
                                    {move || if installed(state, id, kind) { "Uninstall" } else { "Install" }}
                                </button>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

fn kind_label(kind: PluginKind) -> &'static str {
    match kind {
        PluginKind::Scene => "scene",
        PluginKind::Editor => "editor",
        PluginKind::Builtin => "built-in",
        PluginKind::File => "file",
    }
}

fn set_for(state: EditorState, kind: PluginKind) -> RwSignal<Vec<protocol::PluginSource>> {
    if kind == PluginKind::Editor {
        state.editor_plugins
    } else {
        state.plugins
    }
}

fn installed(state: EditorState, id: &str, kind: PluginKind) -> bool {
    set_for(state, kind).with(|plugins| plugins.iter().any(|plugin| plugin.id == id))
}

fn toggle_install(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    state: EditorState,
    id: &'static str,
    kind: PluginKind,
) {
    let signal = set_for(state, kind);
    let present = signal.with_untracked(|plugins| plugins.iter().any(|plugin| plugin.id == id));
    if present {
        signal.update(|plugins| plugins.retain(|plugin| plugin.id != id));
        if state.active_id().as_deref() == Some(id) {
            state.open_in_focused(kind, None);
        }
    } else if let Some(entry) = catalog().iter().find(|entry| entry.id == id) {
        signal.update(|plugins| plugins.push(entry_to_plugin(entry)));
        state.open_in_focused(kind, Some(id.to_string()));
    }
    if kind == PluginKind::Scene
        && let Some(bridge) = bridge.get_value()
    {
        bridge::sync_plugins(&bridge, state);
    }
}
