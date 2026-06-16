# Troubleshooting

## The first build takes a while

The first build compiles nightshade from source, so it runs longer than later
builds. After that, builds are incremental and fast. Let the first one finish and
subsequent runs are quick.

## WebGPU

Neon renders through WebGPU in a worker. Run it in a current Chromium or WebKit
(the desktop shell uses the system webview, so a recent OS webview is enough). If
the viewport does not appear, update the browser or platform webview to a version
with WebGPU and OffscreenCanvas-in-workers support.

## Starting rust-analyzer

rust-analyzer starts after you allow the consent prompt that appears when you
first open a Rust file. It is discovered through `rustup`, so a standard Rust
install works out of the box. If the prompt does not appear, open a `.rs` file
inside an opened folder. The rust-analyzer log panel (`SPC e l`) shows the
server's output if you want to watch it come up.

## Completion reflects on-screen edits

Open files sync to rust-analyzer with the overlay model, so completion, hover,
and diagnostics track your unsaved edits. If results ever feel stale, the
rust-analyzer log shows the live traffic.

## The full editor

The desktop build (`just run`) brings rust-analyzer, the file system, project
search, and Claude through the desktop shell. Use it for everything beyond quick
editor-UI iteration.
