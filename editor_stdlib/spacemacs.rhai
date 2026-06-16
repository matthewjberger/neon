// Spacemacs-style keybindings: vim modal editing plus an SPC leader, modeled on
// the VSpaceCode menu tree. Pressing SPC opens the which-key menu and each prefix
// narrows it. The key letters follow VSpaceCode so the muscle memory carries over.
//
//   SPC SPC          command palette
//   SPC TAB          last buffer (previous tab)
//   SPC /            search the project
//   SPC ;            toggle comment on the line
//   SPC ?            help and keybindings
//   SPC b  b/d/n/p   buffers: list / close / next / previous
//   SPC f  f/s/S/t/n files: open folder / save / save all / tree / new
//   SPC w  v/s/-/d   windows: split right / below / below / close split
//          h/l/j/k   focus previous / next / next / previous
//          w/W/=/x   focus other / previous / balance / close split
//   SPC s  s/p///j   search: find buffer / project / project / jump word
//   SPC j  j/l/w     jump: char / line / word
//   SPC x  ;/d/j/k   text: comment / duplicate / move down / move up
//          J/>/<     join lines / indent / outdent
//   SPC t  p/c/r/a/o/l  toggles: preview / console / reference / Claude /
//                       control panel / rust-analyzer log
//   SPC e  l         errors: rust-analyzer log
//   SPC p  f/t//     project: search / files / search
//   SPC P  n/m/i     plugins: new / manager / installed
//   SPC r            run or pause     SPC T  next theme
//
// Normal mode: hjkl move, i/a/A/I insert, o open line, x/X delete, D/C to line
// end, dd delete line, J join, 0/$ ends, w/b words, / find, Escape leaves
// insert. Edit this live to change anything.

fn leader_menu() {
    #{ title: "Leader", items: [
        #{ key: "SPC", label: "Commands" },
        #{ key: "TAB", label: "Last buffer" },
        #{ key: "/", label: "Search project" },
        #{ key: ";", label: "Toggle comment" },
        #{ key: "?", label: "Help" },
        #{ key: "b", label: "+Buffers" },
        #{ key: "f", label: "+Files" },
        #{ key: "w", label: "+Windows" },
        #{ key: "s", label: "+Search" },
        #{ key: "j", label: "+Jump" },
        #{ key: "x", label: "+Text" },
        #{ key: "t", label: "+Toggles" },
        #{ key: "e", label: "+Errors" },
        #{ key: "p", label: "+Project" },
        #{ key: "P", label: "+Plugins" },
        #{ key: "r", label: "Run or pause" },
        #{ key: "T", label: "Next theme" },
    ] };
}

fn file_menu() {
    #{ title: "+Files", items: [
        #{ key: "f", label: "Open folder" },
        #{ key: "s", label: "Save" },
        #{ key: "S", label: "Save all" },
        #{ key: "t", label: "File tree" },
        #{ key: "n", label: "New plugin" },
    ] };
}

fn buffer_menu() {
    #{ title: "+Buffers", items: [
        #{ key: "b", label: "List buffers" },
        #{ key: "d", label: "Close tab" },
        #{ key: "n", label: "Next tab" },
        #{ key: "p", label: "Previous tab" },
    ] };
}

fn window_menu() {
    #{ title: "+Windows", items: [
        #{ key: "v", label: "Split right" },
        #{ key: "s", label: "Split below" },
        #{ key: "-", label: "Split below" },
        #{ key: "/", label: "Split right" },
        #{ key: "d", label: "Close split" },
        #{ key: "x", label: "Close split" },
        #{ key: "h", label: "Focus previous" },
        #{ key: "l", label: "Focus next" },
        #{ key: "j", label: "Focus next" },
        #{ key: "k", label: "Focus previous" },
        #{ key: "w", label: "Focus other" },
        #{ key: "W", label: "Focus previous" },
        #{ key: "=", label: "Balance splits" },
    ] };
}

fn search_menu() {
    #{ title: "+Search", items: [
        #{ key: "s", label: "Find in buffer" },
        #{ key: "p", label: "Search project" },
        #{ key: "/", label: "Search project" },
        #{ key: "j", label: "Jump to word" },
    ] };
}

fn jump_menu() {
    #{ title: "+Jump", items: [
        #{ key: "j", label: "Jump to char" },
        #{ key: "l", label: "Jump to line" },
        #{ key: "w", label: "Jump to word" },
    ] };
}

fn text_menu() {
    #{ title: "+Text", items: [
        #{ key: ";", label: "Toggle comment" },
        #{ key: "d", label: "Duplicate line" },
        #{ key: "j", label: "Move line down" },
        #{ key: "k", label: "Move line up" },
        #{ key: "J", label: "Join lines" },
        #{ key: ">", label: "Indent" },
        #{ key: "<", label: "Outdent" },
    ] };
}

fn toggle_menu() {
    #{ title: "+Toggles", items: [
        #{ key: "p", label: "Preview" },
        #{ key: "c", label: "Console" },
        #{ key: "r", label: "Reference" },
        #{ key: "a", label: "Claude" },
        #{ key: "o", label: "Control panel" },
        #{ key: "l", label: "rust-analyzer log" },
        #{ key: "t", label: "Next theme" },
    ] };
}

fn error_menu() {
    #{ title: "+Errors", items: [
        #{ key: "l", label: "rust-analyzer log" },
    ] };
}

fn project_menu() {
    #{ title: "+Project", items: [
        #{ key: "f", label: "Search project" },
        #{ key: "t", label: "File tree" },
        #{ key: "/", label: "Search project" },
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
        else if key == "Tab" { ops.push(#{ RunCommand: "prev-tab" }); ops.push("HideMenu"); }
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
        else if key == "x" { state.pending = "SPC x"; ops.push(#{ ShowMenu: text_menu() }); }
        else if key == "t" { state.pending = "SPC t"; ops.push(#{ ShowMenu: toggle_menu() }); }
        else if key == "e" { state.pending = "SPC e"; ops.push(#{ ShowMenu: error_menu() }); }
        else if key == "p" { state.pending = "SPC p"; ops.push(#{ ShowMenu: project_menu() }); }
        else if key == "P" { state.pending = "SPC P"; ops.push(#{ ShowMenu: plugin_menu() }); }
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
        else if key == "n" { ops.push(#{ RunCommand: "new-plugin" }); }
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
        if key == "v" || key == "/" { ops.push(#{ RunCommand: "split-right" }); }
        else if key == "s" || key == "-" { ops.push(#{ RunCommand: "split-below" }); }
        else if key == "d" || key == "x" { ops.push(#{ RunCommand: "close-split" }); }
        else if key == "w" { ops.push(#{ RunCommand: "focus-other" }); }
        else if key == "h" || key == "k" || key == "W" { ops.push(#{ RunCommand: "focus-prev" }); }
        else if key == "l" || key == "j" { ops.push(#{ RunCommand: "focus-next" }); }
        else if key == "=" { ops.push(#{ RunCommand: "balance-splits" }); }
        return;
    }
    if pending == "SPC s" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "s" { ops.push(#{ RunCommand: "find" }); }
        else if key == "p" || key == "/" { ops.push(#{ RunCommand: "show-search" }); }
        else if key == "j" { ops.push(#{ RunCommand: "jump-word" }); }
        return;
    }
    if pending == "SPC j" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "j" { ops.push(#{ RunCommand: "jump-char" }); }
        else if key == "l" { ops.push(#{ RunCommand: "jump-line" }); }
        else if key == "w" { ops.push(#{ RunCommand: "jump-word" }); }
        return;
    }
    if pending == "SPC x" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == ";" { ops.push(#{ ToggleComment: "//" }); }
        else if key == "d" { ops.push("DuplicateLine"); }
        else if key == "j" { ops.push("MoveLineDown"); }
        else if key == "k" { ops.push("MoveLineUp"); }
        else if key == "J" { ops.push("JoinLines"); }
        else if key == ">" { ops.push("Indent"); }
        else if key == "<" { ops.push("Outdent"); }
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
        else if key == "o" { ops.push(#{ RunCommand: "toggle-control-panel" }); }
        else if key == "l" { ops.push(#{ RunCommand: "toggle-lsp-log" }); }
        else if key == "t" { ops.push(#{ RunCommand: "next-theme" }); }
        return;
    }
    if pending == "SPC e" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "l" { ops.push(#{ RunCommand: "toggle-lsp-log" }); }
        return;
    }
    if pending == "SPC p" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "f" || key == "/" { ops.push(#{ RunCommand: "show-search" }); }
        else if key == "t" { ops.push(#{ RunCommand: "show-files" }); }
        return;
    }
    if pending == "SPC P" {
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
    else if key == "/" { ops.push(#{ RunCommand: "find" }); }
    else if key == "i" { ops.push(#{ SetMode: "insert" }); }
    else if key == "I" { ops.push("SmartLineStart"); ops.push(#{ SetMode: "insert" }); }
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
    else if key == "X" { ops.push(#{ DeleteBackward: 1 }); }
    else if key == "D" { ops.push("DeleteToLineEnd"); }
    else if key == "C" { ops.push("DeleteToLineEnd"); ops.push(#{ SetMode: "insert" }); }
    else if key == "J" { ops.push("JoinLines"); }
    else if key == "d" { state.pending = "d"; }
}
