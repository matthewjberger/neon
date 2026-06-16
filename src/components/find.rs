//! In-buffer find and replace. Operates on whichever textarea last had focus,
//! tracked in a thread-local the editor pane sets. Replacing dispatches a
//! synthetic input event so the editor's own commit path (buffer write, worker
//! sync, LSP didChange) runs, keeping one source of truth.

use std::cell::RefCell;

use leptos::html;
use leptos::prelude::*;
use web_sys::HtmlTextAreaElement;

use crate::state::EditorState;

thread_local! {
    static ACTIVE: RefCell<Option<HtmlTextAreaElement>> = const { RefCell::new(None) };
}

/// Records the textarea that last took focus, so the find bar acts on it.
pub fn set_active(element: HtmlTextAreaElement) {
    ACTIVE.with(|slot| *slot.borrow_mut() = Some(element));
}

/// The textarea that last took focus, for the find bar and the LSP client.
pub fn active() -> Option<HtmlTextAreaElement> {
    ACTIVE.with(|slot| slot.borrow().clone())
}

#[component]
pub fn FindBar(state: EditorState) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let replacement = RwSignal::new(String::new());
    let current = RwSignal::new(0_usize);
    let count = RwSignal::new(0_usize);
    let input_ref = NodeRef::<html::Input>::new();

    Effect::new(move |_| {
        if state.find_open.get()
            && let Some(input) = input_ref.get()
        {
            let _ = input.focus();
        }
    });

    let recount = move || {
        let total = active()
            .map(|element| matches(&element.value(), &query.get_untracked()).len())
            .unwrap_or(0);
        count.set(total);
        if current.get_untracked() >= total {
            current.set(0);
        }
    };

    let select = move |index: usize| {
        let Some(element) = active() else {
            return;
        };
        let found = matches(&element.value(), &query.get_untracked());
        if found.is_empty() {
            count.set(0);
            return;
        }
        count.set(found.len());
        let index = index % found.len();
        current.set(index);
        let (start, end) = found[index];
        let _ = element.focus();
        let _ = element.set_selection_range(start, end);
    };

    let step = move |delta: i64| {
        let total = count.get_untracked();
        if total == 0 {
            return;
        }
        let next = (current.get_untracked() as i64 + delta).rem_euclid(total as i64) as usize;
        select(next);
    };

    let replace_one = move || {
        let Some(element) = active() else {
            return;
        };
        let value = element.value();
        let found = matches(&value, &query.get_untracked());
        if found.is_empty() {
            return;
        }
        let index = current.get_untracked().min(found.len() - 1);
        let (start, end) = found[index];
        let replaced = splice_utf16(&value, start, end, &replacement.get_untracked());
        commit_value(&element, &replaced);
        recount();
        step(0);
    };

    let replace_all = move || {
        let Some(element) = active() else {
            return;
        };
        let needle = query.get_untracked();
        if needle.is_empty() {
            return;
        }
        let replaced = replace_all_text(&element.value(), &needle, &replacement.get_untracked());
        commit_value(&element, &replaced);
        recount();
    };

    view! {
        <Show when=move || state.find_open.get() fallback=|| ()>
            <div class="find-bar">
                <input
                    class="find-input"
                    node_ref=input_ref
                    placeholder="Find"
                    prop:value=move || query.get()
                    on:input=move |event| {
                        query.set(event_target_value(&event));
                        select(0);
                    }
                    on:keydown=move |event| {
                        match event.key().as_str() {
                            "Escape" => {
                                event.prevent_default();
                                state.find_open.set(false);
                            }
                            "Enter" => {
                                event.prevent_default();
                                step(if event.shift_key() { -1 } else { 1 });
                            }
                            _ => {}
                        }
                    }
                />
                <span class="find-count">
                    {move || {
                        let total = count.get();
                        if total == 0 {
                            "0/0".to_string()
                        } else {
                            format!("{}/{}", current.get() + 1, total)
                        }
                    }}
                </span>
                <button class="tool-button" on:click=move |_| step(-1)>"<"</button>
                <button class="tool-button" on:click=move |_| step(1)>">"</button>
                <input
                    class="find-input"
                    placeholder="Replace"
                    prop:value=move || replacement.get()
                    on:input=move |event| replacement.set(event_target_value(&event))
                />
                <button class="tool-button" on:click=move |_| replace_one()>"Replace"</button>
                <button class="tool-button" on:click=move |_| replace_all()>"All"</button>
                <button class="tool-button" on:click=move |_| state.find_open.set(false)>"x"</button>
            </div>
        </Show>
    }
}

/// Match ranges as UTF-16 offsets, the units a textarea selection uses.
fn matches(value: &str, needle: &str) -> Vec<(u32, u32)> {
    if needle.is_empty() {
        return Vec::new();
    }
    let hay = value.to_lowercase();
    let needle = needle.to_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(offset) = hay[from..].find(&needle) {
        let byte_start = from + offset;
        let byte_end = byte_start + needle.len();
        let start = hay[..byte_start].encode_utf16().count() as u32;
        let end = hay[..byte_end].encode_utf16().count() as u32;
        out.push((start, end));
        from = byte_end;
    }
    out
}

fn splice_utf16(value: &str, start: u32, end: u32, replacement: &str) -> String {
    let units: Vec<u16> = value.encode_utf16().collect();
    let head = String::from_utf16_lossy(&units[..start as usize]);
    let tail = String::from_utf16_lossy(&units[end as usize..]);
    format!("{head}{replacement}{tail}")
}

fn replace_all_text(value: &str, needle: &str, replacement: &str) -> String {
    let hay = value.to_lowercase();
    let lower = needle.to_lowercase();
    let mut out = String::with_capacity(value.len());
    let mut from = 0;
    while let Some(offset) = hay[from..].find(&lower) {
        let byte_start = from + offset;
        out.push_str(&value[from..byte_start]);
        out.push_str(replacement);
        from = byte_start + lower.len();
    }
    out.push_str(&value[from..]);
    out
}

fn commit_value(element: &HtmlTextAreaElement, value: &str) {
    element.set_value(value);
    if let Ok(event) = web_sys::Event::new("input") {
        let _ = element.dispatch_event(&event);
    }
}
