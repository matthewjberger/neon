# Glossary

**Context.** One of the four isolated execution environments: the page, the
engine worker, the language worker, and the desktop shell.

**Page.** The main-thread Leptos UI, the `neon` crate. Owns the editor state and
the user-facing surfaces.

**Engine worker.** The web worker that runs nightshade through the
`nightshade-api` facade and renders the viewport offscreen.

**Language worker.** The web worker that compile-checks rhai scene plugins.

**Desktop shell.** The `wry` webview that serves the bundle and hosts the
filesystem, language-server, and Claude bridges.

**Protocol.** The serde crate that defines every cross-context message.

**Scene plugin.** A rhai plugin that scripts the 3D view through engine commands.

**Editor plugin.** A rhai plugin that extends the editor by handling keystrokes
and pushing ops.

**Op.** An action an editor plugin pushes for the host to apply, such as a caret
move, an edit, or a `RunCommand`.

**Command.** Either an engine `Command` a scene plugin pushes, or an entry in the
editor command registry an editor action invokes. Context makes which clear.

**Leader.** The `SPC` key and the which-key menu tree it opens.

**Which-key menu.** The bottom panel that lists the next keys in a leader
sequence and what they do.

**Manifest.** The reflected vocabulary of scene commands and standard-library
helpers the worker sends the page, which drives highlighting, completion, the
reference, and validation.

**Standard library.** The rhai helpers prepended to every scene plugin
(`worker/stdlib/`).

**Overlay sync.** Sending the editor's unsaved buffer text to rust-analyzer with
`didChange`, so the server sees what is on screen.
