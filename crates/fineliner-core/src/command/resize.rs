//! The `ResizeCanvas` command (spec §7.3, §10.4).

use super::Command;
use crate::document::{CanvasSize, Document, ImageBuffer};
use crate::error::DocumentError;
use std::any::Any;

/// Resizes the canvas, cropping or extending every layer.
///
/// Phase 1 anchors content at the top-left; the 9-grid anchor arrives in M9
/// (spec §10.4). The previous canvas size and layer pixels are captured on
/// first apply so the operation is fully reversible.
pub struct ResizeCanvas {
    new_width: u32,
    new_height: u32,
    prev: Option<(CanvasSize, Vec<ImageBuffer>)>,
}

impl ResizeCanvas {
    /// Resizes the canvas to `width` × `height`.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            new_width: width,
            new_height: height,
            prev: None,
        }
    }
}

/// Copies `src` into a fresh buffer of `w` × `h`, anchored at the top-left.
fn resized(src: &ImageBuffer, w: u32, h: u32) -> ImageBuffer {
    let mut out = ImageBuffer::new_transparent(w, h);
    let copy_w = w.min(src.width());
    let copy_h = h.min(src.height());
    for y in 0..copy_h {
        for x in 0..copy_w {
            if let Some(c) = src.get_pixel(x, y) {
                out.set_pixel(x, y, c);
            }
        }
    }
    out
}

impl Command for ResizeCanvas {
    fn apply(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        let new_canvas = CanvasSize::new(self.new_width, self.new_height)?;
        if self.prev.is_none() {
            let buffers = doc.layers.iter().map(|l| l.pixels.clone()).collect();
            self.prev = Some((doc.canvas, buffers));
        }
        for layer in &mut doc.layers {
            layer.pixels = resized(&layer.pixels, self.new_width, self.new_height);
        }
        doc.canvas = new_canvas;
        Ok(())
    }

    fn revert(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        let (canvas, buffers) = self.prev.as_ref().ok_or(DocumentError::RegionOutOfBounds)?;
        if buffers.len() != doc.layers.len() {
            return Err(DocumentError::RegionOutOfBounds);
        }
        for (layer, buf) in doc.layers.iter_mut().zip(buffers.iter()) {
            layer.pixels = buf.clone();
        }
        doc.canvas = *canvas;
        Ok(())
    }

    fn label(&self) -> &str {
        "Resize Canvas"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    #[test]
    fn resize_smaller_then_revert_restores_pixels() {
        let mut doc = Document::new(8, 8).unwrap();
        doc.layers[0].pixels.set_pixel(6, 6, Color::WHITE);
        let mut cmd = ResizeCanvas::new(4, 4);
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.canvas.width(), 4);
        // Pixel at (6,6) was cropped away.
        assert_eq!(doc.layers[0].pixels.get_pixel(6, 6), None);

        cmd.revert(&mut doc).unwrap();
        assert_eq!(doc.canvas.width(), 8);
        assert_eq!(doc.layers[0].pixels.get_pixel(6, 6), Some(Color::WHITE));
    }

    #[test]
    fn resize_larger_preserves_existing_pixels() {
        let mut doc = Document::new(4, 4).unwrap();
        doc.layers[0].pixels.set_pixel(1, 1, Color::WHITE);
        let mut cmd = ResizeCanvas::new(8, 8);
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.canvas.width(), 8);
        assert_eq!(doc.layers[0].pixels.get_pixel(1, 1), Some(Color::WHITE));
        assert_eq!(
            doc.layers[0].pixels.get_pixel(5, 5),
            Some(Color::TRANSPARENT)
        );
    }

    #[test]
    fn resize_to_invalid_size_errors() {
        let mut doc = Document::new(4, 4).unwrap();
        let mut cmd = ResizeCanvas::new(0, 4);
        assert!(cmd.apply(&mut doc).is_err());
    }
}
