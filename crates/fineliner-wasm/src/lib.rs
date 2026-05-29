//! wasm-bindgen bindings exposing `fineliner-core` to JavaScript (spec §17).
//!
//! Document state is owned in Rust: handles are opaque indices into a
//! thread-local arena, and JS only ever receives composited pixel buffers and
//! exported bytes (ADR-001). Commands are passed as JSON strings.

use fineliner_core::codec::{to_jpeg_bytes, to_png_bytes, to_webp_bytes};
use fineliner_core::command::{AddLayer, CommandBus, RemoveLayer};
use fineliner_core::{
    compose, Brush, BrushShape, Color, Document, Eraser, EraserMode, Eyedropper, Fill, FillOptions,
    Move, Pencil, Point, SampleSize, SampleSource,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;

thread_local! {
    /// Open documents, indexed by handle. `None` slots are closed documents.
    static DOCUMENTS: RefCell<Vec<Option<CommandBus>>> = const { RefCell::new(Vec::new()) };
}

/// Installs a panic hook that logs Rust panics to the browser console.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Inserts a bus into the arena and returns its handle.
fn insert(bus: CommandBus) -> u32 {
    DOCUMENTS.with(|docs| {
        let mut docs = docs.borrow_mut();
        if let Some(slot) = docs.iter().position(Option::is_none) {
            docs[slot] = Some(bus);
            slot as u32
        } else {
            docs.push(Some(bus));
            (docs.len() - 1) as u32
        }
    })
}

/// Runs `f` against the bus for `handle`, mapping a missing handle to a JS error.
fn with_bus<T>(
    handle: u32,
    f: impl FnOnce(&mut CommandBus) -> Result<T, JsError>,
) -> Result<T, JsError> {
    DOCUMENTS.with(|docs| {
        let mut docs = docs.borrow_mut();
        match docs.get_mut(handle as usize).and_then(Option::as_mut) {
            Some(bus) => f(bus),
            None => Err(JsError::new("invalid document handle")),
        }
    })
}

/// Creates a new blank document of `width` × `height`. Returns its handle.
#[wasm_bindgen]
pub fn create_document(width: u32, height: u32) -> Result<u32, JsError> {
    let doc = Document::new(width, height).map_err(to_js)?;
    Ok(insert(CommandBus::new(doc)))
}

/// Opens an encoded image (PNG/JPEG/WebP/BMP/GIF/TIFF) as a single-layer
/// document. `mime_type` is accepted for API parity but format is auto-detected.
#[wasm_bindgen]
pub fn open_image(data: &[u8], _mime_type: &str) -> Result<u32, JsError> {
    let buffer = fineliner_core::codec::decode(data).map_err(|e| JsError::new(&e.to_string()))?;
    let doc = Document::from_pixels(buffer).map_err(to_js)?;
    Ok(insert(CommandBus::new(doc)))
}

/// Releases the document for `handle`.
#[wasm_bindgen]
pub fn close_document(handle: u32) {
    DOCUMENTS.with(|docs| {
        if let Some(slot) = docs.borrow_mut().get_mut(handle as usize) {
            *slot = None;
        }
    });
}

/// Returns the flattened composite as an RGBA8 `Uint8ClampedArray`, ready to
/// wrap in an `ImageData` (spec §17).
#[wasm_bindgen]
pub fn composite(handle: u32) -> Result<Clamped<Vec<u8>>, JsError> {
    with_bus(handle, |bus| {
        Ok(Clamped(compose(bus.document.layers()).into_raw()))
    })
}

/// Default brush shape when JS omits it (hard round preserves M5 behavior).
fn default_shape() -> String {
    "hard_round".to_string()
}

/// Default brush hardness when JS omits it.
fn default_hardness() -> f32 {
    1.0
}

/// Default eraser mode when JS omits it.
fn default_eraser_mode() -> String {
    "to_transparent".to_string()
}

/// Default eraser background when JS omits it: opaque white, matching the core
/// [`Eraser`] default rather than serde's transparent-black zero value.
fn default_background() -> [u8; 4] {
    [255, 255, 255, 255]
}

/// Default color sample source when JS omits it.
fn default_sample() -> String {
    "current_layer".to_string()
}

/// Maps a brush-shape string to a [`BrushShape`], defaulting to hard round.
fn parse_shape(s: &str) -> BrushShape {
    match s {
        "soft_round" => BrushShape::SoftRound,
        "flat" => BrushShape::Flat,
        _ => BrushShape::HardRound,
    }
}

/// Maps an eraser-mode string to an [`EraserMode`], defaulting to transparent.
fn parse_eraser_mode(s: &str) -> EraserMode {
    match s {
        "to_background" => EraserMode::ToBackground,
        _ => EraserMode::ToTransparent,
    }
}

/// Maps a sample-source string to a [`SampleSource`], defaulting to the layer.
fn parse_sample(s: &str) -> SampleSource {
    match s {
        "all_layers" => SampleSource::AllLayers,
        _ => SampleSource::CurrentLayer,
    }
}

/// Maps an averaging edge length to a [`SampleSize`], defaulting to 1×1.
fn sample_size(edge: u32) -> SampleSize {
    match edge {
        3 => SampleSize::ThreeByThree,
        5 => SampleSize::FiveByFive,
        11 => SampleSize::ElevenByEleven,
        31 => SampleSize::ThirtyOneByThirtyOne,
        _ => SampleSize::One,
    }
}

/// A JSON-serializable command from JS (spec §17 `SerializedCommand`).
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum CommandSpec {
    /// A pencil stroke over a polyline of `[x, y]` points.
    PencilStroke {
        layer: usize,
        size: u32,
        color: [u8; 4],
        opacity: f32,
        #[serde(default = "default_shape")]
        shape: String,
        #[serde(default = "default_hardness")]
        hardness: f32,
        points: Vec<[f32; 2]>,
        /// Identifies the pointer drag; segments sharing it merge into one undo
        /// step. The UI assigns a fresh id per pointer-down.
        stroke_id: u64,
    },
    /// An eraser stroke over a polyline of `[x, y]` points.
    EraserStroke {
        layer: usize,
        size: u32,
        opacity: f32,
        #[serde(default = "default_shape")]
        shape: String,
        #[serde(default = "default_hardness")]
        hardness: f32,
        #[serde(default = "default_eraser_mode")]
        mode: String,
        #[serde(default = "default_background")]
        background: [u8; 4],
        points: Vec<[f32; 2]>,
        stroke_id: u64,
    },
    /// A flood fill seeded at `[x, y]`.
    FillBucket {
        layer: usize,
        color: [u8; 4],
        opacity: f32,
        tolerance: u8,
        contiguous: bool,
        #[serde(default = "default_sample")]
        sample: String,
        x: f32,
        y: f32,
    },
    /// Translate a layer's contents by `(dx, dy)` pixels (the Move tool).
    TranslateLayer { layer: usize, dx: i32, dy: i32 },
    /// Add a transparent layer above `active`.
    AddLayer { active: usize },
    /// Remove the layer at `index`.
    RemoveLayer { index: usize },
}

/// Applies a JSON-encoded command to the document and records it in history.
#[wasm_bindgen]
pub fn apply_command(handle: u32, command: &str) -> Result<(), JsError> {
    let spec: CommandSpec =
        serde_json::from_str(command).map_err(|e| JsError::new(&e.to_string()))?;
    with_bus(handle, |bus| match spec {
        CommandSpec::PencilStroke {
            layer,
            size,
            color,
            opacity,
            shape,
            hardness,
            points,
            stroke_id,
        } => {
            let brush = Brush::new(
                size,
                Color::rgba(color[0], color[1], color[2], color[3]),
                opacity,
            )
            .with_shape(parse_shape(&shape))
            .with_hardness(hardness);
            let pts: Vec<Point> = points.iter().map(|p| Point::new(p[0], p[1])).collect();
            match Pencil::new(brush).stroke(layer, &pts, &bus.document) {
                Some(cmd) => bus
                    .apply(Box::new(cmd.with_stroke(stroke_id)))
                    .map_err(to_js),
                None => Ok(()), // stroke missed the canvas — no-op
            }
        }
        CommandSpec::EraserStroke {
            layer,
            size,
            opacity,
            shape,
            hardness,
            mode,
            background,
            points,
            stroke_id,
        } => {
            let brush = Brush::new(size, Color::TRANSPARENT, opacity)
                .with_shape(parse_shape(&shape))
                .with_hardness(hardness);
            let eraser = Eraser::new(brush, parse_eraser_mode(&mode)).with_background(Color::rgba(
                background[0],
                background[1],
                background[2],
                background[3],
            ));
            let pts: Vec<Point> = points.iter().map(|p| Point::new(p[0], p[1])).collect();
            match eraser.stroke(layer, &pts, &bus.document) {
                Some(cmd) => bus
                    .apply(Box::new(cmd.with_stroke(stroke_id)))
                    .map_err(to_js),
                None => Ok(()),
            }
        }
        CommandSpec::FillBucket {
            layer,
            color,
            opacity,
            tolerance,
            contiguous,
            sample,
            x,
            y,
        } => {
            let options = FillOptions {
                tolerance,
                contiguous,
                sample: parse_sample(&sample),
            };
            let fill = Fill::new(Color::rgba(color[0], color[1], color[2], color[3]), options)
                .with_opacity(opacity);
            match fill.fill(layer, Point::new(x, y), &bus.document) {
                Some(cmd) => bus.apply(Box::new(cmd)).map_err(to_js),
                None => Ok(()),
            }
        }
        CommandSpec::TranslateLayer { layer, dx, dy } => {
            match Move.translate(layer, dx, dy, &bus.document) {
                Some(cmd) => bus.apply(Box::new(cmd)).map_err(to_js),
                None => Ok(()),
            }
        }
        CommandSpec::AddLayer { active } => {
            bus.apply(Box::new(AddLayer::above(active))).map_err(to_js)
        }
        CommandSpec::RemoveLayer { index } => {
            bus.apply(Box::new(RemoveLayer::at(index))).map_err(to_js)
        }
    })
}

/// Samples a color at canvas point `(x, y)` (the Eyedropper tool).
///
/// `sample` is `"current_layer"` or `"all_layers"`; `size` is the averaging
/// edge length (1, 3, 5, 11, or 31). Returns the 4 RGBA bytes, or an empty
/// array if the point lies off the canvas. Sampling is not undoable, so this is
/// a query, not a command.
#[wasm_bindgen]
pub fn pick_color(
    handle: u32,
    x: f32,
    y: f32,
    sample: &str,
    size: u32,
) -> Result<Vec<u8>, JsError> {
    with_bus(handle, |bus| {
        let eyedropper = Eyedropper::new(parse_sample(sample), sample_size(size));
        match eyedropper.pick(Point::new(x, y), &bus.document) {
            Some(c) => Ok(vec![c.r, c.g, c.b, c.a]),
            None => Ok(Vec::new()),
        }
    })
}

/// Undoes the last command. Returns `true` if something was undone.
#[wasm_bindgen]
pub fn undo(handle: u32) -> Result<bool, JsError> {
    with_bus(handle, |bus| bus.undo().map_err(to_js))
}

/// Redoes the last undone command. Returns `true` if something was redone.
#[wasm_bindgen]
pub fn redo(handle: u32) -> Result<bool, JsError> {
    with_bus(handle, |bus| bus.redo().map_err(to_js))
}

/// Exports the flattened composite as PNG bytes. `compression` is 0–9.
#[wasm_bindgen]
pub fn export_png(handle: u32, compression: u8) -> Result<Vec<u8>, JsError> {
    with_bus(handle, |bus| {
        to_png_bytes(&compose(bus.document.layers()), compression)
            .map_err(|e| JsError::new(&e.to_string()))
    })
}

/// Exports the flattened composite as JPEG bytes. `quality` is 1–100.
#[wasm_bindgen]
pub fn export_jpeg(handle: u32, quality: u8) -> Result<Vec<u8>, JsError> {
    with_bus(handle, |bus| {
        to_jpeg_bytes(&compose(bus.document.layers()), quality)
            .map_err(|e| JsError::new(&e.to_string()))
    })
}

/// Exports the flattened composite as lossless WebP bytes (ADR-007).
#[wasm_bindgen]
pub fn export_webp(handle: u32) -> Result<Vec<u8>, JsError> {
    with_bus(handle, |bus| {
        to_webp_bytes(&compose(bus.document.layers())).map_err(|e| JsError::new(&e.to_string()))
    })
}

/// Lightweight document state for the UI (spec §17 `DocumentInfo`).
#[derive(Debug, Serialize)]
struct DocumentInfo {
    width: u32,
    height: u32,
    layer_count: usize,
    active_layer: usize,
    can_undo: bool,
    can_redo: bool,
}

/// Returns the current document state as a plain JS object.
#[wasm_bindgen]
pub fn get_document_info(handle: u32) -> Result<JsValue, JsError> {
    with_bus(handle, |bus| {
        let info = DocumentInfo {
            width: bus.document.canvas.width(),
            height: bus.document.canvas.height(),
            layer_count: bus.document.layer_count(),
            active_layer: bus.document.active_layer_index(),
            can_undo: bus.history.can_undo(),
            can_redo: bus.history.can_redo(),
        };
        serde_wasm_bindgen::to_value(&info).map_err(|e| JsError::new(&e.to_string()))
    })
}

/// Converts a core error into a JS error.
fn to_js(e: fineliner_core::DocumentError) -> JsError {
    JsError::new(&e.to_string())
}
