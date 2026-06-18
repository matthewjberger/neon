//! The engine worker. Owns the `OffscreenCanvas`, the engine `World`, and the
//! render loop, and runs the scene plugins through `nightshade-api`'s scripting
//! runtime. The page talks to it exclusively through the `protocol` messages.
//!
//! Scene logic is written against the facade: plain data in `state.rs` and free
//! functions in `systems/`. The worker drops to the raw engine only for the
//! parts the facade does not cover from a worker: the renderer, the offscreen
//! driver, and input injection.

mod screenshot;
mod state;
mod stdlib;
mod systems;

use std::cell::RefCell;
use std::rc::Rc;

use nightshade::prelude::*;
use nightshade::render::wgpu::create_wgpu_renderer;
use nightshade_api::prelude::{Command as ApiCommand, submit_command};
use protocol::{
    AgentRequest, AgentResponse, CANVAS_KEY, ClientMessage, MESSAGE_KEY, WorkerMessage,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent, OffscreenCanvas};

use crate::state::Scene;
use crate::systems::manifest;

type AppSlot = Rc<RefCell<Option<App>>>;
type PendingMessages = Rc<RefCell<Vec<JsValue>>>;

struct App {
    world: World,
    renderer: WgpuRenderer,
    state: Scene,
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();

    let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
    let app_slot: AppSlot = Rc::new(RefCell::new(None));
    let pending: PendingMessages = Rc::new(RefCell::new(Vec::new()));

    let handler_scope = scope.clone();
    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        handle_data(&handler_scope, &app_slot, &pending, event.data());
    });
    scope.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
}

fn handle_data(
    scope: &DedicatedWorkerGlobalScope,
    app_slot: &AppSlot,
    pending: &PendingMessages,
    data: JsValue,
) {
    let Ok(payload) = js_sys::Reflect::get(&data, &JsValue::from_str(MESSAGE_KEY)) else {
        return;
    };
    let Ok(message) = serde_wasm_bindgen::from_value::<ClientMessage>(payload) else {
        return;
    };

    if !matches!(message, ClientMessage::Init { .. }) && app_slot.borrow().is_none() {
        pending.borrow_mut().push(data);
        return;
    }

    match message {
        ClientMessage::Init { width, height } => {
            let Some(canvas) = canvas_from(&data) else {
                return;
            };
            let scope = scope.clone();
            let app_slot = app_slot.clone();
            let pending = pending.clone();
            spawn_local(async move {
                screenshot::set_canvas(canvas.clone());
                let app = create_app(canvas, width, height).await;
                *app_slot.borrow_mut() = Some(app);
                let queued = std::mem::take(&mut *pending.borrow_mut());
                for data in queued {
                    handle_data(&scope, &app_slot, &pending, data);
                }
                post(&WorkerMessage::Ready {
                    adapter: "WebGPU".to_string(),
                    commands: manifest::commands(),
                    command_schema: manifest::schema(),
                    stdlib: manifest::modules(),
                });
                start_render_loop(app_slot);
            });
        }
        ClientMessage::Resize { width, height } => {
            if let Some(app) = app_slot.borrow_mut().as_mut() {
                let physical_width = (width as u32).max(1);
                let physical_height = (height as u32).max(1);
                resize_offscreen(
                    &mut app.world,
                    &mut app.renderer,
                    physical_width,
                    physical_height,
                );
                app.world.resources.window.active_viewport_rect =
                    Some(nightshade::ecs::window::resources::ViewportRect {
                        x: 0.0,
                        y: 0.0,
                        width: physical_width as f32,
                        height: physical_height as f32,
                    });
            }
        }
        other => {
            if let Some(app) = app_slot.borrow_mut().as_mut() {
                apply_client_message(&mut app.world, &mut app.state, other);
            }
        }
    }
}

fn apply_client_message(world: &mut World, scene: &mut Scene, message: ClientMessage) {
    match message {
        ClientMessage::PointerMove { x, y } => {
            input_inject_cursor_moved(world, Vec2::new(x, y));
        }
        ClientMessage::PointerButton { button, pressed } => {
            let state = if pressed {
                KeyState::Pressed
            } else {
                KeyState::Released
            };
            input_inject_mouse_button(world, mouse_button(button), state);
        }
        ClientMessage::Wheel { delta } => {
            input_inject_mouse_wheel(world, Vec2::new(0.0, -delta / 100.0));
        }
        ClientMessage::Touch { id, phase, x, y } => {
            input_inject_touch(world, id, touch_phase(phase), Vec2::new(x, y));
        }
        ClientMessage::Key {
            code,
            pressed,
            text,
        } => {
            if let Some(key_code) = key_code_from_dom(&code) {
                let state = if pressed {
                    KeyState::Pressed
                } else {
                    KeyState::Released
                };
                input_inject_keyboard(world, key_code, state, text.as_deref());
            }
        }
        ClientMessage::Pick { x, y } => {
            let detail = systems::picking::pick(scene, world, x, y);
            post(&WorkerMessage::Selected { detail });
        }
        ClientMessage::SetPlugins { plugins } => {
            post(&WorkerMessage::Busy { active: true });
            systems::plugins::set_plugins(scene, world, plugins);
            post(&WorkerMessage::Busy { active: false });
        }
        ClientMessage::SubmitCommand { command } => {
            run_submitted_command(world, &command);
        }
        ClientMessage::ResetScene => {
            post(&WorkerMessage::Busy { active: true });
            systems::plugins::reset(scene, world);
            post(&WorkerMessage::Busy { active: false });
        }
        ClientMessage::SetRunning { running } => {
            systems::plugins::set_running(scene, running);
        }
        ClientMessage::Agent(request) => {
            if let AgentRequest::Screenshot {
                correlation_id,
                max_dimension,
            } = *request
            {
                screenshot::queue(correlation_id, max_dimension);
            } else {
                let response = handle_agent(world, *request);
                post(&WorkerMessage::Agent(Box::new(response)));
            }
        }
        ClientMessage::Init { .. } | ClientMessage::Resize { .. } => {}
    }
}

fn run_submitted_command(world: &mut World, command: &str) {
    match serde_json::from_str::<ApiCommand>(command) {
        Ok(command) => {
            submit_command(world, &command);
        }
        Err(error) => post(&WorkerMessage::PluginError {
            plugin: None,
            message: format!("invalid command: {error}"),
        }),
    }
}

/// The scene-domain half of the agent surface. Editor-domain requests (buffers,
/// panels, plugins) are answered by the page and never reach here.
fn handle_agent(world: &mut World, request: AgentRequest) -> AgentResponse {
    match request {
        AgentRequest::RunCommand {
            correlation_id,
            command,
        } => match serde_json::from_str::<ApiCommand>(&command) {
            Ok(command) => {
                let reply = submit_command(world, &command);
                let value = serde_json::to_value(&reply).unwrap_or(serde_json::Value::Null);
                AgentResponse::Scene {
                    correlation_id,
                    result: value,
                }
            }
            Err(error) => AgentResponse::Error {
                correlation_id,
                message: format!("invalid command: {error}"),
            },
        },
        AgentRequest::QueryScene { correlation_id, .. } => {
            let mut ids = Vec::new();
            world.core.query().iter(|entity, _, _| ids.push(entity.id));
            AgentResponse::Scene {
                correlation_id,
                result: serde_json::json!({ "entities": ids }),
            }
        }
        other => AgentResponse::Error {
            correlation_id: protocol::request_correlation(&other),
            message: "request is editor-domain, not scene-domain".to_string(),
        },
    }
}

async fn create_app(canvas: OffscreenCanvas, width: f32, height: f32) -> App {
    let physical_width = (width as u32).max(1);
    let physical_height = (height as u32).max(1);

    let surface_target = wgpu::SurfaceTarget::OffscreenCanvas(canvas);
    let mut renderer = create_wgpu_renderer(surface_target, physical_width, physical_height)
        .await
        .expect("failed to create renderer from offscreen canvas");

    let mut world = World::default();
    let mut state = Scene::new();
    initialize_offscreen(
        &mut world,
        &mut state,
        &mut renderer,
        (physical_width, physical_height),
        1.0,
    );
    world.resources.window.active_viewport_rect =
        Some(nightshade::ecs::window::resources::ViewportRect {
            x: 0.0,
            y: 0.0,
            width: physical_width as f32,
            height: physical_height as f32,
        });

    App {
        world,
        renderer,
        state,
    }
}

fn start_render_loop(app_slot: AppSlot) {
    let last_stats = Rc::new(RefCell::new(0.0_f64));

    spawn_animation_frame_loop(move || {
        if let Some(app) = app_slot.borrow_mut().as_mut() {
            tick_offscreen(&mut app.world, &mut app.state, &mut app.renderer);
            screenshot::flush();

            if !app.state.log.is_empty() {
                let entries = std::mem::take(&mut app.state.log);
                post(&WorkerMessage::Report { entries });
            }

            let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
            if let Some(performance) = scope.performance() {
                let now = performance.now();
                let mut last = last_stats.borrow_mut();
                if now - *last > 500.0 {
                    *last = now;
                    let entity_count = app
                        .world
                        .core
                        .query_entities(
                            nightshade::ecs::world::LOCAL_TRANSFORM
                                | nightshade::ecs::world::GLOBAL_TRANSFORM,
                        )
                        .count() as u32;
                    post(&WorkerMessage::Stats {
                        fps: app.world.resources.window.timing.frames_per_second,
                        entity_count,
                    });
                }
            }
        }
    });
}

fn mouse_button(button: u8) -> MouseButton {
    match button {
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        _ => MouseButton::Left,
    }
}

fn touch_phase(phase: protocol::TouchPhase) -> TouchPhase {
    match phase {
        protocol::TouchPhase::Started => TouchPhase::Started,
        protocol::TouchPhase::Moved => TouchPhase::Moved,
        protocol::TouchPhase::Ended => TouchPhase::Ended,
        protocol::TouchPhase::Cancelled => TouchPhase::Cancelled,
    }
}

fn key_code_from_dom(code: &str) -> Option<KeyCode> {
    Some(match code {
        "KeyA" => KeyCode::KeyA,
        "KeyB" => KeyCode::KeyB,
        "KeyC" => KeyCode::KeyC,
        "KeyD" => KeyCode::KeyD,
        "KeyE" => KeyCode::KeyE,
        "KeyF" => KeyCode::KeyF,
        "KeyG" => KeyCode::KeyG,
        "KeyH" => KeyCode::KeyH,
        "KeyI" => KeyCode::KeyI,
        "KeyJ" => KeyCode::KeyJ,
        "KeyK" => KeyCode::KeyK,
        "KeyL" => KeyCode::KeyL,
        "KeyM" => KeyCode::KeyM,
        "KeyN" => KeyCode::KeyN,
        "KeyO" => KeyCode::KeyO,
        "KeyP" => KeyCode::KeyP,
        "KeyQ" => KeyCode::KeyQ,
        "KeyR" => KeyCode::KeyR,
        "KeyS" => KeyCode::KeyS,
        "KeyT" => KeyCode::KeyT,
        "KeyU" => KeyCode::KeyU,
        "KeyV" => KeyCode::KeyV,
        "KeyW" => KeyCode::KeyW,
        "KeyX" => KeyCode::KeyX,
        "KeyY" => KeyCode::KeyY,
        "KeyZ" => KeyCode::KeyZ,
        "Digit0" => KeyCode::Digit0,
        "Digit1" => KeyCode::Digit1,
        "Digit2" => KeyCode::Digit2,
        "Digit3" => KeyCode::Digit3,
        "Digit4" => KeyCode::Digit4,
        "Digit5" => KeyCode::Digit5,
        "Digit6" => KeyCode::Digit6,
        "Digit7" => KeyCode::Digit7,
        "Digit8" => KeyCode::Digit8,
        "Digit9" => KeyCode::Digit9,
        "Escape" => KeyCode::Escape,
        "Enter" => KeyCode::Enter,
        "NumpadEnter" => KeyCode::NumpadEnter,
        "Tab" => KeyCode::Tab,
        "Space" => KeyCode::Space,
        "Delete" => KeyCode::Delete,
        "Backspace" => KeyCode::Backspace,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        "ArrowUp" => KeyCode::ArrowUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "ShiftLeft" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "ControlLeft" => KeyCode::ControlLeft,
        "ControlRight" => KeyCode::ControlRight,
        "AltLeft" => KeyCode::AltLeft,
        "AltRight" => KeyCode::AltRight,
        "F1" => KeyCode::F1,
        "F2" => KeyCode::F2,
        "F3" => KeyCode::F3,
        "F4" => KeyCode::F4,
        "F5" => KeyCode::F5,
        "F6" => KeyCode::F6,
        "F7" => KeyCode::F7,
        "F8" => KeyCode::F8,
        "F9" => KeyCode::F9,
        "F10" => KeyCode::F10,
        "F11" => KeyCode::F11,
        "F12" => KeyCode::F12,
        "Comma" => KeyCode::Comma,
        "Period" => KeyCode::Period,
        "Minus" => KeyCode::Minus,
        "Equal" => KeyCode::Equal,
        _ => return None,
    })
}

fn canvas_from(data: &JsValue) -> Option<OffscreenCanvas> {
    js_sys::Reflect::get(data, &JsValue::from_str(CANVAS_KEY))
        .ok()
        .and_then(|value| value.dyn_into::<OffscreenCanvas>().ok())
}

pub(crate) fn post(message: &WorkerMessage) {
    let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
    if let Ok(value) = serde_wasm_bindgen::to_value(message) {
        let envelope = js_sys::Object::new();
        if js_sys::Reflect::set(&envelope, &JsValue::from_str(MESSAGE_KEY), &value).is_ok() {
            drop(scope.post_message(&envelope));
        }
    }
}
