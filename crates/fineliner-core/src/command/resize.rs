//! The `ResizeCanvas` command (spec §7.3, §10.4).

use super::Command;
use crate::document::{CanvasSize, Document, ImageBuffer};
use crate::error::DocumentError;
use std::any::Any;

/// Where existing content is anchored when the canvas is resized (spec §10.4).
///
/// The 9-grid positions; `TopLeft` is the default and matches the original
/// Phase-1 behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    /// Anchor at the top-left corner (default).
    #[default]
    TopLeft,
    /// Anchor at the top edge, horizontally centered.
    TopCenter,
    /// Anchor at the top-right corner.
    TopRight,
    /// Anchor at the left edge, vertically centered.
    CenterLeft,
    /// Anchor at the center.
    Center,
    /// Anchor at the right edge, vertically centered.
    CenterRight,
    /// Anchor at the bottom-left corner.
    BottomLeft,
    /// Anchor at the bottom edge, horizontally centered.
    BottomCenter,
    /// Anchor at the bottom-right corner.
    BottomRight,
}

/// Position along one axis: start, center, or end.
#[derive(Clone, Copy)]
enum AxisPos {
    Start,
    Center,
    End,
}

impl Anchor {
    /// The (horizontal, vertical) axis positions for this anchor.
    fn axes(self) -> (AxisPos, AxisPos) {
        use AxisPos::{Center, End, Start};
        match self {
            Anchor::TopLeft => (Start, Start),
            Anchor::TopCenter => (Center, Start),
            Anchor::TopRight => (End, Start),
            Anchor::CenterLeft => (Start, Center),
            Anchor::Center => (Center, Center),
            Anchor::CenterRight => (End, Center),
            Anchor::BottomLeft => (Start, End),
            Anchor::BottomCenter => (Center, End),
            Anchor::BottomRight => (End, End),
        }
    }
}

/// Offset of the old origin within the new extent for one axis.
fn axis_offset(new: u32, old: u32, pos: AxisPos) -> i32 {
    let delta = new as i32 - old as i32;
    match pos {
        AxisPos::Start => 0,
        AxisPos::Center => delta / 2,
        AxisPos::End => delta,
    }
}

/// Resizes the canvas, cropping or extending every layer.
///
/// Content is placed according to the chosen [`Anchor`] (spec §10.4). The
/// previous canvas size and layer pixels are captured on first apply so the
/// operation is fully reversible.
pub struct ResizeCanvas {
    new_width: u32,
    new_height: u32,
    anchor: Anchor,
    prev: Option<(CanvasSize, Vec<ImageBuffer>)>,
}

impl ResizeCanvas {
    /// Resizes the canvas to `width` × `height`, anchored at the top-left.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            new_width: width,
            new_height: height,
            anchor: Anchor::TopLeft,
            prev: None,
        }
    }

    /// Sets the anchor that positions existing content (spec §10.4).
    pub fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }
}

/// Copies `src` into a fresh `w` × `h` buffer, offset by `(dx, dy)`.
fn resized(src: &ImageBuffer, w: u32, h: u32, dx: i32, dy: i32) -> ImageBuffer {
    let mut out = ImageBuffer::new_transparent(w, h);
    for sy in 0..src.height() {
        let ty = sy as i32 + dy;
        if ty < 0 || ty >= h as i32 {
            continue;
        }
        for sx in 0..src.width() {
            let tx = sx as i32 + dx;
            if tx < 0 || tx >= w as i32 {
                continue;
            }
            if let Some(c) = src.get_pixel(sx, sy) {
                out.set_pixel(tx as u32, ty as u32, c);
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
        let (hpos, vpos) = self.anchor.axes();
        let dx = axis_offset(self.new_width, doc.canvas.width(), hpos);
        let dy = axis_offset(self.new_height, doc.canvas.height(), vpos);
        for layer in &mut doc.layers {
            layer.pixels = resized(&layer.pixels, self.new_width, self.new_height, dx, dy);
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

    #[test]
    fn resize_with_center_anchor_centers_content() {
        // Grow 2×2 → 4×4 anchored at center: the old origin shifts by (1, 1).
        let mut doc = Document::new(2, 2).unwrap();
        doc.layers[0].pixels.set_pixel(0, 0, Color::WHITE);
        let mut cmd = ResizeCanvas::new(4, 4).with_anchor(Anchor::Center);
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layers[0].pixels.get_pixel(1, 1), Some(Color::WHITE));
        assert_eq!(
            doc.layers[0].pixels.get_pixel(0, 0),
            Some(Color::TRANSPARENT)
        );
        cmd.revert(&mut doc).unwrap();
        assert_eq!(doc.layers[0].pixels.get_pixel(0, 0), Some(Color::WHITE));
    }

    #[test]
    fn resize_with_bottom_right_anchor_keeps_corner() {
        // Grow 2×2 → 3×3 anchored bottom-right: old (1,1) lands at (2,2).
        let mut doc = Document::new(2, 2).unwrap();
        doc.layers[0].pixels.set_pixel(1, 1, Color::WHITE);
        let mut cmd = ResizeCanvas::new(3, 3).with_anchor(Anchor::BottomRight);
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layers[0].pixels.get_pixel(2, 2), Some(Color::WHITE));
    }
}
