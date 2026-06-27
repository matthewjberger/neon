//! The pane tab strip and cross-pane tab dragging, split out of the editor.
//!
//! Dragging is pointer-driven, not the HTML5 drag API: the desktop webview
//! (WebView2) never delivers `drag*` events to page elements, so a tab is
//! dragged the same way the pane dividers are, with pointer events and a state
//! signal. `App` hosts the window-level `pointermove`/`pointerup` listeners and
//! the floating preview; this strip starts the drag and shows the drop line.

use leptos::prelude::*;

use crate::state::{EditorState, TabDrag};

/// The tab strip for one pane: a draggable, closable tab per open tile. Dragging
/// a tab reorders it within the pane or moves it into another pane; dropping past
/// the last tab appends it. The `data-pane` and `data-index` attributes let the
/// drag hit-test resolve the drop slot from the pointer position.
#[component]
pub(super) fn TabBar(state: EditorState, pane_key: usize) -> impl IntoView {
    let pane = move || {
        state
            .panes
            .with(|panes| panes.iter().find(|pane| pane.key == pane_key).cloned())
    };
    let tab_count = move || pane().map(|pane| pane.tabs.len()).unwrap_or(0);
    let tail_target = move || {
        state.editing.tab_drag.with(|drag| {
            drag.as_ref()
                .is_some_and(|drag| drag.started && drag.target == Some((pane_key, tab_count())))
        })
    };
    view! {
        <div class="tab-bar" data-pane=pane_key>
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
                        let preview = name.clone();
                        let is_drop = move || {
                            state.editing.tab_drag.with(|drag| {
                                drag.as_ref().is_some_and(|drag| {
                                    drag.started && drag.target == Some((pane_key, index))
                                })
                            })
                        };
                        view! {
                            <div
                                class="tab"
                                class:active=index == active
                                class:drop-target=is_drop
                                data-index=index
                                on:click=move |_| state.focus_tab(pane_key, index)
                                on:pointerdown=move |event: web_sys::PointerEvent| {
                                    if event.button() != 0 {
                                        return;
                                    }
                                    state
                                        .editing
                                        .tab_drag
                                        .set(
                                            Some(TabDrag {
                                                from_pane: pane_key,
                                                from_index: index,
                                                title: preview.clone(),
                                                origin_x: event.client_x() as f64,
                                                origin_y: event.client_y() as f64,
                                                x: event.client_x() as f64,
                                                y: event.client_y() as f64,
                                                started: false,
                                                target: None,
                                            }),
                                        );
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
                                    crate::components::overlays::context_menu::open(
                                        state,
                                        event.client_x() as f64,
                                        event.client_y() as f64,
                                        crate::components::overlays::context_menu::tab_menu(),
                                    );
                                }
                            >
                                <span class="tab-name">{name}</span>
                                <Show when=move || dirty fallback=|| ()>
                                    <span class="tab-dirty">"\u{2022}"</span>
                                </Show>
                                <button
                                    class="tab-close"
                                    on:pointerdown=move |event: web_sys::PointerEvent| {
                                        event.stop_propagation();
                                    }
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
            <div class="tab-tail" class:drop-target=tail_target></div>
        </div>
    }
}
