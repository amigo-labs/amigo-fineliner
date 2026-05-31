//! Discrete transform commands: layer flip / 180° rotation and canvas flip /
//! 90°/180° rotation (spec §10.2, §10.3).
//!
//! These are lossless index permutations, so every one is invertible by
//! re-applying its inverse — no pixel snapshot is needed. Layer flips and 180°
//! rotation preserve dimensions and act on the active layer; canvas operations
//! transform every layer in lockstep and (for 90° turns) swap the canvas
//! dimensions. Canvas transforms clear any selection (its geometry no longer
//! matches); the prior selection is restored on undo.

use super::Command;
use crate::document::{CanvasSize, Document};
use crate::error::DocumentError;
use crate::selection::SelectionMask;
use crate::transform::{flip_horizontal, flip_vertical, rotate_180, rotate_90_ccw, rotate_90_cw};
use std::any::Any;

/// A dimension-preserving transform of a single layer (spec §10.2/§10.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerTransform {
    /// Mirror left-to-right.
    FlipHorizontal,
    /// Mirror top-to-bottom.
    FlipVertical,
    /// Rotate 180°.
    Rotate180,
}

impl LayerTransform {
    /// Applies the transform to a layer's pixels in place.
    fn apply_to(self, doc: &mut Document, index: usize) -> Result<(), DocumentError> {
        let len = doc.layers.len();
        let layer = doc
            .layers
            .get_mut(index)
            .ok_or(DocumentError::LayerIndexOutOfBounds { index, len })?;
        layer.pixels = match self {
            LayerTransform::FlipHorizontal => flip_horizontal(&layer.pixels),
            LayerTransform::FlipVertical => flip_vertical(&layer.pixels),
            LayerTransform::Rotate180 => rotate_180(&layer.pixels),
        };
        Ok(())
    }
}

/// Flips or 180°-rotates the active layer (spec §10.2/§10.3). Self-inverse.
pub struct TransformLayer {
    index: usize,
    op: LayerTransform,
}

impl TransformLayer {
    /// Transforms the layer at `index`.
    pub fn new(index: usize, op: LayerTransform) -> Self {
        Self { index, op }
    }
}

impl Command for TransformLayer {
    fn apply(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        self.op.apply_to(doc, self.index)
    }

    fn revert(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        // Flip-H, flip-V and 180° rotation are each their own inverse.
        self.op.apply_to(doc, self.index)
    }

    fn label(&self) -> &str {
        match self.op {
            LayerTransform::FlipHorizontal => "Flip Layer Horizontal",
            LayerTransform::FlipVertical => "Flip Layer Vertical",
            LayerTransform::Rotate180 => "Rotate Layer 180°",
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Flips the whole canvas (all layers) horizontally or vertically (spec §10.2).
/// Dimensions are preserved, so the command is self-inverse.
pub struct FlipCanvas {
    horizontal: bool,
    prev_selection: Option<Option<SelectionMask>>,
}

impl FlipCanvas {
    /// Flips the canvas horizontally (`true`) or vertically (`false`).
    pub fn new(horizontal: bool) -> Self {
        Self {
            horizontal,
            prev_selection: None,
        }
    }

    fn flip_all(&self, doc: &mut Document) {
        for layer in &mut doc.layers {
            layer.pixels = if self.horizontal {
                flip_horizontal(&layer.pixels)
            } else {
                flip_vertical(&layer.pixels)
            };
        }
    }
}

impl Command for FlipCanvas {
    fn apply(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        if self.prev_selection.is_none() {
            self.prev_selection = Some(doc.selection.take());
        } else {
            doc.selection = None;
        }
        self.flip_all(doc);
        Ok(())
    }

    fn revert(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        self.flip_all(doc);
        if let Some(sel) = self.prev_selection.clone() {
            doc.selection = sel;
        }
        Ok(())
    }

    fn label(&self) -> &str {
        if self.horizontal {
            "Flip Canvas Horizontal"
        } else {
            "Flip Canvas Vertical"
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A 90°/180° rotation of the whole canvas (spec §10.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasRotation {
    /// Quarter turn clockwise (swaps dimensions).
    Cw90,
    /// Quarter turn counter-clockwise (swaps dimensions).
    Ccw90,
    /// Half turn (dimensions preserved).
    Rotate180,
}

impl CanvasRotation {
    /// The rotation that undoes this one.
    fn inverse(self) -> CanvasRotation {
        match self {
            CanvasRotation::Cw90 => CanvasRotation::Ccw90,
            CanvasRotation::Ccw90 => CanvasRotation::Cw90,
            CanvasRotation::Rotate180 => CanvasRotation::Rotate180,
        }
    }
}

/// Rotates the whole canvas (all layers) by a quarter or half turn (spec §10.3).
pub struct RotateCanvas {
    rotation: CanvasRotation,
    prev_selection: Option<Option<SelectionMask>>,
}

impl RotateCanvas {
    /// Rotates the canvas by `rotation`.
    pub fn new(rotation: CanvasRotation) -> Self {
        Self {
            rotation,
            prev_selection: None,
        }
    }

    /// Rotates every layer and updates the canvas size accordingly.
    fn rotate_all(doc: &mut Document, rotation: CanvasRotation) -> Result<(), DocumentError> {
        for layer in &mut doc.layers {
            layer.pixels = match rotation {
                CanvasRotation::Cw90 => rotate_90_cw(&layer.pixels),
                CanvasRotation::Ccw90 => rotate_90_ccw(&layer.pixels),
                CanvasRotation::Rotate180 => rotate_180(&layer.pixels),
            };
        }
        if matches!(rotation, CanvasRotation::Cw90 | CanvasRotation::Ccw90) {
            // Quarter turns swap the canvas dimensions.
            let (w, h) = (doc.canvas.width(), doc.canvas.height());
            doc.canvas = CanvasSize::new(h, w)?;
        }
        Ok(())
    }
}

impl Command for RotateCanvas {
    fn apply(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        if self.prev_selection.is_none() {
            self.prev_selection = Some(doc.selection.take());
        } else {
            doc.selection = None;
        }
        Self::rotate_all(doc, self.rotation)
    }

    fn revert(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        Self::rotate_all(doc, self.rotation.inverse())?;
        if let Some(sel) = self.prev_selection.clone() {
            doc.selection = sel;
        }
        Ok(())
    }

    fn label(&self) -> &str {
        match self.rotation {
            CanvasRotation::Cw90 => "Rotate Canvas 90° CW",
            CanvasRotation::Ccw90 => "Rotate Canvas 90° CCW",
            CanvasRotation::Rotate180 => "Rotate Canvas 180°",
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    fn doc_with_corner(w: u32, h: u32) -> Document {
        // A single bright pixel at the top-left corner marks orientation.
        let mut doc = Document::new(w, h).unwrap();
        doc.layers[0].pixels.set_pixel(0, 0, Color::WHITE);
        doc
    }

    #[test]
    fn transform_layer_flip_h_round_trips() {
        let mut doc = doc_with_corner(4, 3);
        let mut cmd = TransformLayer::new(0, LayerTransform::FlipHorizontal);
        cmd.apply(&mut doc).unwrap();
        // The corner pixel moved to the top-right.
        assert_eq!(doc.layers[0].pixels.get_pixel(3, 0), Some(Color::WHITE));
        cmd.revert(&mut doc).unwrap();
        assert_eq!(doc.layers[0].pixels.get_pixel(0, 0), Some(Color::WHITE));
    }

    #[test]
    fn transform_layer_180_round_trips() {
        let mut doc = doc_with_corner(4, 3);
        let mut cmd = TransformLayer::new(0, LayerTransform::Rotate180);
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layers[0].pixels.get_pixel(3, 2), Some(Color::WHITE));
        cmd.revert(&mut doc).unwrap();
        assert_eq!(doc.layers[0].pixels.get_pixel(0, 0), Some(Color::WHITE));
    }

    #[test]
    fn flip_canvas_transforms_all_layers_and_round_trips() {
        let mut doc = doc_with_corner(4, 3);
        doc.add_layer("Top").unwrap();
        doc.layers[1].pixels.set_pixel(0, 0, Color::WHITE);
        let mut cmd = FlipCanvas::new(true);
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layers[0].pixels.get_pixel(3, 0), Some(Color::WHITE));
        assert_eq!(doc.layers[1].pixels.get_pixel(3, 0), Some(Color::WHITE));
        cmd.revert(&mut doc).unwrap();
        assert_eq!(doc.layers[0].pixels.get_pixel(0, 0), Some(Color::WHITE));
        assert_eq!(doc.layers[1].pixels.get_pixel(0, 0), Some(Color::WHITE));
    }

    #[test]
    fn rotate_canvas_90_swaps_dimensions_and_round_trips() {
        let mut doc = doc_with_corner(4, 3);
        let mut cmd = RotateCanvas::new(CanvasRotation::Cw90);
        cmd.apply(&mut doc).unwrap();
        assert_eq!((doc.canvas.width(), doc.canvas.height()), (3, 4));
        assert_eq!(doc.layers[0].pixels.width(), 3);
        cmd.revert(&mut doc).unwrap();
        assert_eq!((doc.canvas.width(), doc.canvas.height()), (4, 3));
        assert_eq!(doc.layers[0].pixels.get_pixel(0, 0), Some(Color::WHITE));
    }

    #[test]
    fn rotate_canvas_clears_selection_and_restores_on_undo() {
        let mut doc = doc_with_corner(4, 3);
        doc.selection = Some(SelectionMask::new_full(4, 3));
        let mut cmd = RotateCanvas::new(CanvasRotation::Cw90);
        cmd.apply(&mut doc).unwrap();
        assert!(doc.selection.is_none());
        cmd.revert(&mut doc).unwrap();
        assert!(doc.selection.is_some());
    }
}
