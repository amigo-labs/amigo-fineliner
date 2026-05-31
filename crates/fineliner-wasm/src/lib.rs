//! wasm-bindgen bindings exposing `fineliner-core` to JavaScript (spec §17).
//!
//! Document state is owned in Rust: handles are opaque indices into a
//! thread-local arena, and JS only ever receives composited pixel buffers and
//! exported bytes (ADR-001). Commands are passed as JSON strings.

use fineliner_core::codec::{to_jpeg_bytes, to_png_bytes, to_webp_bytes};
use fineliner_core::command::{
    AddLayer, Anchor, CanvasRotation, CommandBus, CropToSelection, DuplicateLayer, FlattenImage,
    FlipCanvas, LayerTransform, MergeDown, MergeVisible, MoveLayer, RemoveLayer, RenameLayer,
    ResizeCanvas, RotateCanvas, RotateLayer90, ScaleImage, SetLayerBlendMode, SetLayerLocked,
    SetLayerOpacity, SetLayerVisible, SetSelection, TransformLayer,
};
use fineliner_core::{
    apply_mode, compose, magic_wand, BlendMode, Brush, BrushShape, Color, Document, Eraser,
    EraserMode, Eyedropper, Fill, FillOptions, ImageBuffer, Interpolation, Move, Pencil, Point,
    Rect, SampleSize, SampleSource, SelectionMask, SelectionMode, Shape, ShapeMode, ShapeStyle,
    Shapes,
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

/// Default selection mode when JS omits it.
fn default_selection_mode() -> String {
    "replace".to_string()
}

/// Default resampling when JS omits it.
fn default_interpolation() -> String {
    "bilinear".to_string()
}

/// Default resize anchor when JS omits it.
fn default_anchor() -> String {
    "top_left".to_string()
}

/// Default shape mode when JS omits it.
fn default_shape_mode() -> String {
    "outline".to_string()
}

/// Default shape stroke width when JS omits it.
fn default_stroke_width() -> f32 {
    1.0
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

/// Maps a snake_case blend-mode string to a [`BlendMode`], defaulting to Normal.
fn parse_blend_mode(s: &str) -> BlendMode {
    match s {
        "multiply" => BlendMode::Multiply,
        "screen" => BlendMode::Screen,
        "overlay" => BlendMode::Overlay,
        "darken" => BlendMode::Darken,
        "lighten" => BlendMode::Lighten,
        "color_dodge" => BlendMode::ColorDodge,
        "color_burn" => BlendMode::ColorBurn,
        "hard_light" => BlendMode::HardLight,
        "soft_light" => BlendMode::SoftLight,
        "difference" => BlendMode::Difference,
        "exclusion" => BlendMode::Exclusion,
        _ => BlendMode::Normal,
    }
}

/// Maps a [`BlendMode`] to its stable snake_case string for the JS layer.
fn blend_mode_str(mode: BlendMode) -> &'static str {
    match mode {
        BlendMode::Normal => "normal",
        BlendMode::Multiply => "multiply",
        BlendMode::Screen => "screen",
        BlendMode::Overlay => "overlay",
        BlendMode::Darken => "darken",
        BlendMode::Lighten => "lighten",
        BlendMode::ColorDodge => "color_dodge",
        BlendMode::ColorBurn => "color_burn",
        BlendMode::HardLight => "hard_light",
        BlendMode::SoftLight => "soft_light",
        BlendMode::Difference => "difference",
        BlendMode::Exclusion => "exclusion",
    }
}

/// Edge length of layer thumbnails (spec §5.3).
const THUMBNAIL_DIM: u32 = 32;

/// Downscales a layer buffer to a 32×32 RGBA8 thumbnail by nearest-neighbor
/// sampling (spec §5.3).
fn thumbnail(src: &ImageBuffer) -> Vec<u8> {
    let mut out = vec![0u8; (THUMBNAIL_DIM * THUMBNAIL_DIM * 4) as usize];
    let sw = src.width().max(1);
    let sh = src.height().max(1);
    for ty in 0..THUMBNAIL_DIM {
        for tx in 0..THUMBNAIL_DIM {
            let sx = (tx * sw / THUMBNAIL_DIM).min(sw - 1);
            let sy = (ty * sh / THUMBNAIL_DIM).min(sh - 1);
            let c = src.get_pixel(sx, sy).unwrap_or(Color::TRANSPARENT);
            let i = ((ty * THUMBNAIL_DIM + tx) * 4) as usize;
            out[i] = c.r;
            out[i + 1] = c.g;
            out[i + 2] = c.b;
            out[i + 3] = c.a;
        }
    }
    out
}

/// Maps a selection-mode string to a [`SelectionMode`], defaulting to Replace.
fn parse_selection_mode(s: &str) -> SelectionMode {
    match s {
        "add" => SelectionMode::Add,
        "subtract" => SelectionMode::Subtract,
        "intersect" => SelectionMode::Intersect,
        _ => SelectionMode::Replace,
    }
}

/// Maps a shape-mode string to a [`ShapeMode`], defaulting to outline.
fn parse_shape_mode(s: &str) -> ShapeMode {
    match s {
        "fill" => ShapeMode::Fill,
        "fill_and_outline" => ShapeMode::FillAndOutline,
        _ => ShapeMode::Outline,
    }
}

/// Maps an interpolation string to an [`Interpolation`], defaulting to bilinear.
fn parse_interpolation(s: &str) -> Interpolation {
    match s {
        "nearest" => Interpolation::Nearest,
        "bicubic" => Interpolation::Bicubic,
        _ => Interpolation::Bilinear,
    }
}

/// Maps a 9-grid anchor string to an [`Anchor`], defaulting to top-left.
fn parse_anchor(s: &str) -> Anchor {
    match s {
        "top_center" => Anchor::TopCenter,
        "top_right" => Anchor::TopRight,
        "center_left" => Anchor::CenterLeft,
        "center" => Anchor::Center,
        "center_right" => Anchor::CenterRight,
        "bottom_left" => Anchor::BottomLeft,
        "bottom_center" => Anchor::BottomCenter,
        "bottom_right" => Anchor::BottomRight,
        _ => Anchor::TopLeft,
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
    /// Reorder the layer at `from` to position `to`.
    MoveLayer { from: usize, to: usize },
    /// Duplicate the layer at `index`, inserting the copy above it.
    DuplicateLayer { index: usize },
    /// Rename the layer at `index`.
    RenameLayer { index: usize, name: String },
    /// Set the opacity (0.0–1.0) of the layer at `index`.
    SetLayerOpacity { index: usize, opacity: f32 },
    /// Set the blend mode of the layer at `index` (snake_case string).
    SetLayerBlendMode { index: usize, mode: String },
    /// Show or hide the layer at `index`.
    SetLayerVisible { index: usize, visible: bool },
    /// Lock or unlock pixel edits on the layer at `index`.
    SetLayerLocked { index: usize, locked: bool },
    /// Merge the layer at `index` onto the layer below it.
    MergeDown { index: usize },
    /// Flatten all visible layers into one.
    MergeVisible,
    /// Flatten every layer onto an opaque white background.
    FlattenImage,
    /// Rectangular selection over `(x, y, w, h)` combined per `mode`.
    SelectRectangle {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        #[serde(default = "default_selection_mode")]
        mode: String,
        #[serde(default)]
        feather: u32,
    },
    /// Elliptical selection inscribed in `(x, y, w, h)` combined per `mode`.
    SelectEllipse {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        #[serde(default = "default_selection_mode")]
        mode: String,
        #[serde(default)]
        feather: u32,
    },
    /// Polygonal selection over `points` ([x, y] each) combined per `mode`.
    SelectPolygon {
        points: Vec<[f32; 2]>,
        #[serde(default = "default_selection_mode")]
        mode: String,
        #[serde(default)]
        feather: u32,
    },
    /// Magic-wand selection seeded at `(x, y)` on `layer`, combined per `mode`.
    SelectWand {
        layer: usize,
        x: f32,
        y: f32,
        tolerance: u8,
        contiguous: bool,
        #[serde(default = "default_sample")]
        sample: String,
        #[serde(default = "default_selection_mode")]
        mode: String,
    },
    /// Select the whole canvas (spec §8.4 Select All).
    SelectAll,
    /// Clear the selection (spec §8.4 Deselect).
    Deselect,
    /// Invert the selection (spec §8.4 Invert).
    InvertSelection,
    /// Grow the selection by `radius` pixels (spec §8.4 Expand).
    ExpandSelection { radius: u32 },
    /// Shrink the selection by `radius` pixels (spec §8.4 Contract).
    ContractSelection { radius: u32 },
    /// Soften the selection edges by `radius` pixels (spec §8.4 Feather).
    FeatherSelection { radius: u32 },
    /// Flip or 180°-rotate the active layer (`op`: flip_h/flip_v/rotate_180).
    TransformLayer { layer: usize, op: String },
    /// Rotate the active layer 90° (counter-clockwise when `ccw`).
    RotateLayer90 { layer: usize, ccw: bool },
    /// Flip the whole canvas (all layers) horizontally or vertically.
    FlipCanvas { horizontal: bool },
    /// Rotate the whole canvas (`rotation`: cw90/ccw90/rotate_180).
    RotateCanvas { rotation: String },
    /// Scale the whole image to `width` × `height` with `interpolation`.
    ScaleImage {
        width: u32,
        height: u32,
        #[serde(default = "default_interpolation")]
        interpolation: String,
    },
    /// Resize the canvas to `width` × `height`, placing content per `anchor`.
    ResizeCanvas {
        width: u32,
        height: u32,
        #[serde(default = "default_anchor")]
        anchor: String,
    },
    /// Crop the canvas to the current selection's bounding box (spec §10.6).
    CropToSelection,
    /// Rasterize a shape onto `layer` (spec §9.2 Shapes).
    ///
    /// `shape` selects the geometry: `line`/`rectangle`/`rounded_rectangle`/
    /// `ellipse` read `points` as `[a, b]` (endpoints or opposite corners);
    /// `polygon` reads `center`, `radius`, `sides` and `rotation`. The
    /// `corner_radius` applies to rounded rectangles only.
    DrawShape {
        layer: usize,
        shape: String,
        #[serde(default)]
        points: Vec<[f32; 2]>,
        #[serde(default)]
        corner_radius: f32,
        #[serde(default)]
        center: [f32; 2],
        #[serde(default)]
        radius: f32,
        #[serde(default)]
        sides: u32,
        #[serde(default)]
        rotation: f32,
        #[serde(default = "default_shape_mode")]
        mode: String,
        #[serde(default = "default_stroke_width")]
        stroke_width: f32,
        #[serde(default)]
        stroke_color: [u8; 4],
        #[serde(default)]
        fill_color: [u8; 4],
        #[serde(default)]
        anti_alias: bool,
    },
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
        CommandSpec::MoveLayer { from, to } => {
            bus.apply(Box::new(MoveLayer::new(from, to))).map_err(to_js)
        }
        CommandSpec::DuplicateLayer { index } => bus
            .apply(Box::new(DuplicateLayer::at(index)))
            .map_err(to_js),
        CommandSpec::RenameLayer { index, name } => bus
            .apply(Box::new(RenameLayer::new(index, name)))
            .map_err(to_js),
        CommandSpec::SetLayerOpacity { index, opacity } => bus
            .apply(Box::new(SetLayerOpacity::new(index, opacity)))
            .map_err(to_js),
        CommandSpec::SetLayerBlendMode { index, mode } => bus
            .apply(Box::new(SetLayerBlendMode::new(
                index,
                parse_blend_mode(&mode),
            )))
            .map_err(to_js),
        CommandSpec::SetLayerVisible { index, visible } => bus
            .apply(Box::new(SetLayerVisible::new(index, visible)))
            .map_err(to_js),
        CommandSpec::SetLayerLocked { index, locked } => bus
            .apply(Box::new(SetLayerLocked::new(index, locked)))
            .map_err(to_js),
        CommandSpec::MergeDown { index } => {
            bus.apply(Box::new(MergeDown::at(index))).map_err(to_js)
        }
        CommandSpec::MergeVisible => bus.apply(Box::new(MergeVisible::new())).map_err(to_js),
        CommandSpec::FlattenImage => bus.apply(Box::new(FlattenImage::new())).map_err(to_js),
        CommandSpec::SelectRectangle {
            x,
            y,
            w,
            h,
            mode,
            feather,
        } => {
            let (cw, ch) = (bus.document.canvas.width(), bus.document.canvas.height());
            let mut shape = SelectionMask::rectangle(cw, ch, Rect::new(x, y, w, h));
            shape.feather(feather);
            apply_selection(bus, shape, parse_selection_mode(&mode), "Rectangle Select")
        }
        CommandSpec::SelectEllipse {
            x,
            y,
            w,
            h,
            mode,
            feather,
        } => {
            let (cw, ch) = (bus.document.canvas.width(), bus.document.canvas.height());
            let mut shape = SelectionMask::ellipse(cw, ch, Rect::new(x, y, w, h));
            shape.feather(feather);
            apply_selection(bus, shape, parse_selection_mode(&mode), "Ellipse Select")
        }
        CommandSpec::SelectPolygon {
            points,
            mode,
            feather,
        } => {
            let (cw, ch) = (bus.document.canvas.width(), bus.document.canvas.height());
            let pts: Vec<Point> = points.iter().map(|p| Point::new(p[0], p[1])).collect();
            let mut shape = SelectionMask::polygon(cw, ch, &pts);
            shape.feather(feather);
            apply_selection(bus, shape, parse_selection_mode(&mode), "Lasso Select")
        }
        CommandSpec::SelectWand {
            layer,
            x,
            y,
            tolerance,
            contiguous,
            sample,
            mode,
        } => {
            match magic_wand(
                &bus.document,
                layer,
                Point::new(x, y),
                tolerance,
                contiguous,
                parse_sample(&sample),
            ) {
                Some(shape) => {
                    apply_selection(bus, shape, parse_selection_mode(&mode), "Magic Wand")
                }
                None => Ok(()), // off-canvas seed — no-op
            }
        }
        CommandSpec::SelectAll => {
            let (cw, ch) = (bus.document.canvas.width(), bus.document.canvas.height());
            bus.apply(Box::new(
                SetSelection::replace(SelectionMask::new_full(cw, ch)).with_label("Select All"),
            ))
            .map_err(to_js)
        }
        CommandSpec::Deselect => bus.apply(Box::new(SetSelection::clear())).map_err(to_js),
        CommandSpec::InvertSelection => {
            let (cw, ch) = (bus.document.canvas.width(), bus.document.canvas.height());
            // No selection means everything is selected, so its inverse is empty.
            let mut mask = bus
                .document
                .selection
                .clone()
                .unwrap_or_else(|| SelectionMask::new_full(cw, ch));
            mask.invert();
            bus.apply(Box::new(
                SetSelection::replace(mask).with_label("Invert Selection"),
            ))
            .map_err(to_js)
        }
        CommandSpec::ExpandSelection { radius } => {
            modify_selection(bus, "Expand Selection", |m| m.expand(radius))
        }
        CommandSpec::ContractSelection { radius } => {
            modify_selection(bus, "Contract Selection", |m| m.contract(radius))
        }
        CommandSpec::FeatherSelection { radius } => {
            modify_selection(bus, "Feather Selection", |m| m.feather(radius))
        }
        CommandSpec::TransformLayer { layer, op } => {
            let op = match op.as_str() {
                "flip_v" => LayerTransform::FlipVertical,
                "rotate_180" => LayerTransform::Rotate180,
                _ => LayerTransform::FlipHorizontal,
            };
            bus.apply(Box::new(TransformLayer::new(layer, op)))
                .map_err(to_js)
        }
        CommandSpec::RotateLayer90 { layer, ccw } => bus
            .apply(Box::new(RotateLayer90::new(layer, ccw)))
            .map_err(to_js),
        CommandSpec::FlipCanvas { horizontal } => bus
            .apply(Box::new(FlipCanvas::new(horizontal)))
            .map_err(to_js),
        CommandSpec::RotateCanvas { rotation } => {
            let rot = match rotation.as_str() {
                "ccw90" => CanvasRotation::Ccw90,
                "rotate_180" => CanvasRotation::Rotate180,
                _ => CanvasRotation::Cw90,
            };
            bus.apply(Box::new(RotateCanvas::new(rot))).map_err(to_js)
        }
        CommandSpec::ScaleImage {
            width,
            height,
            interpolation,
        } => bus
            .apply(Box::new(ScaleImage::new(
                width,
                height,
                parse_interpolation(&interpolation),
            )))
            .map_err(to_js),
        CommandSpec::ResizeCanvas {
            width,
            height,
            anchor,
        } => bus
            .apply(Box::new(
                ResizeCanvas::new(width, height).with_anchor(parse_anchor(&anchor)),
            ))
            .map_err(to_js),
        CommandSpec::CropToSelection => bus.apply(Box::new(CropToSelection::new())).map_err(to_js),
        CommandSpec::DrawShape {
            layer,
            shape,
            points,
            corner_radius,
            center,
            radius,
            sides,
            rotation,
            mode,
            stroke_width,
            stroke_color,
            fill_color,
            anti_alias,
        } => {
            let corner = |i: usize| points.get(i).map(|q| Point::new(q[0], q[1]));
            // line / rectangle / rounded_rectangle / ellipse take `points[0..2]`.
            let built = match shape.as_str() {
                "line" => corner(0).zip(corner(1)).map(|(a, b)| Shape::Line { a, b }),
                "rectangle" => corner(0)
                    .zip(corner(1))
                    .map(|(a, b)| Shape::Rectangle { a, b }),
                "rounded_rectangle" => {
                    corner(0)
                        .zip(corner(1))
                        .map(|(a, b)| Shape::RoundedRectangle {
                            a,
                            b,
                            radius: corner_radius,
                        })
                }
                "ellipse" => corner(0)
                    .zip(corner(1))
                    .map(|(a, b)| Shape::Ellipse { a, b }),
                "polygon" => Some(Shape::Polygon {
                    center: Point::new(center[0], center[1]),
                    radius,
                    sides,
                    rotation,
                }),
                _ => None,
            };
            let style = ShapeStyle {
                mode: parse_shape_mode(&mode),
                stroke_width,
                stroke_color: Color::rgba(
                    stroke_color[0],
                    stroke_color[1],
                    stroke_color[2],
                    stroke_color[3],
                ),
                fill_color: Color::rgba(fill_color[0], fill_color[1], fill_color[2], fill_color[3]),
                anti_alias,
            };
            match built.and_then(|s| Shapes::new(s, style).draw(layer, &bus.document)) {
                Some(cmd) => bus.apply(Box::new(cmd)).map_err(to_js),
                None => Ok(()), // invalid geometry or off-canvas — no-op
            }
        }
    })
}

/// Combines `shape` with the current selection per `mode` and applies it as an
/// undoable [`SetSelection`] labeled `label`.
fn apply_selection(
    bus: &mut CommandBus,
    shape: SelectionMask,
    mode: SelectionMode,
    label: &'static str,
) -> Result<(), JsError> {
    let after = apply_mode(bus.document.selection.as_ref(), shape, mode);
    bus.apply(Box::new(SetSelection::replace(after).with_label(label)))
        .map_err(to_js)
}

/// Applies an in-place modifier (`expand`/`contract`/`feather`) to the current
/// selection. A no-op when nothing is selected (the whole canvas is implied).
fn modify_selection(
    bus: &mut CommandBus,
    label: &'static str,
    f: impl FnOnce(&mut SelectionMask),
) -> Result<(), JsError> {
    let Some(mut mask) = bus.document.selection.clone() else {
        return Ok(());
    };
    f(&mut mask);
    bus.apply(Box::new(SetSelection::replace(mask).with_label(label)))
        .map_err(to_js)
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

/// Selects the active layer by `index`. Layer selection is UI state, not an
/// undoable edit, so this is a setter rather than a command (spec §5, §7.3).
#[wasm_bindgen]
pub fn set_active_layer(handle: u32, index: usize) -> Result<(), JsError> {
    with_bus(handle, |bus| {
        bus.document.set_active_layer(index).map_err(to_js)
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

/// Per-layer state for the layers panel (spec §16.5).
#[derive(Debug, Serialize)]
struct LayerInfo {
    /// Stable layer id (string form of the core `Uuid`).
    id: String,
    name: String,
    opacity: f32,
    /// Blend mode as a snake_case string (see [`blend_mode_str`]).
    blend_mode: String,
    visible: bool,
    locked: bool,
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
    /// Whether a selection is currently active (`Some` mask in the document).
    has_selection: bool,
    /// Layers ordered bottom (index 0) to top, matching core storage order.
    layers: Vec<LayerInfo>,
}

/// Returns the current document state as a plain JS object.
#[wasm_bindgen]
pub fn get_document_info(handle: u32) -> Result<JsValue, JsError> {
    with_bus(handle, |bus| {
        let layers = bus
            .document
            .layers()
            .iter()
            .map(|l| LayerInfo {
                id: l.id.to_string(),
                name: l.name.clone(),
                opacity: l.opacity,
                blend_mode: blend_mode_str(l.blend_mode).to_string(),
                visible: l.visible,
                locked: l.locked,
            })
            .collect();
        let info = DocumentInfo {
            width: bus.document.canvas.width(),
            height: bus.document.canvas.height(),
            layer_count: bus.document.layer_count(),
            active_layer: bus.document.active_layer_index(),
            can_undo: bus.history.can_undo(),
            can_redo: bus.history.can_redo(),
            has_selection: bus.document.selection.is_some(),
            layers,
        };
        serde_wasm_bindgen::to_value(&info).map_err(|e| JsError::new(&e.to_string()))
    })
}

/// Returns a 32×32 RGBA8 thumbnail of the layer with `layer_id` as a
/// `Uint8ClampedArray`, ready to wrap in an `ImageData` (spec §5.3, §17).
#[wasm_bindgen]
pub fn get_layer_thumbnail(handle: u32, layer_id: &str) -> Result<Clamped<Vec<u8>>, JsError> {
    with_bus(handle, |bus| {
        let layer = bus
            .document
            .layers()
            .iter()
            .find(|l| l.id.to_string() == layer_id)
            .ok_or_else(|| JsError::new("layer not found"))?;
        Ok(Clamped(thumbnail(&layer.pixels)))
    })
}

/// Returns the selection's bounding box as `[x, y, w, h]`, or an empty array
/// when there is no active selection. Used to position the marching-ants
/// overlay (spec §8.5).
#[wasm_bindgen]
pub fn get_selection_bounds(handle: u32) -> Result<Vec<u32>, JsError> {
    with_bus(handle, |bus| {
        match bus
            .document
            .selection
            .as_ref()
            .and_then(|s| s.bounding_box())
        {
            Some(r) => Ok(vec![r.x.max(0) as u32, r.y.max(0) as u32, r.w, r.h]),
            None => Ok(Vec::new()),
        }
    })
}

/// Returns the raw selection coverage mask (one byte per pixel, row-major,
/// canvas-sized), or an empty array when there is no active selection. The UI
/// overlay derives the marching-ants outline from it (spec §8.5).
#[wasm_bindgen]
pub fn get_selection_mask(handle: u32) -> Result<Clamped<Vec<u8>>, JsError> {
    with_bus(handle, |bus| {
        Ok(Clamped(
            bus.document
                .selection
                .as_ref()
                .map(|s| s.data().to_vec())
                .unwrap_or_default(),
        ))
    })
}

/// Converts a core error into a JS error.
fn to_js(e: fineliner_core::DocumentError) -> JsError {
    JsError::new(&e.to_string())
}
