// Spacemacs-style keybindings: vim modal editing plus an SPC leader, modeled on
// the VSpaceCode menu tree. Pressing SPC opens the which-key menu and each prefix
// narrows it.
//
//   SPC SPC        command palette
//   SPC /          search the project
//   SPC ;          toggle comment on the line
//   SPC f f/s/S/t  open folder / save / save all / file tree
//   SPC b b/d/n/p  buffer / close tab / next / previous
//   SPC w v/s/d/w  split right / below / close / focus other
//   SPC w h/l/=    focus previous / next / balance splits
//   SPC s s/p      find in buffer / search project
//   SPC j j/w/l    jump to char / word / line
//   SPC t p/c/r/a/l  toggle preview / console / reference / Claude / LSP log
//   SPC p n/m/i    new plugin / manager / installed
//   SPC r          run or pause     SPC T  next theme     SPC ?  help
//
// Normal mode: hjkl move, i/a/A/o insert, x delete, dd delete line, 0/$ ends,
// w/b words, Escape leaves insert. Edit this live to change anything.

fn leader_menu() {
    #{ title: "Leader", items: [
        #{ key: "SPC", label: "Commands" },
        #{ key: "/", label: "Search project" },
        #{ key: ";", label: "Toggle comment" },
        #{ key: "f", label: "+Files" },
        #{ key: "b", label: "+Buffers" },
        #{ key: "w", label: "+Windows" },
        #{ key: "s", label: "+Search" },
        #{ key: "j", label: "+Jump" },
        #{ key: "t", label: "+Toggles" },
        #{ key: "p", label: "+Plugins" },
        #{ key: "r", label: "Run or pause" },
        #{ key: "T", label: "Next theme" },
        #{ key: "?", label: "Help" },
    ] };
}

fn file_menu() {
    #{ title: "+Files", items: [
        #{ key: "f", label: "Open folder" },
        #{ key: "s", label: "Save" },
        #{ key: "S", label: "Save all" },
        #{ key: "t", label: "File tree" },
    ] };
}

fn buffer_menu() {
    #{ title: "+Buffers", items: [
        #{ key: "b", label: "Open buffer" },
        #{ key: "d", label: "Close tab" },
        #{ key: "n", label: "Next tab" },
        #{ key: "p", label: "Previous tab" },
    ] };
}

fn window_menu() {
    #{ title: "+Windows", items: [
        #{ key: "v", label: "Split right" },
        #{ key: "s", label: "Split below" },
        #{ key: "d", label: "Close split" },
        #{ key: "w", label: "Focus other" },
        #{ key: "h", label: "Focus previous" },
        #{ key: "l", label: "Focus next" },
        #{ key: "=", label: "Balance splits" },
    ] };
}

fn search_menu() {
    #{ title: "+Search", items: [
        #{ key: "s", label: "Find in buffer" },
        #{ key: "p", label: "Search project" },
    ] };
}

fn jump_menu() {
    #{ title: "+Jump", items: [
        #{ key: "j", label: "Jump to char" },
        #{ key: "w", label: "Jump to word" },
        #{ key: "l", label: "Jump to line" },
    ] };
}

fn toggle_menu() {
    #{ title: "+Toggles", items: [
        #{ key: "p", label: "Preview" },
        #{ key: "c", label: "Console" },
        #{ key: "r", label: "Reference" },
        #{ key: "a", label: "Claude" },
        #{ key: "l", label: "LSP log" },
        #{ key: "t", label: "Next theme" },
    ] };
}

fn plugin_menu() {
    #{ title: "+Plugins", items: [
        #{ key: "n", label: "New plugin" },
        #{ key: "m", label: "Plugin manager" },
        #{ key: "i", label: "Installed" },
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
        else if key == "/" { ops.push(#{ RunCommand: "show-search" }); ops.push("HideMenu"); }
        else if key == ";" { ops.push(#{ ToggleComment: "//" }); ops.push("HideMenu"); }
        else if key == "?" { ops.push(#{ RunCommand: "open-help" }); ops.push("HideMenu"); }
        else if key == "r" { ops.push(#{ RunCommand: "run-pause" }); ops.push("HideMenu"); }
        else if key == "T" { ops.push(#{ RunCommand: "next-theme" }); ops.push("HideMenu"); }
        else if key == "f" { state.pending = "SPC f"; ops.push(#{ ShowMenu: file_menu() }); }
        else if key == "b" { state.pending = "SPC b"; ops.push(#{ ShowMenu: buffer_menu() }); }
        else if key == "w" { state.pending = "SPC w"; ops.push(#{ ShowMenu: window_menu() }); }
        else if key == "s" { state.pending = "SPC s"; ops.push(#{ ShowMenu: search_menu() }); }
        else if key == "j" { state.pending = "SPC j"; ops.push(#{ ShowMenu: jump_menu() }); }
        else if key == "t" { state.pending = "SPC t"; ops.push(#{ ShowMenu: toggle_menu() }); }
        else if key == "p" { state.pending = "SPC p"; ops.push(#{ ShowMenu: plugin_menu() }); }
        else { ops.push("HideMenu"); }
        return;
    }
    if pending == "SPC f" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "f" { ops.push(#{ RunCommand: "open-folder" }); }
        else if key == "s" { ops.push(#{ RunCommand: "save-file" }); }
        else if key == "S" { ops.push(#{ RunCommand: "save-all" }); }
        else if key == "t" { ops.push(#{ RunCommand: "show-files" }); }
        return;
    }
    if pending == "SPC b" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "b" { ops.push(#{ RunCommand: "open-palette" }); }
        else if key == "d" { ops.push(#{ RunCommand: "close-tab" }); }
        else if key == "n" { ops.push(#{ RunCommand: "next-tab" }); }
        else if key == "p" { ops.push(#{ RunCommand: "prev-tab" }); }
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
        else if key == "h" { ops.push(#{ RunCommand: "focus-prev" }); }
        else if key == "l" { ops.push(#{ RunCommand: "focus-next" }); }
        else if key == "=" { ops.push(#{ RunCommand: "balance-splits" }); }
        return;
    }
    if pending == "SPC s" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "s" { ops.push(#{ RunCommand: "find" }); }
        else if key == "p" { ops.push(#{ RunCommand: "show-search" }); }
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
    if pending == "SPC t" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "p" { ops.push(#{ RunCommand: "toggle-preview" }); }
        else if key == "c" { ops.push(#{ RunCommand: "toggle-console" }); }
        else if key == "r" { ops.push(#{ RunCommand: "toggle-reference" }); }
        else if key == "a" { ops.push(#{ RunCommand: "toggle-chat" }); }
        else if key == "l" { ops.push(#{ RunCommand: "toggle-lsp-log" }); }
        else if key == "t" { ops.push(#{ RunCommand: "next-theme" }); }
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
