# Enabling rust-analyzer

For a Rust file, neon is an LSP client to rust-analyzer. Starting it needs only
`rustup` and `cargo`, like VS Code, because neon discovers the server through
`rustup`.

## The consent prompt

Starting rust-analyzer spawns a process, so neon asks first. The first time you
open a Rust file in a folder, a consent toast appears: "Start rust-analyzer for
<folder>?". Allow it once and the server starts for the session. Dismiss it and
nothing spawns.

## Discovery

The desktop language-server bridge finds the server by running
`rustup which rust-analyzer`. If the component is missing it runs
`rustup component add rust-analyzer` and tries again, then falls back to a
`rust-analyzer` on the path. So a standard Rust install just works, with no
separate download.

## Where it runs

The LSP bridge lives in the desktop shell, so rust-analyzer support comes with
the desktop build (`just run`).

## What you get

Once it is running you have diagnostics, completion, hover, signature help, and
the navigation and refactoring features. The next chapters cover them:
[Diagnostics, Completion, Hover](lsp-features.md) and
[Navigation and Refactoring](lsp-navigation.md).
