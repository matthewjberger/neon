//! The language worker. Links only `rhai`. It compile-checks plugin source for
//! syntax errors and flags `commands.x(` calls that are not a known command or
//! standard-library helper, off the render thread. The page seeds it with the
//! vocabulary once, then sends source to check on each pause in typing.

use std::cell::RefCell;
use std::collections::HashSet;

use protocol::{CommandInfo, Diagnostic, LangRequest, LangResponse, MESSAGE_KEY, Severity, StdModule};
use rhai::Engine;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

thread_local! {
    static VOCAB: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

const BUILTINS: &[&str] = &[
    "push", "tag", "last", "log", "print", "len", "clear", "pad", "to_float", "to_int", "abs",
    "sin", "cos", "tan", "sqrt", "floor", "ceil", "round", "min", "max", "random", "random_range",
    "random_int", "entity_ref", "result",
];

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
        for builtin in BUILTINS {
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
    unknown_calls(source)
}

fn unknown_calls(source: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    VOCAB.with(|vocab| {
        let vocab = vocab.borrow();
        if vocab.is_empty() {
            return;
        }
        let bytes = source.as_bytes();
        let needle = b"commands.";
        let mut index = 0;
        while index + needle.len() <= bytes.len() {
            if &bytes[index..index + needle.len()] == needle {
                let start = index + needle.len();
                let mut end = start;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                {
                    end += 1;
                }
                let mut after = end;
                while after < bytes.len() && bytes[after] == b' ' {
                    after += 1;
                }
                if end > start && after < bytes.len() && bytes[after] == b'(' {
                    let name = &source[start..end];
                    if !vocab.contains(name) {
                        let (line, column) = line_col(source, start);
                        out.push(Diagnostic {
                            message: format!("unknown command or helper: commands.{name}"),
                            line,
                            column,
                            severity: Severity::Warning,
                        });
                    }
                }
                index = end;
            } else {
                index += 1;
            }
        }
    });
    out
}

fn line_col(source: &str, index: usize) -> (u32, u32) {
    let mut line = 1_u32;
    let mut column = 1_u32;
    for (offset, character) in source.char_indices() {
        if offset >= index {
            break;
        }
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
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
