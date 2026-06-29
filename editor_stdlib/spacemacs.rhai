// Spacemacs-style keybindings: vim modal editing plus an SPC leader, modeled on
// the VSpaceCode menu tree. Pressing SPC opens the which-key menu and each prefix
// narrows it. The key letters follow VSpaceCode so the muscle memory carries over.
//
//   SPC SPC          command palette
//   SPC '            toggle the terminal
//   SPC TAB          last buffer (previous tab)
//   SPC /            search the project
//   SPC ;            toggle comment on the line
//   SPC ?            help and keybindings
//   SPC b  b/d/n/p/R buffers: list / close / next / previous / revert
//   SPC f  f/s/S/r   files: open folder / save / save all / revert
//          t/n/N/R/D tree / new file / new plugin / rename / delete
//   SPC w  v/s/-/d   windows: split right / below / below / close split
//          h/l/j/k   focus previous / next / next / previous
//          w/W/=/x/n focus other / previous / balance / close split / new window
//   SPC s  s/p/r/j/J search: find buffer / project / references / symbol / workspace
//   SPC j  j/l/w/i/+ jump: char / line / word / symbol / format
//   SPC g  g/t/i/r/R/s/S  goto: definition (also "gd") / type / impl /
//                         references / rename / symbol / workspace symbols
//   SPC h  h/s/k/t   help: hover (also "K") / signature / keybindings / tour
//   SPC a            code action (also SPC x .)
//   SPC c  c/b/t/r/k/o  cargo: check / build / test / run / interrupt / terminal
//   SPC x  ;/d/j/k   text: comment / duplicate / move down / move up
//   SPC x  u/U/s/w/r case lower / upper / sort lines / trim / rename
//          i/R/.     organize imports / refactor / code action
//          J/>/<     join lines / indent / outdent
//   SPC t  p/c/r/a/o/l/f/u/g/s/t  toggles: preview / console / reference /
//                       Claude / control panel / rust-analyzer log /
//                       format on save / undo tree / source control /
//                       outline / next theme
//   SPC e  n/p/f/e/l errors: next / previous / quick fix / problems / log
//   SPC p  f/t//     project: search / files / search
//   SPC P  n/m/i     plugins: new / manager / installed
//   SPC r            run or pause     SPC T  next theme
//
// Normal mode: hjkl move, i/a/A/I insert, o/O open line, x/X delete, D/C to line
// end, dd delete line, J join, 0/$ ends, w/b/e words, W/B/E WORDs, ge prev end,
// % match pair, ~ toggle case, gu/gU/g~ case ops, / find, Escape leaves insert.
//
// Operators and text objects: d, c, y take any motion (dw, d$, d0, db, dj, dG,
// 3dw), a find (dt) df), a text object (diw, ci", daw), gg (dgg), or the doubled
// form (dd, cc, yy). r replaces a char, O opens a line above, >> / << indent the
// line. v starts a charwise selection, V a linewise one; in visual the motions
// extend it and d/c/x/s act on it, > / < indent, u / U change case, gc comments,
// i/a select a text object, S wraps it (S) ' " ( [ { < `, and + / - grow / shrink
// the selection to the enclosing syntax (expand-region).
//
// Yank and paste: yy yanks the line, yiw / yi" yank an object, y in visual yanks
// the selection; p / P paste. "a before a yank or paste uses register a.
//
// Search and find: * / # jump to the next/previous match of the word under the
// caret, n / N repeat it; f / F / t / T jump to a character on the line and ; / ,
// repeat that.
//
// Repeat and jumps: . replays the last change, a count repeats a motion (5j,
// 3l), gg / G go to the top / bottom (42G to a line), zz / zt / zb center / top /
// bottom the view, H / M / L move to the top / middle / bottom line.
//
// Marks and the jump list: m{a} sets mark a, `a jumps to it, 'a to its line;
// Ctrl-o / Ctrl-i walk the jump list; Ctrl-d / Ctrl-u scroll a half page.
//
// Macros: q{a} records keystrokes into register a, q stops, @{a} replays, @@
// replays the last. Edit this live to change anything.

fn leader_menu() {
    #{ title: "Leader", items: [
        #{ key: "SPC", label: "Commands" },
        #{ key: "'", label: "Terminal" },
        #{ key: "TAB", label: "Last buffer" },
        #{ key: "/", label: "Search project" },
        #{ key: ";", label: "Toggle comment" },
        #{ key: "?", label: "Help" },
        #{ key: "b", label: "+Buffers" },
        #{ key: "f", label: "+Files" },
        #{ key: "w", label: "+Windows" },
        #{ key: "s", label: "+Search" },
        #{ key: "j", label: "+Jump" },
        #{ key: "g", label: "+Goto" },
        #{ key: "c", label: "+Cargo" },
        #{ key: "h", label: "+Help" },
        #{ key: "a", label: "Code action" },
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
        #{ key: "r", label: "Revert file" },
        #{ key: "t", label: "File tree" },
        #{ key: "n", label: "New file" },
        #{ key: "N", label: "New plugin" },
        #{ key: "R", label: "Rename file" },
        #{ key: "D", label: "Delete file" },
    ] };
}

fn buffer_menu() {
    #{ title: "+Buffers", items: [
        #{ key: "b", label: "List buffers" },
        #{ key: "d", label: "Close tab" },
        #{ key: "n", label: "Next tab" },
        #{ key: "p", label: "Previous tab" },
        #{ key: "R", label: "Revert buffer" },
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
        #{ key: "n", label: "New window" },
    ] };
}

fn search_menu() {
    #{ title: "+Search", items: [
        #{ key: "s", label: "Find in buffer" },
        #{ key: "p", label: "Search project" },
        #{ key: "/", label: "Search project" },
        #{ key: "r", label: "Find references" },
        #{ key: "j", label: "Jump to symbol" },
        #{ key: "J", label: "Workspace symbols" },
    ] };
}

fn jump_menu() {
    #{ title: "+Jump", items: [
        #{ key: "j", label: "Jump to char" },
        #{ key: "l", label: "Jump to line" },
        #{ key: "w", label: "Jump to word" },
        #{ key: "i", label: "Jump to symbol" },
        #{ key: "+", label: "Format buffer" },
        #{ key: "=", label: "Format buffer" },
    ] };
}

fn cargo_menu() {
    #{ title: "+Cargo", items: [
        #{ key: "c", label: "Check" },
        #{ key: "b", label: "Build" },
        #{ key: "t", label: "Test" },
        #{ key: "r", label: "Run" },
        #{ key: "k", label: "Interrupt" },
        #{ key: "o", label: "Terminal" },
    ] };
}

fn goto_menu() {
    #{ title: "+Goto", items: [
        #{ key: "g", label: "Go to definition" },
        #{ key: "d", label: "Go to definition" },
        #{ key: "t", label: "Go to type definition" },
        #{ key: "i", label: "Go to implementation" },
        #{ key: "r", label: "Find references" },
        #{ key: "R", label: "Rename symbol" },
        #{ key: "s", label: "Jump to symbol" },
        #{ key: "S", label: "Workspace symbols" },
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
        #{ key: "u", label: "Lower case word" },
        #{ key: "U", label: "Upper case word" },
        #{ key: "s", label: "Sort lines" },
        #{ key: "w", label: "Delete trailing space" },
        #{ key: "r", label: "Rename symbol" },
        #{ key: "i", label: "Organize imports" },
        #{ key: "R", label: "Refactor (code action)" },
        #{ key: ".", label: "Code action" },
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
        #{ key: "f", label: "Format on save" },
        #{ key: "u", label: "Undo tree" },
        #{ key: "g", label: "Source control" },
        #{ key: "s", label: "Outline" },
        #{ key: "t", label: "Next theme" },
    ] };
}

fn help_menu() {
    #{ title: "+Help", items: [
        #{ key: "h", label: "Describe symbol (hover)" },
        #{ key: "s", label: "Signature help" },
        #{ key: "k", label: "Keybindings" },
        #{ key: "t", label: "Interactive tour" },
    ] };
}

fn error_menu() {
    #{ title: "+Errors", items: [
        #{ key: "n", label: "Next error" },
        #{ key: "p", label: "Previous error" },
        #{ key: "f", label: "Quick fix" },
        #{ key: "e", label: "List problems" },
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

// A pending count string as an integer, or the fallback when none was typed.
fn count_or(text, fallback) {
    if text == "" { return fallback; }
    let value = parse_int(text);
    if value < 1 { 1 } else { value }
}

// The ops that perform a charwise motion key, repeated by the count. Empty when
// the key is not a motion an operator can take.
fn motion_for(key, count) {
    let n = count_or(count, 1);
    if key == "h" || key == "ArrowLeft" { return [#{ Move: -n }]; }
    if key == "l" || key == "ArrowRight" { return [#{ Move: n }]; }
    if key == "j" || key == "ArrowDown" { return [#{ MoveLine: n }]; }
    if key == "k" || key == "ArrowUp" { return [#{ MoveLine: -n }]; }
    if key == "w" { let ops = []; for step in 0..n { ops.push("NextWord"); } return ops; }
    if key == "b" { let ops = []; for step in 0..n { ops.push("PrevWord"); } return ops; }
    if key == "e" { let ops = []; for step in 0..n { ops.push("WordEnd"); } return ops; }
    if key == "E" { let ops = []; for step in 0..n { ops.push("BigWordEnd"); } return ops; }
    if key == "W" { let ops = []; for step in 0..n { ops.push("BigWord"); } return ops; }
    if key == "B" { let ops = []; for step in 0..n { ops.push("BigWordBack"); } return ops; }
    if key == "0" { return ["LineStart"]; }
    if key == "$" { return ["LineEnd"]; }
    if key == "G" { return ["BufferEnd"]; }
    if key == "%" { return ["MatchPair"]; }
    return [];
}

// Whether an operator motion includes its endpoint, so the operator extends one
// more before cutting (e, E, and % land on the last char).
fn motion_inclusive(key) {
    return key == "e" || key == "E" || key == "%";
}

// The ops that finish an operator once its range is selected: d cuts, c cuts and
// enters insert, y copies.
fn finish_for(op) {
    if op == "d" { return ["Cut"]; }
    if op == "c" { return ["Cut", #{ SetMode: "insert" }]; }
    return ["Copy"];
}

// The bracket or quote pair a surround key names, or () for an unknown key.
fn surround_pair(key) {
    if key == "(" || key == ")" || key == "b" { return ["(", ")"]; }
    if key == "{" || key == "}" || key == "B" { return ["{", "}"]; }
    if key == "[" || key == "]" { return ["[", "]"]; }
    if key == "<" || key == ">" { return ["<", ">"]; }
    if key == "\"" { return ["\"", "\""]; }
    if key == "'" { return ["'", "'"]; }
    if key == "`" { return ["`", "`"]; }
    return ();
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

    if ctrl && key == "r" {
        ops.push("Consume");
        ops.push(#{ RunCommand: "redo" });
        return;
    }
    if ctrl && key == "o" { ops.push("Consume"); ops.push("JumpBack"); return; }
    if ctrl && key == "i" { ops.push("Consume"); ops.push("JumpForward"); return; }
    if ctrl && key == "d" { ops.push("Consume"); ops.push("HalfPageDown"); return; }
    if ctrl && key == "u" { ops.push("Consume"); ops.push("HalfPageUp"); return; }

    if ctrl || alt {
        return;
    }

    // Visual mode: the mark is live, so motions paint the selection and the
    // operators act on it. i/a pick a text object, S surrounds, gc comments.
    if mode == "visual" {
        let vpending = if "vpending" in state { state.vpending } else { "" };
        ops.push("Consume");
        if vpending == "S" {
            state.vpending = "";
            let pair = surround_pair(key);
            if pair != () { ops.push(#{ Surround: pair }); }
            ops.push(#{ SetMode: "normal" });
            return;
        }
        if vpending == "g" {
            state.vpending = "";
            if key == "c" { ops.push(#{ ToggleComment: "//" }); ops.push(#{ SetMode: "normal" }); }
            return;
        }
        if vpending == "i" || vpending == "a" {
            let around = vpending == "a";
            state.vpending = "";
            if around { ops.push(#{ SelectAround: key }); } else { ops.push(#{ SelectInner: key }); }
            return;
        }
        if vpending == "f" || vpending == "F" || vpending == "t" || vpending == "T" {
            let kind = vpending;
            state.vpending = "";
            if kind == "f" { ops.push(#{ FindChar: key }); }
            else if kind == "F" { ops.push(#{ FindCharBack: key }); }
            else if kind == "t" { ops.push(#{ TillChar: key }); }
            else if kind == "T" { ops.push(#{ TillCharBack: key }); }
            return;
        }
        if key == "h" || key == "ArrowLeft" { ops.push(#{ Move: -1 }); }
        else if key == "l" || key == "ArrowRight" { ops.push(#{ Move: 1 }); }
        else if key == "j" || key == "ArrowDown" { ops.push(#{ MoveLine: 1 }); }
        else if key == "k" || key == "ArrowUp" { ops.push(#{ MoveLine: -1 }); }
        else if key == "0" { ops.push("LineStart"); }
        else if key == "$" { ops.push("LineEnd"); }
        else if key == "w" { ops.push("NextWord"); }
        else if key == "b" { ops.push("PrevWord"); }
        else if key == "e" { ops.push("WordEnd"); }
        else if key == "E" { ops.push("BigWordEnd"); }
        else if key == "W" { ops.push("BigWord"); }
        else if key == "B" { ops.push("BigWordBack"); }
        else if key == "%" { ops.push("MatchPair"); }
        else if key == "~" { ops.push("ToggleCase"); ops.push(#{ SetMode: "normal" }); }
        else if key == "+" || key == "=" { ops.push("ExpandSelection"); }
        else if key == "-" { ops.push("ShrinkSelection"); }
        else if key == "o" { ops.push("SwapEnds"); }
        else if key == "f" { state.vpending = "f"; }
        else if key == "F" { state.vpending = "F"; }
        else if key == "t" { state.vpending = "t"; }
        else if key == "T" { state.vpending = "T"; }
        else if key == ";" { ops.push("RepeatFind"); }
        else if key == "," { ops.push("RepeatFindBack"); }
        else if key == "n" { ops.push("SearchNext"); }
        else if key == "N" { ops.push("SearchPrev"); }
        else if key == "i" { state.vpending = "i"; }
        else if key == "a" { state.vpending = "a"; }
        else if key == "g" { state.vpending = "g"; }
        else if key == "S" { state.vpending = "S"; }
        else if key == "y" { ops.push("Copy"); ops.push(#{ SetMode: "normal" }); }
        else if key == "p" || key == "P" { ops.push("Paste"); ops.push(#{ SetMode: "normal" }); }
        else if key == "d" || key == "x" { ops.push("Cut"); ops.push(#{ SetMode: "normal" }); }
        else if key == "c" || key == "s" { ops.push("Cut"); ops.push(#{ SetMode: "insert" }); }
        else if key == ">" { ops.push("Indent"); ops.push(#{ SetMode: "normal" }); }
        else if key == "<" { ops.push("Outdent"); ops.push(#{ SetMode: "normal" }); }
        else if key == "u" { ops.push("LowerCaseWord"); ops.push(#{ SetMode: "normal" }); }
        else if key == "U" { ops.push("UpperCaseWord"); ops.push(#{ SetMode: "normal" }); }
        else if key == "Escape" { ops.push("Collapse"); ops.push(#{ SetMode: "normal" }); }
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
        else if key == "'" { ops.push(#{ RunCommand: "toggle-terminal" }); ops.push("HideMenu"); }
        else if key == "a" { ops.push(#{ RunCommand: "code-action" }); ops.push("HideMenu"); }
        else if key == "T" { ops.push(#{ RunCommand: "next-theme" }); ops.push("HideMenu"); }
        else if key == "f" { state.pending = "SPC f"; ops.push(#{ ShowMenu: file_menu() }); }
        else if key == "b" { state.pending = "SPC b"; ops.push(#{ ShowMenu: buffer_menu() }); }
        else if key == "w" { state.pending = "SPC w"; ops.push(#{ ShowMenu: window_menu() }); }
        else if key == "s" { state.pending = "SPC s"; ops.push(#{ ShowMenu: search_menu() }); }
        else if key == "j" { state.pending = "SPC j"; ops.push(#{ ShowMenu: jump_menu() }); }
        else if key == "g" { state.pending = "SPC g"; ops.push(#{ ShowMenu: goto_menu() }); }
        else if key == "c" { state.pending = "SPC c"; ops.push(#{ ShowMenu: cargo_menu() }); }
        else if key == "h" { state.pending = "SPC h"; ops.push(#{ ShowMenu: help_menu() }); }
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
        else if key == "r" { ops.push(#{ RunCommand: "revert-file" }); }
        else if key == "t" { ops.push(#{ RunCommand: "show-files" }); }
        else if key == "n" { ops.push(#{ RunCommand: "new-file" }); }
        else if key == "N" { ops.push(#{ RunCommand: "new-plugin" }); }
        else if key == "R" { ops.push(#{ RunCommand: "rename-entry" }); }
        else if key == "D" { ops.push(#{ RunCommand: "delete-entry" }); }
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
        else if key == "R" { ops.push(#{ RunCommand: "revert-file" }); }
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
        else if key == "n" { ops.push(#{ RunCommand: "new-window" }); }
        return;
    }
    if pending == "SPC s" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "s" { ops.push(#{ RunCommand: "find" }); }
        else if key == "p" || key == "/" { ops.push(#{ RunCommand: "show-search" }); }
        else if key == "r" { ops.push(#{ RunCommand: "find-references" }); }
        else if key == "j" { ops.push(#{ RunCommand: "jump-symbol" }); }
        else if key == "J" { ops.push(#{ RunCommand: "workspace-symbols" }); }
        return;
    }
    if pending == "SPC j" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "j" { ops.push(#{ RunCommand: "jump-char" }); }
        else if key == "l" { ops.push(#{ RunCommand: "jump-line" }); }
        else if key == "w" { ops.push(#{ RunCommand: "jump-word" }); }
        else if key == "i" { ops.push(#{ RunCommand: "jump-symbol" }); }
        else if key == "+" || key == "=" { ops.push(#{ RunCommand: "format-document" }); }
        return;
    }
    if pending == "SPC g" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "g" || key == "d" { ops.push(#{ RunCommand: "go-to-definition" }); }
        else if key == "t" { ops.push(#{ RunCommand: "go-to-type-definition" }); }
        else if key == "i" { ops.push(#{ RunCommand: "go-to-implementation" }); }
        else if key == "r" { ops.push(#{ RunCommand: "find-references" }); }
        else if key == "R" { ops.push(#{ RunCommand: "rename-symbol" }); }
        else if key == "s" { ops.push(#{ RunCommand: "jump-symbol" }); }
        else if key == "S" { ops.push(#{ RunCommand: "workspace-symbols" }); }
        return;
    }
    if pending == "SPC c" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "c" { ops.push(#{ RunCommand: "cargo-check" }); }
        else if key == "b" { ops.push(#{ RunCommand: "cargo-build" }); }
        else if key == "t" { ops.push(#{ RunCommand: "cargo-test" }); }
        else if key == "r" { ops.push(#{ RunCommand: "cargo-run" }); }
        else if key == "k" { ops.push(#{ RunCommand: "interrupt" }); }
        else if key == "o" { ops.push(#{ RunCommand: "toggle-terminal" }); }
        return;
    }
    if pending == "SPC h" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "h" { ops.push(#{ RunCommand: "hover" }); }
        else if key == "s" { ops.push(#{ RunCommand: "signature-help" }); }
        else if key == "k" { ops.push(#{ RunCommand: "open-help" }); }
        else if key == "t" { ops.push(#{ RunCommand: "tour" }); }
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
        else if key == "u" { ops.push("LowerCaseWord"); }
        else if key == "U" { ops.push("UpperCaseWord"); }
        else if key == "s" { ops.push("SortLines"); }
        else if key == "w" { ops.push("DeleteTrailingWhitespace"); }
        else if key == "r" { ops.push(#{ RunCommand: "rename-symbol" }); }
        else if key == "i" { ops.push(#{ RunCommand: "organize-imports" }); }
        else if key == "R" { ops.push(#{ RunCommand: "code-action" }); }
        else if key == "." { ops.push(#{ RunCommand: "code-action" }); }
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
        else if key == "f" { ops.push(#{ RunCommand: "toggle-format-on-save" }); }
        else if key == "u" { ops.push(#{ RunCommand: "toggle-undo-tree" }); }
        else if key == "g" { ops.push(#{ RunCommand: "toggle-git" }); }
        else if key == "s" { ops.push(#{ RunCommand: "toggle-outline" }); }
        else if key == "t" { ops.push(#{ RunCommand: "next-theme" }); }
        return;
    }
    if pending == "SPC e" {
        ops.push("Consume");
        ops.push("HideMenu");
        state.pending = "";
        if key == "n" { ops.push(#{ RunCommand: "next-error" }); }
        else if key == "p" { ops.push(#{ RunCommand: "prev-error" }); }
        else if key == "f" { ops.push(#{ RunCommand: "code-action" }); }
        else if key == "e" { ops.push(#{ RunCommand: "toggle-problems" }); }
        else if key == "l" { ops.push(#{ RunCommand: "toggle-lsp-log" }); }
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
    // Operators d/c/y take a text object (i/a), a find (f/F/t/T), gg, the doubled
    // form (dd/cc/yy = the whole line), or any charwise motion.
    if pending == "d" || pending == "c" || pending == "y" {
        ops.push("Consume");
        let op = pending;
        let opcount = if "opcount" in state { state.opcount } else { "" };
        if key == "i" || key == "a" { state.pending = op + key; return; }
        if key == "f" || key == "F" || key == "t" || key == "T" { state.pending = op + key; return; }
        if key == "g" { state.pending = op + "g"; return; }
        state.pending = "";
        state.opcount = "";
        if key == op {
            if op == "d" { for step in 0..count_or(opcount, 1) { ops.push("DeleteLine"); } }
            else if op == "y" { ops.push("Copy"); }
            else { ops.push("LineStart"); ops.push("DeleteToLineEnd"); ops.push(#{ SetMode: "insert" }); }
            return;
        }
        let motion = motion_for(key, opcount);
        if motion.len() > 0 {
            ops.push("Anchor");
            for o in motion { ops.push(o); }
            if motion_inclusive(key) { ops.push(#{ Move: 1 }); }
            for o in finish_for(op) { ops.push(o); }
        }
        return;
    }
    if pending == "gu" || pending == "gU" || pending == "g~" {
        ops.push("Consume");
        let cop = pending;
        state.pending = "";
        let case_op = if cop == "gu" { "LowerCaseWord" } else if cop == "gU" { "UpperCaseWord" } else { "ToggleCase" };
        let doubled = (cop == "gu" && key == "u") || (cop == "gU" && key == "U") || (cop == "g~" && key == "~");
        if doubled {
            ops.push("SelectLine");
            ops.push(case_op);
        } else {
            let motion = motion_for(key, "");
            if motion.len() > 0 {
                ops.push("Anchor");
                for o in motion { ops.push(o); }
                if motion_inclusive(key) { ops.push(#{ Move: 1 }); }
                ops.push(case_op);
            }
        }
        return;
    }
    if pending == "di" || pending == "da" || pending == "ci" || pending == "ca"
        || pending == "yi" || pending == "ya" {
        ops.push("Consume");
        let op = pending.sub_string(0, 1);
        let around = pending.sub_string(1, 1) == "a";
        state.pending = "";
        if around { ops.push(#{ SelectAround: key }); } else { ops.push(#{ SelectInner: key }); }
        for o in finish_for(op) { ops.push(o); }
        return;
    }
    if pending.len() == 2 && (pending.sub_string(0, 1) == "d" || pending.sub_string(0, 1) == "c"
        || pending.sub_string(0, 1) == "y")
        && (pending.sub_string(1, 1) == "f" || pending.sub_string(1, 1) == "F"
        || pending.sub_string(1, 1) == "t" || pending.sub_string(1, 1) == "T") {
        ops.push("Consume");
        let op = pending.sub_string(0, 1);
        let fk = pending.sub_string(1, 1);
        state.pending = "";
        ops.push("Anchor");
        if fk == "f" { ops.push(#{ FindChar: key }); ops.push(#{ Move: 1 }); }
        else if fk == "t" { ops.push(#{ FindChar: key }); }
        else if fk == "F" { ops.push(#{ FindCharBack: key }); }
        else if fk == "T" { ops.push(#{ FindCharBack: key }); ops.push(#{ Move: 1 }); }
        for o in finish_for(op) { ops.push(o); }
        return;
    }
    if pending.len() == 2 && pending.sub_string(1, 1) == "g"
        && (pending.sub_string(0, 1) == "d" || pending.sub_string(0, 1) == "c"
        || pending.sub_string(0, 1) == "y") {
        ops.push("Consume");
        let op = pending.sub_string(0, 1);
        state.pending = "";
        if key == "g" {
            ops.push("Anchor");
            ops.push("BufferStart");
            for o in finish_for(op) { ops.push(o); }
        }
        return;
    }
    if pending == "\"" {
        ops.push("Consume");
        state.pending = "";
        ops.push(#{ Register: key });
        return;
    }
    if pending == "f" || pending == "F" || pending == "t" || pending == "T" {
        ops.push("Consume");
        let kind = pending;
        state.pending = "";
        if kind == "f" { ops.push(#{ FindChar: key }); }
        else if kind == "F" { ops.push(#{ FindCharBack: key }); }
        else if kind == "t" { ops.push(#{ TillChar: key }); }
        else if kind == "T" { ops.push(#{ TillCharBack: key }); }
        return;
    }
    if pending == "g" {
        ops.push("Consume");
        if key == "u" { state.pending = "gu"; return; }
        if key == "U" { state.pending = "gU"; return; }
        if key == "~" { state.pending = "g~"; return; }
        state.pending = "";
        if key == "d" { ops.push(#{ RunCommand: "go-to-definition" }); }
        else if key == "c" { ops.push(#{ ToggleComment: "//" }); }
        else if key == "g" { ops.push("BufferStart"); }
        else if key == "e" { ops.push("WordEndBack"); }
        return;
    }
    if pending == "z" {
        ops.push("Consume");
        state.pending = "";
        if key == "z" { ops.push("Center"); }
        else if key == "t" { ops.push("ScrollLineTop"); }
        else if key == "b" { ops.push("ScrollLineBottom"); }
        return;
    }
    if pending == "m" {
        ops.push("Consume");
        state.pending = "";
        if key.len() == 1 { ops.push(#{ SetMark: key }); }
        return;
    }
    if pending == "`" {
        ops.push("Consume");
        state.pending = "";
        if key.len() == 1 { ops.push(#{ GotoMark: key }); }
        return;
    }
    if pending == "'" {
        ops.push("Consume");
        state.pending = "";
        if key.len() == 1 { ops.push(#{ GotoMarkLine: key }); }
        return;
    }
    if pending == "q" {
        ops.push("Consume");
        state.pending = "";
        if key.len() == 1 { ops.push(#{ StartMacro: key }); state.recording = true; }
        return;
    }
    if pending == "@" {
        ops.push("Consume");
        state.pending = "";
        if key.len() == 1 { ops.push(#{ PlayMacro: key }); }
        return;
    }
    if pending == "]" {
        ops.push("Consume");
        state.pending = "";
        if key == "c" { ops.push(#{ RunCommand: "next-hunk" }); }
        return;
    }
    if pending == "[" {
        ops.push("Consume");
        state.pending = "";
        if key == "c" { ops.push(#{ RunCommand: "prev-hunk" }); }
        return;
    }
    if pending == "r" {
        ops.push("Consume");
        state.pending = "";
        if key.len() == 1 {
            ops.push(#{ DeleteForward: 1 });
            ops.push(#{ Insert: key });
            ops.push(#{ Move: -1 });
        }
        return;
    }
    if pending == ">" {
        ops.push("Consume");
        state.pending = "";
        if key == ">" { ops.push("Indent"); }
        return;
    }
    if pending == "<" {
        ops.push("Consume");
        state.pending = "";
        if key == "<" { ops.push("Outdent"); }
        return;
    }

    ops.push("Consume");
    // Count prefix: digits accumulate so 5j, 3l, and 42G repeat or jump. 0 only
    // counts once a count is going, so a bare 0 is still line start.
    let count = if "count" in state { state.count } else { "" };
    state.count = "";
    if (key == "0" && count != "") || (key.len() == 1 && key >= "1" && key <= "9") {
        state.count = count + key;
        return;
    }
    if key == " " { state.pending = "SPC"; ops.push(#{ ShowMenu: leader_menu() }); }
    else if key == ":" { ops.push(#{ RunCommand: "open-palette" }); }
    else if key == "/" { ops.push(#{ RunCommand: "find" }); }
    else if key == "i" { ops.push(#{ SetMode: "insert" }); }
    else if key == "I" { ops.push("SmartLineStart"); ops.push(#{ SetMode: "insert" }); }
    else if key == "a" { ops.push(#{ Move: 1 }); ops.push(#{ SetMode: "insert" }); }
    else if key == "A" { ops.push("LineEnd"); ops.push(#{ SetMode: "insert" }); }
    else if key == "o" { ops.push("LineEnd"); ops.push(#{ Insert: "\n" }); ops.push(#{ SetMode: "insert" }); }
    else if key == "h" || key == "ArrowLeft" { ops.push(#{ Move: -count_or(count, 1) }); }
    else if key == "l" || key == "ArrowRight" { ops.push(#{ Move: count_or(count, 1) }); }
    else if key == "j" || key == "ArrowDown" { ops.push(#{ MoveLine: count_or(count, 1) }); }
    else if key == "k" || key == "ArrowUp" { ops.push(#{ MoveLine: -count_or(count, 1) }); }
    else if key == "0" { ops.push("LineStart"); }
    else if key == "$" { ops.push("LineEnd"); }
    else if key == "w" { ops.push("NextWord"); }
    else if key == "b" { ops.push("PrevWord"); }
    else if key == "e" { ops.push("WordEnd"); }
    else if key == "E" { ops.push("BigWordEnd"); }
    else if key == "W" { ops.push("BigWord"); }
    else if key == "B" { ops.push("BigWordBack"); }
    else if key == "%" { ops.push("MatchPair"); }
    else if key == "~" { ops.push("ToggleCase"); }
    else if key == "H" { ops.push("CaretViewTop"); }
    else if key == "M" { ops.push("CaretViewMiddle"); }
    else if key == "L" { ops.push("CaretViewBottom"); }
    else if key == "m" { state.pending = "m"; }
    else if key == "`" { state.pending = "`"; }
    else if key == "'" { state.pending = "'"; }
    else if key == "f" { state.pending = "f"; }
    else if key == "F" { state.pending = "F"; }
    else if key == "t" { state.pending = "t"; }
    else if key == "T" { state.pending = "T"; }
    else if key == ";" { ops.push("RepeatFind"); }
    else if key == "," { ops.push("RepeatFindBack"); }
    else if key == "n" { ops.push("SearchNext"); }
    else if key == "N" { ops.push("SearchPrev"); }
    else if key == "*" { if word != "" { ops.push(#{ Search: word }); } }
    else if key == "#" { if word != "" { ops.push(#{ SearchBack: word }); } }
    else if key == "g" { state.pending = "g"; }
    else if key == "x" { ops.push(#{ DeleteForward: 1 }); }
    else if key == "X" { ops.push(#{ DeleteBackward: 1 }); }
    else if key == "D" { ops.push("DeleteToLineEnd"); }
    else if key == "C" { ops.push("DeleteToLineEnd"); ops.push(#{ SetMode: "insert" }); }
    else if key == "J" { ops.push("JoinLines"); }
    else if key == "K" { ops.push(#{ RunCommand: "hover" }); }
    else if key == "u" { ops.push(#{ RunCommand: "undo" }); }
    else if key == "d" { state.pending = "d"; state.opcount = count; }
    else if key == "c" { state.pending = "c"; state.opcount = count; }
    else if key == "y" { state.pending = "y"; state.opcount = count; }
    else if key == "p" || key == "P" { ops.push("Paste"); }
    else if key == "\"" { state.pending = "\""; }
    else if key == "." { ops.push("Repeat"); }
    else if key == "G" { if count != "" { ops.push(#{ Goto: count_or(count, 1) }); } else { ops.push("BufferEnd"); } }
    else if key == "z" { state.pending = "z"; }
    else if key == "r" { state.pending = "r"; }
    else if key == ">" { state.pending = ">"; }
    else if key == "<" { state.pending = "<"; }
    else if key == "O" { ops.push("LineStart"); ops.push(#{ Insert: "\n" }); ops.push(#{ Move: -1 }); ops.push(#{ SetMode: "insert" }); }
    else if key == "q" {
        if "recording" in state && state.recording { ops.push("StopMacro"); state.recording = false; }
        else { state.pending = "q"; }
    }
    else if key == "@" { state.pending = "@"; }
    else if key == "]" { state.pending = "]"; }
    else if key == "[" { state.pending = "["; }
    else if key == "v" { ops.push("Anchor"); ops.push(#{ SetMode: "visual" }); }
    else if key == "V" { ops.push("SelectLine"); ops.push(#{ SetMode: "visual" }); }
}
