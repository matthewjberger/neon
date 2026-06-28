//! The language worker. Links only `rhai`. It compile-checks plugin source for
//! syntax errors and flags `commands.x(` calls that are not a known command or
//! standard-library helper, off the render thread. The page seeds it with the
//! vocabulary once, then sends source to check on each pause in typing.

use std::cell::RefCell;
use std::collections::HashSet;

use protocol::{
    CommandInfo, Diagnostic, LangRequest, LangResponse, MESSAGE_KEY, RHAI_BUILTINS, Severity,
    StdModule, unknown_command_calls,
};
use rhai::Engine;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

thread_local! {
    static VOCAB: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        handle(event.data());
    });
    scope.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
    post(&LangResponse::Ready);
}

fn handle(data: JsValue) {
    let Ok(payload) = js_sys::Reflect::get(&data, &JsValue::from_str(MESSAGE_KEY)) else {
        return;
    };
    let Ok(request) = serde_wasm_bindgen::from_value::<LangRequest>(payload) else {
        return;
    };
    match request {
        LangRequest::Init { commands, stdlib } => seed(&commands, &stdlib),
        LangRequest::Check { request_id, source } => {
            let diagnostics = check(&source);
            post(&LangResponse::Diagnostics {
                request_id,
                diagnostics,
            });
        }
    }
}

fn seed(commands: &[CommandInfo], stdlib: &[StdModule]) {
    VOCAB.with(|vocab| {
        let mut set = vocab.borrow_mut();
        set.clear();
        for command in commands {
            set.insert(command.method.clone());
        }
        for module in stdlib {
            for helper in &module.helpers {
                set.insert(helper.name.clone());
            }
        }
        for builtin in RHAI_BUILTINS {
            set.insert((*builtin).to_string());
        }
    });
}

/// A rhai engine matching the runtime's lifted limits, so a valid but complex
/// script (a long key dispatch, deep loops) is not falsely flagged.
fn make_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(0, 0);
    engine.set_max_operations(0);
    engine
}

fn check(source: &str) -> Vec<Diagnostic> {
    let engine = make_engine();
    if let Err(error) = engine.compile(source) {
        let position = error.position();
        return vec![Diagnostic {
            message: error.to_string(),
            line: position.line().unwrap_or(0) as u32,
            column: position.position().unwrap_or(0) as u32,
            severity: Severity::Error,
        }];
    }
    VOCAB.with(|vocab| unknown_command_calls(source, &vocab.borrow()))
}

fn post(response: &LangResponse) {
    let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
    if let Ok(value) = serde_wasm_bindgen::to_value(response) {
        let envelope = js_sys::Object::new();
        if js_sys::Reflect::set(&envelope, &JsValue::from_str(MESSAGE_KEY), &value).is_ok() {
            drop(scope.post_message(&envelope));
        }
    }
}

#[cfg(test)]
mod tests {
    use rhai::Engine;

    const STDLIB: &[&str] = &[
        include_str!("../../worker/stdlib/shapes.rhai"),
        include_str!("../../worker/stdlib/color.rhai"),
        include_str!("../../worker/stdlib/motion.rhai"),
        include_str!("../../worker/stdlib/events.rhai"),
        include_str!("../../worker/stdlib/input.rhai"),
        include_str!("../../worker/stdlib/random.rhai"),
    ];

    #[test]
    fn prelude_compiles() {
        let prelude = STDLIB.join("\n\n");
        let engine = super::make_engine();
        if let Err(error) = engine.compile(&prelude) {
            panic!("prelude does not compile: {error}");
        }
    }

    const VIM: &str = include_str!("../../editor_stdlib/vim.rhai");
    const EDITOR_TEMPLATE: &str = include_str!("../../editor_stdlib/editor_template.rhai");
    const SPACEMACS: &str = include_str!("../../editor_stdlib/spacemacs.rhai");

    fn run_key(
        engine: &Engine,
        ast: &rhai::AST,
        state_map: &mut rhai::Map,
        key: &str,
    ) -> rhai::Array {
        let mut scope = rhai::Scope::new();
        scope.push("key", key.to_string());
        scope.push("mode", "normal".to_string());
        scope.push("ctrl", false);
        scope.push("shift", false);
        scope.push("alt", false);
        scope.push("ops", rhai::Array::new());
        scope.push("state", state_map.clone());
        engine
            .call_fn::<()>(&mut scope, ast, "on_key", ())
            .expect("on_key failed");
        if let Some(updated) = scope.get_value::<rhai::Map>("state") {
            *state_map = updated;
        }
        scope.get_value::<rhai::Array>("ops").unwrap_or_default()
    }

    fn runs_command(ops: &rhai::Array, id: &str) -> bool {
        ops.iter().any(|op| {
            op.clone()
                .try_cast::<rhai::Map>()
                .and_then(|map| map.get("RunCommand").cloned())
                .and_then(|value| value.into_string().ok())
                .map(|command| command == id)
                .unwrap_or(false)
        })
    }

    fn menu_title(ops: &rhai::Array) -> Option<String> {
        ops.iter().find_map(|op| {
            op.clone()
                .try_cast::<rhai::Map>()
                .and_then(|map| map.get("ShowMenu").cloned())
                .and_then(|value| value.try_cast::<rhai::Map>())
                .and_then(|menu| menu.get("title").cloned())
                .and_then(|title| title.into_string().ok())
        })
    }

    #[test]
    fn spacemacs_compiles() {
        let engine = super::make_engine();
        if let Err(error) = engine.compile(SPACEMACS) {
            panic!("spacemacs does not compile: {error}");
        }
    }

    #[test]
    fn spacemacs_leader_splits_window() {
        let engine = super::make_engine();
        let ast = engine.compile(SPACEMACS).unwrap();
        let mut state_map = rhai::Map::new();
        run_key(&engine, &ast, &mut state_map, " ");
        run_key(&engine, &ast, &mut state_map, "w");
        let ops = run_key(&engine, &ast, &mut state_map, "v");
        assert!(
            runs_command(&ops, "split-right"),
            "SPC w v did not split right"
        );
    }

    #[test]
    fn spacemacs_leader_opens_menu() {
        let engine = super::make_engine();
        let ast = engine.compile(SPACEMACS).unwrap();
        let mut state_map = rhai::Map::new();
        let ops = run_key(&engine, &ast, &mut state_map, " ");
        assert_eq!(
            menu_title(&ops).as_deref(),
            Some("Leader"),
            "SPC did not open the leader menu"
        );
        let ops = run_key(&engine, &ast, &mut state_map, "w");
        assert_eq!(
            menu_title(&ops).as_deref(),
            Some("+Windows"),
            "SPC w did not open the window menu"
        );
    }

    #[test]
    fn spacemacs_leader_opens_palette() {
        let engine = super::make_engine();
        let ast = engine.compile(SPACEMACS).unwrap();
        let mut state_map = rhai::Map::new();
        run_key(&engine, &ast, &mut state_map, " ");
        let ops = run_key(&engine, &ast, &mut state_map, " ");
        assert!(
            runs_command(&ops, "open-palette"),
            "SPC SPC did not open the palette"
        );
    }

    #[test]
    fn catalog_plugins_compile() {
        let sources: &[(&str, &str)] = &[
            (
                "auto_pairs",
                include_str!("../../editor_stdlib/auto_pairs.rhai"),
            ),
            (
                "better_escape",
                include_str!("../../editor_stdlib/better_escape.rhai"),
            ),
            (
                "line_comment",
                include_str!("../../editor_stdlib/line_comment.rhai"),
            ),
            (
                "line_tools",
                include_str!("../../editor_stdlib/line_tools.rhai"),
            ),
            (
                "word_motions",
                include_str!("../../editor_stdlib/word_motions.rhai"),
            ),
            (
                "join_lines",
                include_str!("../../editor_stdlib/join_lines.rhai"),
            ),
            (
                "smart_home",
                include_str!("../../editor_stdlib/smart_home.rhai"),
            ),
            (
                "jump_to_char",
                include_str!("../../editor_stdlib/jump_to_char.rhai"),
            ),
            (
                "blank_lines",
                include_str!("../../editor_stdlib/blank_lines.rhai"),
            ),
            (
                "commentary",
                include_str!("../../editor_stdlib/commentary.rhai"),
            ),
            (
                "move_lines",
                include_str!("../../editor_stdlib/move_lines.rhai"),
            ),
            ("wave_grid", include_str!("../../examples/wave_grid.rhai")),
            ("orbits", include_str!("../../examples/orbits.rhai")),
            ("spiral", include_str!("../../examples/spiral.rhai")),
            ("lissajous", include_str!("../../examples/lissajous.rhai")),
            ("starfield", include_str!("../../examples/starfield.rhai")),
            ("helix", include_str!("../../examples/helix.rhai")),
            (
                "bouncing_balls",
                include_str!("../../examples/bouncing_balls.rhai"),
            ),
            ("snowfall", include_str!("../../examples/snowfall.rhai")),
            (
                "breathing_sphere",
                include_str!("../../examples/breathing_sphere.rhai"),
            ),
            ("fireworks", include_str!("../../examples/fireworks.rhai")),
        ];
        let engine = super::make_engine();
        for (name, source) in sources {
            if let Err(error) = engine.compile(source) {
                panic!("{name} does not compile: {error}");
            }
        }
    }

    #[test]
    fn editor_plugins_compile() {
        let engine = super::make_engine();
        if let Err(error) = engine.compile(VIM) {
            panic!("vim does not compile: {error}");
        }
        if let Err(error) = engine.compile(EDITOR_TEMPLATE) {
            panic!("editor template does not compile: {error}");
        }
    }

    #[test]
    fn vim_normal_i_enters_insert() {
        use rhai::{Array, Map, Scope};
        let engine = super::make_engine();
        let ast = engine.compile(VIM).unwrap();
        let mut scope = Scope::new();
        scope.push("key", "i".to_string());
        scope.push("mode", "normal".to_string());
        scope.push("ctrl", false);
        scope.push("shift", false);
        scope.push("alt", false);
        scope.push("ops", Array::new());
        scope.push("state", Map::new());
        if let Err(error) = engine.call_fn::<()>(&mut scope, &ast, "on_key", ()) {
            panic!("vim on_key failed: {error}");
        }
        let ops = scope.get_value::<Array>("ops").unwrap();
        assert!(ops.len() >= 2, "vim normal 'i' produced {} ops", ops.len());
    }

    #[test]
    fn this_method_helper() {
        use rhai::{Array, Scope};
        let engine = super::make_engine();
        let ast = engine
            .compile("fn cube(value) { this.push(value); }\nfn on_start() { commands.cube(7); commands.cube(8); }")
            .unwrap();
        let mut scope = Scope::new();
        scope.push("commands", Array::new());
        if let Err(error) = engine.call_fn::<()>(&mut scope, &ast, "on_start", ()) {
            panic!("on_start failed: {error}");
        }
        let commands = scope.get_value::<Array>("commands").unwrap();
        assert_eq!(
            commands.len(),
            2,
            "this-method helper produced {} commands",
            commands.len()
        );
    }

    #[test]
    fn call_fn_sees_scope() {
        let engine = super::make_engine();
        let ast = engine.compile("fn handler() { ops.push(42); }").unwrap();
        let mut scope = rhai::Scope::new();
        scope.push("ops", rhai::Array::new());
        if let Err(error) = engine.call_fn::<()>(&mut scope, &ast, "handler", ()) {
            panic!("call_fn error: {error}");
        }
        let ops = scope.get_value::<rhai::Array>("ops").unwrap();
        assert_eq!(ops.len(), 1, "function could not reach the scope's ops");
    }
}
