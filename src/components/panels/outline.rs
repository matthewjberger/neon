//! The document outline: a side panel showing the focused file's symbols as a
//! hierarchical tree from the language server. Each row carries a kind glyph and
//! its depth indent; clicking one jumps to the symbol's line in the file.

use leptos::prelude::*;

use crate::state::{EditorState, OutlineNode};

#[component]
pub fn Outline(state: EditorState) -> impl IntoView {
    view! {
        <Show when=move || state.panels.outline.get() fallback=|| ()>
            <div class="outline">
                <div class="outline-header">
                    <span>"Outline"</span>
                    <button
                        class="outline-refresh"
                        title="Refresh"
                        on:click=move |_| crate::lsp::request_outline(state)
                    >
                        "\u{21bb}"
                    </button>
                    <button class="outline-close" on:click=move |_| state.panels.outline.set(false)>
                        "\u{00d7}"
                    </button>
                </div>
                <div class="outline-list">
                    {move || {
                        let nodes = state.lsp.outline.get();
                        if nodes.is_empty() {
                            view! { <div class="outline-empty">"No symbols"</div> }.into_any()
                        } else {
                            outline_rows(state, &nodes, 0).into_any()
                        }
                    }}
                </div>
            </div>
        </Show>
    }
}

fn outline_rows(state: EditorState, nodes: &[OutlineNode], depth: usize) -> AnyView {
    nodes
        .iter()
        .map(|node| {
            let line = node.line + 1;
            let indent = format!("padding-left: {}px;", 8 + depth * 14);
            let glyph = symbol_glyph(node.kind);
            let name = node.name.clone();
            let children = node.children.clone();
            view! {
                <div>
                    <div
                        class="outline-row"
                        style=indent
                        on:click=move |_| {
                            let path = state.lsp.outline_path.get_untracked();
                            if !path.is_empty() {
                                state.explorer.goto.set(Some((path, line)));
                            }
                        }
                    >
                        <span class="outline-kind">{glyph}</span>
                        <span class="outline-name">{name}</span>
                    </div>
                    {if children.is_empty() {
                        ().into_any()
                    } else {
                        outline_rows(state, &children, depth + 1).into_any()
                    }}
                </div>
            }
        })
        .collect_view()
        .into_any()
}

/// A one-character badge for an LSP `SymbolKind`, so the tree reads at a glance.
fn symbol_glyph(kind: u8) -> &'static str {
    match kind {
        2..=4 => "M",
        5 => "C",
        6 | 9 => "m",
        7 | 8 => "\u{2022}",
        10 => "E",
        11 => "I",
        12 => "\u{0192}",
        13 | 14 => "v",
        22 => "e",
        23 => "S",
        26 => "T",
        _ => "\u{00b7}",
    }
}
