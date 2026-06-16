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

fn check(source: &str) -> Vec<Diagnostic> {
    let engine = Engine::new();
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
