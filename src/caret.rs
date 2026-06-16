//! Pixel geometry for the textarea: where a document cell sits on screen, and
//! which cell a pointer is over. Shared by the LSP popups and the jump overlay.
//! Character advance is measured once per font with a 2d canvas and cached.

use std::cell::RefCell;
use std::collections::HashMap;

use wasm_bindgen::JsCast;
use web_sys::HtmlTextAreaElement;

struct Metrics {
    pad_left: f64,
    pad_top: f64,
    char_width: f64,
    line_height: f64,
}

fn metrics(element: &HtmlTextAreaElement) -> Metrics {
    let style =
        web_sys::window().and_then(|window| window.get_computed_style(element).ok().flatten());
    let font_size = parse_px(style.as_ref(), "font-size").unwrap_or(13.0);
    let line_height = parse_px(style.as_ref(), "line-height").unwrap_or(font_size * 1.5);
    let pad_left = parse_px(style.as_ref(), "padding-left").unwrap_or(0.0);
    let pad_top = parse_px(style.as_ref(), "padding-top").unwrap_or(0.0);
    let family = style
        .as_ref()
        .and_then(|style| style.get_property_value("font-family").ok())
        .filter(|family| !family.is_empty())
        .unwrap_or_else(|| "monospace".to_string());
    let font = format!("{font_size}px {family}");
    Metrics {
        pad_left,
        pad_top,
        char_width: char_width(&font).unwrap_or(font_size * 0.6),
        line_height,
    }
}

fn char_width(font: &str) -> Option<f64> {
    thread_local! {
        static CACHE: RefCell<HashMap<String, f64>> = RefCell::new(HashMap::new());
    }
    if let Some(width) = CACHE.with(|cache| cache.borrow().get(font).copied()) {
        return Some(width);
    }
    let document = web_sys::window()?.document()?;
    let canvas: web_sys::HtmlCanvasElement =
        document.create_element("canvas").ok()?.dyn_into().ok()?;
    let context: web_sys::CanvasRenderingContext2d =
        canvas.get_context("2d").ok()??.dyn_into().ok()?;
    context.set_font(font);
    let sample = "MMMMMMMMMMMMMMMMMMMM";
    let width = context.measure_text(sample).ok()?.width() / sample.len() as f64;
    if width <= 0.0 {
        return None;
    }
    CACHE.with(|cache| cache.borrow_mut().insert(font.to_string(), width));
    Some(width)
}

fn parse_px(style: Option<&web_sys::CssStyleDeclaration>, property: &str) -> Option<f64> {
    let raw = style?.get_property_value(property).ok()?;
    raw.trim_end_matches("px").trim().parse().ok()
}

/// The top-left viewport pixel of the cell at a document line and UTF-16 column.
pub fn cell(element: &HtmlTextAreaElement, line: u32, column: u32) -> (f64, f64) {
    let rect = element.get_bounding_client_rect();
    let metrics = metrics(element);
    let x = rect.left() + metrics.pad_left + column as f64 * metrics.char_width
        - element.scroll_left() as f64;
    let y = rect.top() + metrics.pad_top + line as f64 * metrics.line_height
        - element.scroll_top() as f64;
    (x, y)
}

/// The line height in pixels, for placing a popup below a cell.
pub fn line_height(element: &HtmlTextAreaElement) -> f64 {
    metrics(element).line_height
}

/// The document (line, column) under a viewport pixel point.
pub fn locate(element: &HtmlTextAreaElement, client_x: f64, client_y: f64) -> (u32, u32) {
    let rect = element.get_bounding_client_rect();
    let metrics = metrics(element);
    let column = (((client_x - rect.left() - metrics.pad_left + element.scroll_left() as f64)
        / metrics.char_width)
        .floor())
    .max(0.0) as u32;
    let line = (((client_y - rect.top() - metrics.pad_top + element.scroll_top() as f64)
        / metrics.line_height)
        .floor())
    .max(0.0) as u32;
    (line, column)
}
