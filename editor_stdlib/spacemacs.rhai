// Spacemacs-style keybindings: vim modal editing plus an SPC leader that runs
// editor commands. Pressing SPC opens the which-key menu (the bottom panel that
// lists the next keys and what they do), and each submenu narrows it. The leader
// menus are where window management lives.
//
//   SPC SPC        command palette
//   SPC w v / s    split the editor right / below
//   SPC w d        close the split
//   SPC w w        focus the other pane
//   SPC t p/c/r/a  toggle preview / console / reference / Claude
//   SPC b b        open a buffer
//   SPC p n/m/i    new plugin / manager / installed
//   SPC r          run or pause
//   SPC T          next theme
//   SPC ?          help
//   :              command palette
//
// Normal mode: hjkl move, i/a/A/o insert, x delete, dd delete line, 0/$ ends,
// w/b words, Escape leaves insert. Edit this live to change anything.

fn leader_menu() {
    #{ title: "Leader", items: [
        #{ key: "SPC", label: "Commands" },
        #{ key: "w", label: "+Windows" },
        #{ key: "t", label: "+Toggles" },
        #{ key: "b", label: "+Buffers" },
        #{ key: "p", label: "+Plugins" },
        #{ key: "j", label: "+Jump" },
        #{ key: "r", label: "Run or pause" },
        #{ key: "T", label: "Next theme" },
        #{ key: "?", label: "Help" },
    ] };
}

fn window_menu() {
    #{ title: "+Windows", items: [
        #{ key: "v", label: "Split right" },
        #{ key: "s", label: "Split below" },
        #{ key: "d", label: "Close split" },
        #{ key: "w", label: "Focus other pane" },
    ] };
}

fn toggle_menu() {
    #{ title: "+Toggles", items: [
        #{ key: "p", label: "Preview" },
        #{ key: "c", label: "Console" },
        #{ key: "r", label: "Reference" },
        #{ key: "a", label: "Claude" },
    ] };
}

fn buffer_menu() {
    #{ title: "+Buffers", items: [
        #{ key: "b", label: "Open buffer" },
    ] };
}

fn plugin_menu() {
    #{ title: "+Plugins", items: [
        #{ key: "n", label: "New plugin" },
        #{ key: "m", label: "Plugin manager" },
        #{ key: "i", label: "Installed" },
    ] };
}

fn jump_menu() {
    #{ title: "+Jump", items: [
        #{ key: "j", label: "Jump to char" },
        #{ key: "w", label: "Jump to word" },
        #{ key: "l", label: "Jump to line" },
    ] };
}

fn on_key() {
    if mode == "insert" {
        if key == "Escape" {
            ops.push("Consume");
            ops.push(#{ SetMode: "normal" });
            ops.push(#{ Move: -1 });
        }
        return;
    }

    if ctrl || alt {
        return;
    }

    let pending = if "pending" in state { state.pending } else { "" };

    if pending == "SPC" {
        ops.push("Consume");
        state.pending = "";
        if key == " " { ops.push(#{ RunCommand: "open-palette" }); ops.push("HideMenu"); }
        else if key == "?" { ops.push(#{ RunCommand: "open-help" }); ops.push("HideMenu"); }
        else if key == "r" { ops.push(#{ RunCommand: "run-pause" }); ops.push("HideMenu"); }
        else if key == "T" { ops.push(#{ RunCommand: "next-theme" }); ops.push("HideMenu"); }
        else if key == "w" { state.pending = "SPC w"; ops.push(#{ ShowMenu: window_menu() }); }
        else if key == "t" { state.pending = "SPC t"; ops.push(#{ ShowMenu: toggle_menu() }); }
        else if key == "b" { state.pending = "SPC b"; ops.push(#{ ShowMenu: buffer_menu() }); }
        else if key == "p" { state.pending = "SPC p"; ops.push(#{ ShowMenu: plugin_menu() }); }
        else if key == "j" { state.pending = "SPC j"; ops.push(#{ ShowMenu: jump_menu() }); }
        else { ops.push("HideMenu"); }
        return;
    }
    if pending == "SPC j" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "j" { ops.push(#{ RunCommand: "jump-char" }); }
        else if key == "w" { ops.push(#{ RunCommand: "jump-word" }); }
        else if key == "l" { ops.push(#{ RunCommand: "jump-line" }); }
        return;
    }
    if pending == "SPC w" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "v" { ops.push(#{ RunCommand: "split-right" }); }
        else if key == "s" { ops.push(#{ RunCommand: "split-below" }); }
        else if key == "d" { ops.push(#{ RunCommand: "close-split" }); }
        else if key == "w" { ops.push(#{ RunCommand: "focus-other" }); }
        return;
    }
    if pending == "SPC t" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "p" { ops.push(#{ RunCommand: "toggle-preview" }); }
        else if key == "c" { ops.push(#{ RunCommand: "toggle-console" }); }
        else if key == "r" { ops.push(#{ RunCommand: "toggle-reference" }); }
        else if key == "a" { ops.push(#{ RunCommand: "toggle-chat" }); }
        return;
    }
    if pending == "SPC b" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "b" { ops.push(#{ RunCommand: "open-palette" }); }
        return;
    }
    if pending == "SPC p" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "n" { ops.push(#{ RunCommand: "new-plugin" }); }
        else if key == "m" { ops.push(#{ RunCommand: "show-manager" }); }
        else if key == "i" { ops.push(#{ RunCommand: "show-installed" }); }
        return;
    }
    if pending == "d" {
        ops.push("Consume");
        state.pending = "";
        if key == "d" { ops.push("DeleteLine"); }
        return;
    }

    ops.push("Consume");
    if key == " " { state.pending = "SPC"; ops.push(#{ ShowMenu: leader_menu() }); }
    else if key == ":" { ops.push(#{ RunCommand: "open-palette" }); }
    else if key == "i" { ops.push(#{ SetMode: "insert" }); }
    else if key == "a" { ops.push(#{ Move: 1 }); ops.push(#{ SetMode: "insert" }); }
    else if key == "A" { ops.push("LineEnd"); ops.push(#{ SetMode: "insert" }); }
    else if key == "o" { ops.push("LineEnd"); ops.push(#{ Insert: "\n" }); ops.push(#{ SetMode: "insert" }); }
    else if key == "h" || key == "ArrowLeft" { ops.push(#{ Move: -1 }); }
    else if key == "l" || key == "ArrowRight" { ops.push(#{ Move: 1 }); }
    else if key == "j" || key == "ArrowDown" { ops.push(#{ MoveLine: 1 }); }
    else if key == "k" || key == "ArrowUp" { ops.push(#{ MoveLine: -1 }); }
    else if key == "0" { ops.push("LineStart"); }
    else if key == "$" { ops.push("LineEnd"); }
    else if key == "w" { ops.push("NextWord"); }
    else if key == "b" { ops.push("PrevWord"); }
    else if key == "x" { ops.push(#{ DeleteForward: 1 }); }
    else if key == "d" { state.pending = "d"; }
}
