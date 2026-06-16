//! The left panel: the plugin list (select, enable, delete, create) and the
//! read-only standard-library source, so every plugin's source and the helpers
//! it builds on are visible in one place.

use leptos::prelude::*;

use crate::bridge::{self, Bridge};
use crate::editor_plugins;
use crate::state::{EditorState, PluginKind};

#[component]
pub fn PluginPanel(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    state: EditorState,
) -> impl IntoView {
    let apply = move || {
        crate::plugins::save(&state.plugins.get_untracked());
        if let Some(bridge) = bridge.get_value() {
            bridge::sync_plugins(&bridge, state);
        }
    };

    let new_plugin = move |_| {
        let plugin = crate::plugins::new_plugin("Untitled");
        let id = plugin.id.clone();
        state.plugins.update(|plugins| plugins.push(plugin));
        state.active.set(Some(id));
        apply();
    };

    view! {
        <div class="plugin-panel">
            <div class="panel-title">
                <span>"Installed"</span>
                <button class="icon-button" title="New plugin" on:click=new_plugin>"+"</button>
            </div>
            <div class="plugin-list">
                <For
                    each=move || state.plugins.get()
                    key=|plugin| (plugin.id.clone(), plugin.enabled, plugin.name.clone())
                    let:plugin
                >
                    {
                        let select_id = plugin.id.clone();
                        let toggle_id = plugin.id.clone();
                        let delete_id = plugin.id.clone();
                        let active_id = plugin.id.clone();
                        view! {
                            <div
                                class="plugin-row"
                                class:active=move || {
                                    state.active.get().as_deref() == Some(active_id.as_str())
                                }
                            >
                                <input
                                    type="checkbox"
                                    prop:checked=plugin.enabled
                                    on:change=move |event| {
                                        let enabled = event_target_checked(&event);
                                        state.plugins.update(|plugins| {
                                            if let Some(plugin) = plugins
                                                .iter_mut()
                                                .find(|plugin| plugin.id == toggle_id)
                                            {
                                                plugin.enabled = enabled;
                                            }
                                        });
                                        apply();
                                    }
                                />
                                <span
                                    class="plugin-name"
                                    on:click=move |_| {
                                        state.active_kind.set(PluginKind::Scene);
                                        state.active.set(Some(select_id.clone()));
                                    }
                                >
                                    {plugin.name.clone()}
                                </span>
                                <button
                                    class="icon-button"
                                    title="Delete"
                                    on:click=move |_| {
                                        state.plugins.update(|plugins| {
                                            plugins.retain(|plugin| plugin.id != delete_id)
                                        });
                                        if state.active.get_untracked().as_deref()
                                            == Some(delete_id.as_str())
                                        {
                                            state.active.set(None);
                                        }
                                        apply();
                                    }
                                >
                                    "x"
                                </button>
                            </div>
                        }
                    }
                </For>
            </div>
            <div class="panel-title">"Editor plugins"</div>
            <div class="plugin-list">
                <For
                    each=move || state.editor_plugins.get()
                    key=|plugin| (plugin.id.clone(), plugin.enabled, plugin.name.clone())
                    let:plugin
                >
                    {
                        let select_id = plugin.id.clone();
                        let toggle_id = plugin.id.clone();
                        let active_id = plugin.id.clone();
                        view! {
                            <div
                                class="plugin-row"
                                class:active=move || {
                                    state.active.get().as_deref() == Some(active_id.as_str())
                                }
                            >
                                <input
                                    type="checkbox"
                                    prop:checked=plugin.enabled
                                    on:change=move |event| {
                                        let enabled = event_target_checked(&event);
                                        state.editor_plugins.update(|plugins| {
                                            if let Some(plugin) = plugins
                                                .iter_mut()
                                                .find(|plugin| plugin.id == toggle_id)
                                            {
                                                plugin.enabled = enabled;
                                            }
                                        });
                                        editor_plugins::reset_mode(state);
                                    }
                                />
                                <span
                                    class="plugin-name"
                                    on:click=move |_| {
                                        state.active_kind.set(PluginKind::Editor);
                                        state.active.set(Some(select_id.clone()));
                                    }
                                >
                                    {plugin.name.clone()}
                                </span>
                            </div>
                        }
                    }
                </For>
            </div>
            <div class="panel-title">"Standard library"</div>
            <div class="stdlib-list">
                <For
                    each=move || state.stdlib.get()
                    key=|module| module.name.clone()
                    let:module
                >
                    {
                        let open_name = module.name.clone();
                        let active_name = module.name.clone();
                        view! {
                            <div
                                class="plugin-row"
                                class:active=move || {
                                    state.active_kind.get() == PluginKind::Builtin
                                        && state.active.get().as_deref() == Some(active_name.as_str())
                                }
                            >
                                <span
                                    class="plugin-name builtin-name"
                                    on:click=move |_| {
                                        state.active_kind.set(PluginKind::Builtin);
                                        state.active.set(Some(open_name.clone()));
                                    }
                                >
                                    {module.name.clone()}
                                </span>
                            </div>
                        }
                    }
                </For>
            </div>
        </div>
    }
}
