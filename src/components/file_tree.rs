//! The file tree sidebar: the opened workspace folder, lazily expanded. Clicking
//! a directory loads and toggles its children, clicking a file opens it in the
//! focused pane through the filesystem bridge.

use leptos::prelude::*;

use crate::fs;
use crate::state::{EditorState, TreeNode, basename};

#[component]
pub fn FileTree(state: EditorState) -> impl IntoView {
    view! {
        <div class="file-tree-panel">
            <div class="panel-title">
                <span>
                    {move || {
                        state
                            .workspace_root
                            .get()
                            .map(|root| basename(&root).to_string())
                            .unwrap_or_else(|| "No folder".to_string())
                    }}
                </span>
                <button
                    class="icon-button"
                    title="Open folder"
                    on:click=move |_| fs::open_folder()
                >
                    "open"
                </button>
            </div>
            <div class="file-tree">{move || render_nodes(state, state.tree.get(), 0)}</div>
        </div>
    }
}

fn render_nodes(state: EditorState, nodes: Vec<TreeNode>, depth: usize) -> AnyView {
    nodes
        .into_iter()
        .map(|node| render_node(state, node, depth))
        .collect_view()
        .into_any()
}

fn render_node(state: EditorState, node: TreeNode, depth: usize) -> AnyView {
    let path = node.path.clone();
    let is_dir = node.is_dir;
    let expanded = node.expanded;
    let indent = format!("padding-left: {}px", 8 + depth * 12);
    let icon = if is_dir {
        if expanded { "v" } else { ">" }
    } else {
        " "
    };
    let row = view! {
        <div
            class="tree-row"
            class:dir=is_dir
            style=indent
            on:click=move |_| {
                if is_dir {
                    fs::toggle_dir(state, &path);
                } else {
                    fs::read_file(&path);
                }
            }
            on:contextmenu=move |event: web_sys::MouseEvent| {
                event.prevent_default();
                event.stop_propagation();
                crate::components::context_menu::open(
                    state,
                    event.client_x() as f64,
                    event.client_y() as f64,
                    crate::components::context_menu::file_menu(),
                );
            }
        >
            <span class="tree-icon">{icon}</span>
            <span class="tree-name">{node.name.clone()}</span>
        </div>
    };
    if is_dir && expanded {
        view! {
            {row}
            {render_nodes(state, node.children, depth + 1)}
        }
        .into_any()
    } else {
        row.into_any()
    }
}
