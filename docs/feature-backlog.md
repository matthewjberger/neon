# Neon roadmap

Vision-led, not a parity checklist. The constraints that shape this: no users,
single author, full reign, no effort too large. That means we optimize for the
right foundation and a distinct identity, not for catching up to VSCode on
chrome. A raw per-editor gap list (vim, neovim, emacs, kakoune, helix, vscode,
zed) is preserved as an appendix; everything above it is reorganized around what
neon should *be*.

Effort tags: `[P]` plugin-only on existing primitives, `[H]` small host
primitive then plugin work, `[B]` big lift / new subsystem.

## What neon is

A Rust, no-npm code editor whose power comes from plugins over a stable core
(like Emacs), with three things no mainstream editor has together:

1. A **reflective rhai plugin system** — editor and scene plugins share
   authoring, source is visible and hot-editable in-app, the API is derived from
   the engine manifest.
2. **Deep agent integration** — the MCP bridge already lets Claude read/write
   buffers, run commands, and screenshot the viewport.
3. A **live nightshade 3D view** — scene plugins drive a real-time engine; the
   editor exists to author them.

We lean into those. We do not try to out-VSCode VSCode.

## Already shipped (do not re-build)

LSP (completion, hover, signature, goto/type/impl, references, symbols, rename,
code actions, formatting, diagnostics, organize imports, didSave, progress),
tree-sitter highlighting (native bridge), panes/splits/tabs, multi-cursor (faked
over the textarea), avy jumps, find/replace, project search, terminal,
Claude/MCP, theming, palette, which-key leader, session restore, plugin system,
multi-window (hearsay). Modal layer: counts, dot-repeat, gg/G, f/F/t/T with ;/,,
*/#/n/N, visual mode, text objects (word/quote/bracket), surround, kill-ring +
named registers, region indent/comment/case.

---

## I. Foundations — the three rewrites everything depends on

Most "features" in the appendix are downstream of these. Do them first.

### 1. Rope buffer core `[B]`
The buffer is `String` / `Vec<char>`, re-collected on every keystroke
(`editor_plugins.rs` splices a fresh `Vec<char>` and rejoins each edit) — O(n)
per edit plus a full re-highlight. Replace with a rope (`ropey`). Prerequisite
for large files, clean edit primitives, and incremental tree-sitter.

### 2. Custom-rendered editing surface `[B]`
The `<textarea>` + `<pre>` overlay is the ceiling. It blocks, all at once:
inlay hints, semantic virtual text, real multi-cursor (today's is offsets
painted over a textarea), block / column selection, inline diff gutters, sticky
scroll, soft-wrap, minimap, ligatures. A custom view (DOM spans or canvas with a
virtual caret, IME, and accessibility handled deliberately) unlocks every one of
them. This is DESIGN.md milestone 5 and the single highest-leverage move.

### 3. Selection-first core `[B]`
Make **multiple selections the core primitive** (the `MARK` + `cursors` state
already gesture at it). Vim is expressible on top of selections — that is how
Helix works — so the vim/spacemacs layer becomes a plugin over the selection
model instead of a parallel paradigm. Unifies multi-cursor, visual mode, and the
kakoune/helix track into one foundation.

---

## II. Identity pillars — what makes neon neon

### A. AI-native editing `[B]`
The MCP bridge exists; build editing *modalities* on it, not just a chat pane.
- inline agent edits in the buffer (select → transform, ghost-text diffs to accept)
- agent-driven multi-file refactors surfaced as a reviewable change set
- AI-aware multibuffer: the agent gathers call sites / matches into one editable buffer
- the agent reads diagnostics, the scene, and the console it already has access to

### B. Live / creative coding with the 3D engine `[B]`
Nothing else edits code that drives a live 3D scene. Serve that.
- live-reload of scene values without a full reset; hot-tunable parameters
- inline visualization of expressions (color swatches, vectors, curves)
- scene/shader editing affordances; scrub time; pin and watch values
- a "creative coding" identity: the editor and the thing it makes are one screen

### C. Plugin-API depth `[H]/[B]`
The reflective rhai model is a moat. Make features *be* plugins.
- plugins that add panels, draw overlays, define commands, hook editor events
- richer op/host surface (the P1–P5 work is the start); document and stabilize it
- a plugin can ship its own which-key tree, status segment, and gutter marks

---

## III. Table-stakes (parity, after foundations)

Universal and worth doing — but most are cheaper or only possible once I/II land.

### LSP depth (capabilities already advertised)
- `[H]` inlay hints, semantic tokens, code lens, call/type hierarchy
- `[H]` outline / breadcrumbs, folding ranges, peek definition/references
- `[H]` expand/shrink selection (`selectionRange`), `gq` range formatting
- `[B]` **multi-language servers** — currently rust-analyzer only; auto-discover per language

### Tree-sitter structural editing `[B]`
- structural selection: function / class / parameter / comment objects (`mi f`/`ma f`)
- structural motion: move by sibling / parent / child node
- tree-sitter folds

### Vim completeness (mostly cheap, do early in parallel)
- `[P]` operators + motions: `dt)` `df,` `dw` `d$` `dG` `d%` `c/y{motion}` (Anchor→motion→Cut)
- `[P]` `O` open above, `>>`/`<<` normal indent, `r{char}`, `gv`
- `[H]` `~`/`g~`/`gu`/`gU` case ops; `%` match pair; `e`/`E`/`W`/`B`/`ge`; `ip`/`ap`/`it`/`at`/`is`/`as`
- `[B]` operator-counts (`3dd`), faithful dot (capture insert typing), `:` ex / `:s`

### Selection model extras (free once III/selection-first lands)
- `[H]` select-by-regex within selection, split, keep/drop matching
- `[B]` pipe selection through shell (`|`), insert command output (`!`)
- `[H]` align / rotate selections

### Editing power & navigation
- `[B]` macros (record/replay), `[B]` undo tree, `[B]` snippets (tabstops)
- `[H]` marks/bookmarks, mark ring, `[B]` jumplist/changelist
- `[B]` multibuffer / editable grep results (overlaps pillar A)
- `[H]` block/rectangle editing (free once selection-first + custom surface land)

### Git `[B]`
- gutter signs, blame, diff view, hunk stage/revert, stage/commit, next/prev hunk

### UX (mostly downstream of the custom surface)
- `[H]` minimap, sticky scroll, indent guides, bracket-pair colorization
- `[H]` soft-wrap, render-whitespace, relative line numbers, zen mode

---

## IV. Cut / defer (do not spend milestones here)

- **Real-time collaboration / Live Share** — multiplayer for zero players. Cut until there's a reason.
- **Remote editing (SSH / containers / TRAMP)** — solving a problem we do not have.
- **Debugger (DAP)** — enormous, table-stakes-not-differentiating, low solo ROI. Defer hard.
- **Settings UI, Emmet, spell check** — low leverage; revisit only if they get cheap.
- **Minimap as a priority** — fine as a downstream freebie, never a milestone.

---

## Recommended order

1. Rope buffer core
2. Custom-rendered editing surface
3. Selection-first core (vim/multicursor become layers)
4. AI-native editing (inline edits + multibuffer on the MCP bridge)
5. LSP depth + multi-server
6. Tree-sitter structural editing
7. Vim operators+motions `[P]` — slot in early, it is cheap and closes the obvious gap
8. Git, macros, snippets, undo tree, marks
9. Live-coding / 3D-editor features — the identity wildcard, dedicated exploration

---

## Appendix: per-editor gap list (raw input)

Kept for traceability. Items above subsume these.

- **Vim**: operators+motions; `O`/`>>`/`r`/`gv`; case ops; `%`; word/WORD motions;
  more text objects; operator-counts; faithful dot; visual block; `:`/`:s`; macros.
- **Neovim**: jumplist/changelist; folding; tree-sitter text objects; `gq`; diff mode; snippets.
- **Emacs**: kmacro; rectangle editing; undo tree; mark ring; expand-region;
  narrowing; bookmarks; TRAMP; editable occur/grep.
- **Kakoune**: multiple-selections-as-core; select-by-regex/split; pipe-to-shell;
  align/rotate selections; search-from-selection.
- **Helix**: tree-sitter structural selection/motion; selection-first cursors;
  macros; soft-wrap; inline diagnostics; pickers.
- **VSCode**: git SCM; debugger (DAP); inlay hints/code lens/hierarchy; peek;
  breadcrumbs/outline; minimap/sticky-scroll/indent-guides/bracket-colors;
  snippets/Emmet; remote dev; settings UI.
- **Zed**: real-time collaboration; multibuffer; git gutter/blame; inline AI edits;
  outline/project panel; tasks.
