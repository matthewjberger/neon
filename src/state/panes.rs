//! Pane and tab layout: splits, tab order, focus movement, and divider
//! drags. Split from `state.rs` to keep the state struct and its
//! persistence apart from the window-management methods, which form their
//! own cohesive surface.

use leptos::prelude::*;

use crate::state::{BufferRef, EditorState, Pane, PluginKind};

impl EditorState {
    /// The focused pane, falling back to the first if the key is stale.
    pub fn focused(&self) -> Pane {
        self.panes.with(|panes| {
            let key = self.focused_key.get();
            panes
                .iter()
                .find(|pane| pane.key == key)
                .or_else(|| panes.first())
                .cloned()
                .unwrap_or(Pane {
                    key: 0,
                    tabs: Vec::new(),
                    active: 0,
                    flex: 1.0,
                })
        })
    }

    /// The focused pane's active buffer.
    pub fn focused_buffer(&self) -> BufferRef {
        self.focused().buffer().cloned().unwrap_or(BufferRef {
            kind: PluginKind::Scene,
            id: None,
        })
    }

    /// The focused pane's open buffer id.
    pub fn active_id(&self) -> Option<String> {
        self.focused_buffer().id
    }

    /// The focused pane's buffer kind.
    pub fn active_kind(&self) -> PluginKind {
        self.focused_buffer().kind
    }

    /// The focused pane's source, used by the agent relay.
    pub fn active_source(&self) -> String {
        let buffer = self.focused_buffer();
        self.buffer_source(buffer.kind, &buffer.id)
    }

    /// The number of open panes.
    pub fn pane_count(&self) -> usize {
        self.panes.with(|panes| panes.len())
    }

    /// Open a buffer in the focused pane: focus its tab if already open, else add
    /// a tab and focus it.
    pub fn open_in_focused(&self, kind: PluginKind, id: Option<String>) {
        let key = self.focused_key.get_untracked();
        self.panes.update(|panes| {
            let index = panes.iter().position(|pane| pane.key == key).unwrap_or(0);
            if let Some(pane) = panes.get_mut(index) {
                if let Some(existing) = pane
                    .tabs
                    .iter()
                    .position(|tab| tab.kind == kind && tab.id == id)
                {
                    pane.active = existing;
                } else {
                    pane.tabs.push(BufferRef { kind, id });
                    pane.active = pane.tabs.len() - 1;
                }
            }
        });
    }

    /// Switches the active tab in a pane.
    pub fn focus_tab(&self, pane_key: usize, index: usize) {
        self.panes.update(|panes| {
            if let Some(pane) = panes.iter_mut().find(|pane| pane.key == pane_key)
                && index < pane.tabs.len()
            {
                pane.active = index;
            }
        });
        self.focused_key.set(pane_key);
    }

    /// Closes a tab in a pane, leaving the pane open even with no tabs.
    pub fn close_tab(&self, pane_key: usize, index: usize) {
        self.panes.update(|panes| {
            if let Some(pane) = panes.iter_mut().find(|pane| pane.key == pane_key)
                && index < pane.tabs.len()
            {
                pane.tabs.remove(index);
                if index < pane.active {
                    pane.active -= 1;
                }
                if pane.active >= pane.tabs.len() {
                    pane.active = pane.tabs.len().saturating_sub(1);
                }
            }
        });
    }

    /// Reorder a tab within a pane, keeping the active buffer selected.
    pub fn move_tab(&self, pane_key: usize, from: usize, to: usize) {
        self.panes.update(|panes| {
            if let Some(pane) = panes.iter_mut().find(|pane| pane.key == pane_key)
                && from < pane.tabs.len()
                && to < pane.tabs.len()
                && from != to
            {
                let active = pane.tabs.get(pane.active).cloned();
                let tab = pane.tabs.remove(from);
                pane.tabs.insert(to, tab);
                if let Some(active) = active
                    && let Some(index) = pane.tabs.iter().position(|tab| *tab == active)
                {
                    pane.active = index;
                }
            }
        });
        self.focused_key.set(pane_key);
    }

    /// Append a pane next to the focused one, cloning its buffer, and focus it.
    pub fn split(&self, vertical: bool) {
        self.split_vertical.set(vertical);
        let source = self.focused();
        let key = self
            .panes
            .with_untracked(|panes| panes.iter().map(|pane| pane.key).max().unwrap_or(0) + 1);
        self.panes.update(|panes| {
            let index = panes
                .iter()
                .position(|pane| pane.key == source.key)
                .map(|index| index + 1)
                .unwrap_or(panes.len());
            panes.insert(
                index,
                Pane {
                    key,
                    tabs: source.tabs.clone(),
                    active: source.active,
                    flex: 1.0,
                },
            );
        });
        self.focused_key.set(key);
    }

    /// Close the focused pane unless it is the only one, focusing a neighbor.
    pub fn close_focused(&self) {
        let key = self.focused_key.get_untracked();
        let neighbor = self.panes.with_untracked(|panes| {
            if panes.len() <= 1 {
                return None;
            }
            let index = panes.iter().position(|pane| pane.key == key).unwrap_or(0);
            let neighbor = if index == 0 {
                panes.get(1)
            } else {
                panes.get(index - 1)
            };
            neighbor.map(|pane| pane.key)
        });
        if let Some(next_key) = neighbor {
            self.panes
                .update(|panes| panes.retain(|pane| pane.key != key));
            self.focused_key.set(next_key);
        }
    }

    /// Move focus to the next pane, wrapping around.
    pub fn focus_next(&self) {
        self.focus_step(1);
    }

    /// Move focus to the previous pane, wrapping around.
    pub fn focus_prev(&self) {
        self.focus_step(-1);
    }

    fn focus_step(&self, delta: i64) {
        let key = self.focused_key.get_untracked();
        let next = self.panes.with_untracked(|panes| {
            if panes.is_empty() {
                return None;
            }
            let count = panes.len() as i64;
            let index = panes.iter().position(|pane| pane.key == key).unwrap_or(0) as i64;
            let next = (index + delta).rem_euclid(count) as usize;
            panes.get(next).map(|pane| pane.key)
        });
        if let Some(next_key) = next {
            self.focused_key.set(next_key);
        }
    }

    /// Switches the focused pane's active tab by a delta, wrapping around.
    pub fn cycle_tab(&self, delta: i64) {
        let key = self.focused_key.get_untracked();
        self.panes.update(|panes| {
            if let Some(pane) = panes.iter_mut().find(|pane| pane.key == key)
                && !pane.tabs.is_empty()
            {
                let count = pane.tabs.len() as i64;
                pane.active = (pane.active as i64 + delta).rem_euclid(count) as usize;
            }
        });
    }

    /// Closes the focused pane's active tab.
    pub fn close_focused_tab(&self) {
        let key = self.focused_key.get_untracked();
        let index = self.panes.with_untracked(|panes| {
            panes
                .iter()
                .find(|pane| pane.key == key)
                .map(|pane| pane.active)
        });
        if let Some(index) = index {
            self.close_tab(key, index);
        }
    }

    /// Resets every pane to an equal share of the split.
    pub fn balance_splits(&self) {
        self.panes.update(|panes| {
            for pane in panes.iter_mut() {
                pane.flex = 1.0;
            }
        });
    }

    /// Drag the divider before `right_key`, transferring flex weight from the
    /// pane on one side to the other. `delta_fraction` is the pointer move as a
    /// fraction of the editor area along the split axis.
    pub fn drag_divider(&self, right_key: usize, delta_fraction: f32) {
        self.panes.update(|panes| {
            let Some(right) = panes.iter().position(|pane| pane.key == right_key) else {
                return;
            };
            if right == 0 {
                return;
            }
            let left = right - 1;
            let total: f32 = panes.iter().map(|pane| pane.flex).sum();
            let delta = delta_fraction * total;
            let minimum = 0.1;
            if panes[left].flex + delta >= minimum && panes[right].flex - delta >= minimum {
                panes[left].flex += delta;
                panes[right].flex -= delta;
            }
        });
    }
}
