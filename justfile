set windows-shell := ["powershell.exe"]
export RUST_BACKTRACE := "1"

# Displays the list of available commands
@just:
    just --list

# Installs the tools pinned in mise.toml (rust, wasm-bindgen, wasm-opt, trunk)
init:
    mise install

# Builds the engine worker crate to wasm and generates web bindings into runtime/
engine:
    cargo build --release -p worker --target wasm32-unknown-unknown
    wasm-bindgen --target web --out-dir runtime --out-name engine target/wasm32-unknown-unknown/release/worker.wasm
    wasm-opt -O3 --enable-simd runtime/engine_bg.wasm -o runtime/engine_bg.wasm

# Builds the language worker crate to wasm and generates web bindings into runtime/
lang:
    cargo build --release -p lang --target wasm32-unknown-unknown
    wasm-bindgen --target web --out-dir runtime --out-name lang target/wasm32-unknown-unknown/release/lang.wasm
    wasm-opt -O3 runtime/lang_bg.wasm -o runtime/lang_bg.wasm

# Builds both workers
workers: engine lang

# Builds the workers and the Leptos app bundle
build: workers
    trunk build

# Builds the web bundle and opens the app in a native webview window
run: build
    cargo run -p desktop

# Builds the workers, then serves the app in the browser at http://127.0.0.1:8080
run-web: workers
    trunk serve

# Serves the already-built app without rebuilding the workers
serve:
    trunk serve

# Produces a production web bundle in dist
dist: workers
    trunk build --release

# Builds the standalone executable with the web bundle embedded
build-desktop: dist
    cargo build --release -p desktop

# Runs the language-worker tests
test:
    cargo test -p lang

# Runs cargo check, the tests, and a format check across the workspace
check: test
    cargo check -p protocol -p worker -p lang -p neon --target wasm32-unknown-unknown
    cargo check -p desktop
    cargo fmt --all -- --check

# Runs clippy across the workspace and denies warnings
lint:
    cargo clippy -p protocol -p worker -p lang -p neon --target wasm32-unknown-unknown -- -D warnings
    cargo clippy -p desktop -- -D warnings

# Formats the code
format:
    cargo fmt --all

# Removes build artifacts (Windows)
[windows]
clean:
    cargo clean
    Remove-Item -Recurse -Force dist -ErrorAction SilentlyContinue
    Remove-Item -Force runtime/engine.js, runtime/engine_bg.wasm, runtime/engine.d.ts, runtime/engine_bg.wasm.d.ts -ErrorAction SilentlyContinue
    Remove-Item -Force runtime/lang.js, runtime/lang_bg.wasm, runtime/lang.d.ts, runtime/lang_bg.wasm.d.ts -ErrorAction SilentlyContinue

# Removes build artifacts (Unix)
[unix]
clean:
    cargo clean
    rm -rf dist runtime/engine.js runtime/engine_bg.wasm runtime/engine.d.ts runtime/engine_bg.wasm.d.ts runtime/lang.js runtime/lang_bg.wasm runtime/lang.d.ts runtime/lang_bg.wasm.d.ts
