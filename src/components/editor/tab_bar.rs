//! The pane tab strip and cross-pane tab dragging, split out of the editor.

use std::cell::Cell;

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::state::EditorState;

thread_local! {
    /// The tab a drag started on, as `(pane_key, index)`, shared across every
    /// pane's tab bar so a tab can be dropped into a different pane.
    static TAB_DRAG: Cell<Option<(usize, usize)>> = const { Cell::new(None) };
}

fn set_drop_target(event: &web_sys::DragEvent, on: bool) {
    if let Some(element) = event
        .current_target()
        .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let list = element.class_list();
        if on {
            let _ = list.add_1("drop-target");
        } else {
            let _ = list.remove_1("drop-target");
        }
    }
}

/// The tab strip for one pane: a draggable, closable tab per open tile. Dragging
/// a tab reorders it within the pane or moves it into another pane; dropping on
/// the empty strip appends it.
#[component]
pub(super) fn TabBar(state: EditorState, pane_key: usize) -> impl IntoView {
    let pane = move || {
        state
            .panes
            .with(|panes| panes.iter().find(|pane| pane.key == pane_key).cloned())
    };
    let tab_count = move || pane().map(|pane| pane.tabs.len()).unwrap_or(0);
    view! {
        <div
            class="tab-bar"
            on:dragover=move |event: web_sys::DragEvent| {
                if TAB_DRAG.with(|drag| drag.get().is_some()) {
                    event.prevent_default();
                }
            }
            on:drop=move |event: web_sys::DragEvent| {
                event.prevent_default();
                if let Some((from_pane, from_index)) = TAB_DRAG.with(|drag| drag.take()) {
                    state.move_tab_across(from_pane, from_index, pane_key, tab_count());
                }
            }
        >
            {move || {
                let current_pane = pane();
                let active = current_pane.as_ref().map(|pane| pane.active).unwrap_or(0);
                current_pane
                    .map(|pane| pane.tabs)
                    .unwrap_or_default()
                    .into_iter()
                    .enumerate()
                    .map(|(index, tab)| {
                        let name = tab.title(&state);
                        let dirty = tab
                            .as_buffer()
                            .map(|buffer| state.is_dirty(buffer.kind, &buffer.id))
                            .unwrap_or(false);
                        view! {
                            <div
                                class="tab"
                                class:active=index == active
                                draggable="true"
                                on:click=move |_| state.focus_tab(pane_key, index)
                                on:dragstart=move |_| {
                                    TAB_DRAG.with(|drag| drag.set(Some((pane_key, index))));
                                }
                                on:dragover=move |event: web_sys::DragEvent| {
                                    if TAB_DRAG.with(|drag| drag.get().is_some()) {
                                        event.prevent_default();
                                        event.stop_propagation();
                                        set_drop_target(&event, true);
                                    }
                                }
                                on:dragleave=move |event: web_sys::DragEvent| {
                                    set_drop_target(&event, false);
                                }
                                on:drop=move |event: web_sys::DragEvent| {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    set_drop_target(&event, false);
                                    if let Some((from_pane, from_index)) = TAB_DRAG
                                        .with(|drag| drag.take())
                                    {
                                        state.move_tab_across(from_pane, from_index, pane_key, index);
                                    }
                                }
                                on:dragend=move |event: web_sys::DragEvent| {
                                    set_drop_target(&event, false);
                                    TAB_DRAG.with(|drag| drag.set(None));
                                }
                                on:mousedown=move |event: web_sys::MouseEvent| {
                                    if event.button() == 1 {
                                        event.prevent_default();
                                        state.close_tab(pane_key, index);
                                    }
                                }
                                on:contextmenu=move |event: web_sys::MouseEvent| {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    state.focus_tab(pane_key, index);
                                    crate::components::context_menu::open(
                                        state,
                                        event.client_x() as f64,
                                        event.client_y() as f64,
                                        crate::components::context_menu::tab_menu(),
                                    );
                                }
                            >
                                <span class="tab-name">{name}</span>
                                <Show when=move || dirty fallback=|| ()>
                                    <span class="tab-dirty">"\u{2022}"</span>
                                </Show>
                                <button
                                    class="tab-close"
                                    on:click=move |event: web_sys::MouseEvent| {
                                        event.stop_propagation();
                                        state.close_tab(pane_key, index);
                                    }
                                >
                                    "\u{00d7}"
                                </button>
                            </div>
                        }
                    })
                    .collect_view()
            }}
        </div>
    }
}
