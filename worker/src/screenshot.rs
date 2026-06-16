//! Viewport capture for the agent. A WebGPU `OffscreenCanvas` is only readable
//! right after the frame that drew it, before the next present clears it, so a
//! request queues here and the capture fires from the render loop immediately
//! after `tick_offscreen`. The canvas is read back through `convert_to_blob`,
//! the browser's PNG encoder, then base64 via a `FileReaderSync` data url. No
//! wgpu readback is involved.

use std::cell::RefCell;

use protocol::{AgentResponse, CorrelationId, WorkerMessage};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Blob, FileReaderSync, OffscreenCanvas, OffscreenCanvasRenderingContext2d};

use crate::post;

struct Pending {
    correlation_id: CorrelationId,
    max_dimension: Option<u32>,
}

thread_local! {
    static CANVAS: RefCell<Option<OffscreenCanvas>> = const { RefCell::new(None) };
    static PENDING: RefCell<Vec<Pending>> = const { RefCell::new(Vec::new()) };
}

/// Keeps a handle to the render canvas so a capture can read it back. This is the
/// same underlying JS object the renderer draws to.
pub fn set_canvas(canvas: OffscreenCanvas) {
    CANVAS.with(|slot| *slot.borrow_mut() = Some(canvas));
}

/// Queues a capture for the next post-render flush.
pub fn queue(correlation_id: CorrelationId, max_dimension: Option<u32>) {
    PENDING.with(|pending| {
        pending.borrow_mut().push(Pending {
            correlation_id,
            max_dimension,
        });
    });
}

/// Captures every queued screenshot. Called from the render loop right after the
/// frame is submitted, in the same task, so the canvas still holds it.
pub fn flush() {
    let ready: Vec<Pending> = PENDING.with(|pending| std::mem::take(&mut *pending.borrow_mut()));
    for request in ready {
        capture(request.correlation_id, request.max_dimension);
    }
}

fn capture(correlation_id: CorrelationId, max_dimension: Option<u32>) {
    let Some(canvas) = CANVAS.with(|slot| slot.borrow().clone()) else {
        fail(correlation_id, "the render canvas is not available");
        return;
    };
    let (promise, width, height) = match begin_capture(&canvas, max_dimension) {
        Ok(begun) => begun,
        Err(error) => {
            fail(correlation_id, &error);
            return;
        }
    };
    spawn_local(async move {
        match finish_capture(promise).await {
            Ok(png_base64) => {
                post(&WorkerMessage::Agent(Box::new(AgentResponse::Screenshot {
                    correlation_id,
                    width,
                    height,
                    png_base64,
                })));
            }
            Err(error) => fail(correlation_id, &error),
        }
    });
}

fn begin_capture(
    canvas: &OffscreenCanvas,
    max_dimension: Option<u32>,
) -> Result<(js_sys::Promise, u32, u32), String> {
    let source_width = canvas.width();
    let source_height = canvas.height();
    if source_width == 0 || source_height == 0 {
        return Err("the render canvas has zero size".to_string());
    }
    let longer = source_width.max(source_height);
    let (target, width, height) = match max_dimension {
        Some(limit) if limit > 0 && longer > limit => {
            let scale = limit as f64 / longer as f64;
            let width = ((source_width as f64 * scale).round() as u32).max(1);
            let height = ((source_height as f64 * scale).round() as u32).max(1);
            let scaled = OffscreenCanvas::new(width, height)
                .map_err(|_| "failed to create the scaling canvas".to_string())?;
            let context = scaled
                .get_context("2d")
                .ok()
                .flatten()
                .and_then(|object| object.dyn_into::<OffscreenCanvasRenderingContext2d>().ok())
                .ok_or_else(|| "failed to get a 2d context".to_string())?;
            context
                .draw_image_with_offscreen_canvas_and_dw_and_dh(
                    canvas,
                    0.0,
                    0.0,
                    width as f64,
                    height as f64,
                )
                .map_err(|_| "failed to downscale the capture".to_string())?;
            (scaled, width, height)
        }
        _ => (canvas.clone(), source_width, source_height),
    };
    let promise = target
        .convert_to_blob()
        .map_err(|_| "failed to start the png encode".to_string())?;
    Ok((promise, width, height))
}

async fn finish_capture(promise: js_sys::Promise) -> Result<String, String> {
    let blob: Blob = JsFuture::from(promise)
        .await
        .map_err(|_| "the png encode failed".to_string())?
        .unchecked_into();
    let reader =
        FileReaderSync::new().map_err(|_| "failed to create the blob reader".to_string())?;
    let data_url = reader
        .read_as_data_url(&blob)
        .map_err(|_| "failed to read the png blob".to_string())?;
    let comma = data_url
        .find(',')
        .ok_or_else(|| "unexpected data url shape".to_string())?;
    Ok(data_url[comma + 1..].to_string())
}

fn fail(correlation_id: CorrelationId, message: &str) {
    post(&WorkerMessage::Agent(Box::new(AgentResponse::Error {
        correlation_id,
        message: message.to_string(),
    })));
}
