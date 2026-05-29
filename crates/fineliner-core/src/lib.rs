//! # fineliner-core
//!
//! Pure logic core for the Fineliner image editor: document model, color and
//! blend modes, geometry, commands/undo, compositing, codecs, and tools.
//!
//! This crate has **no platform dependencies** — no async runtime, no
//! `wasm-bindgen`, no Tauri. It is `std`-only and safe to compile to
//! `wasm32-unknown-unknown`. See `CLAUDE.md` §5.1.

#![forbid(unsafe_code)]

pub mod codec;
pub mod color;
pub mod command;
pub mod document;
pub mod error;
pub mod geometry;
pub mod render;
pub mod tools;

pub use color::{BlendMode, Color};
pub use command::{
    AddLayer, Command, CommandBus, MoveLayer, RemoveLayer, ResizeCanvas, SetPixels, UndoStack,
};
pub use document::{
    CanvasSize, ColorProfile, Document, DocumentMetadata, ImageBuffer, Layer, LayerKind,
    MAX_CANVAS_DIM, MAX_LAYERS, MIN_CANVAS_DIM,
};
pub use error::DocumentError;
pub use geometry::{Point, Rect, Size};
pub use render::compose;
pub use tools::{
    Brush, BrushShape, Eraser, EraserMode, Eyedropper, Fill, FillOptions, Move, Pencil, SampleSize,
    SampleSource,
};
