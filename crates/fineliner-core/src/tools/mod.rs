//! Drawing tools (spec §9).
//!
//! Phase-1 / M5 scope: the **Pencil** with a hard round brush. The full
//! pointer-event [`Tool`] trait with modifiers, cursors and options arrives in
//! M6; here a tool simply turns a stroke (a polyline of points) into a
//! [`SetPixels`] command.

mod eyedropper;
mod fill;

pub use eyedropper::{Eyedropper, SampleSize};
pub use fill::{Fill, FillOptions, SampleSource};

use crate::color::Color;
use crate::command::SetPixels;
use crate::document::{Document, ImageBuffer};
use crate::geometry::{Point, Rect};

/// A hard round brush (spec §9.2 Pencil; soft/flat variants are M6).
#[derive(Debug, Clone, Copy)]
pub struct Brush {
    /// Diameter in pixels, 1–500.
    pub size: u32,
    /// Paint color (alpha is combined with `opacity`).
    pub color: Color,
    /// Stroke opacity in `[0.0, 1.0]`.
    pub opacity: f32,
}

impl Brush {
    /// Creates a brush, clamping size to 1–500 and opacity to `[0, 1]`.
    pub fn new(size: u32, color: Color, opacity: f32) -> Self {
        Self {
            size: size.clamp(1, 500),
            color,
            opacity: opacity.clamp(0.0, 1.0),
        }
    }

    fn radius(&self) -> f32 {
        self.size as f32 / 2.0
    }
}

/// The Pencil tool — freehand hard-round painting (spec §9.2).
#[derive(Debug, Clone, Copy)]
pub struct Pencil {
    /// The active brush.
    pub brush: Brush,
}

impl Pencil {
    /// Creates a pencil with the given brush.
    pub fn new(brush: Brush) -> Self {
        Self { brush }
    }

    /// Rasterizes a stroke over `points` into a [`SetPixels`] command.
    ///
    /// Returns `None` if the stroke does not touch the canvas or `points` is
    /// empty. A single point paints one dab; multiple points paint connected
    /// segments. Painting uses straight-alpha source-over within the stroke's
    /// bounding region; the command captures the prior pixels for undo.
    pub fn stroke(
        &self,
        layer_index: usize,
        points: &[Point],
        doc: &Document,
    ) -> Option<SetPixels> {
        let layer = doc.layers.get(layer_index)?;
        let region = self.stroke_region(points, doc.canvas.width(), doc.canvas.height())?;

        // Start from the layer's current pixels in the region, then paint over.
        let mut after = layer.pixels.copy_region(region).ok()?;
        let ca = self.brush.color.a as f32 / 255.0 * self.brush.opacity;
        if ca <= 0.0 {
            return None;
        }

        if points.len() == 1 {
            self.stamp(&mut after, region, points[0], ca);
        } else {
            for pair in points.windows(2) {
                self.stamp_segment(&mut after, region, pair[0], pair[1], ca);
            }
        }

        Some(SetPixels::new(layer_index, region, after).with_label("Pencil Stroke"))
    }

    /// Bounding region of the stroke, clamped to the canvas. `None` if off-canvas.
    fn stroke_region(&self, points: &[Point], cw: u32, ch: u32) -> Option<Rect> {
        if points.is_empty() {
            return None;
        }
        let r = self.brush.radius() + 1.0;
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for p in points {
            min_x = min_x.min(p.x - r);
            min_y = min_y.min(p.y - r);
            max_x = max_x.max(p.x + r);
            max_y = max_y.max(p.y + r);
        }
        let x0 = (min_x.floor() as i32).clamp(0, cw as i32);
        let y0 = (min_y.floor() as i32).clamp(0, ch as i32);
        let x1 = (max_x.ceil() as i32).clamp(0, cw as i32);
        let y1 = (max_y.ceil() as i32).clamp(0, ch as i32);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        Some(Rect::new(x0, y0, (x1 - x0) as u32, (y1 - y0) as u32))
    }

    /// Paints one round dab centered at canvas-space `center` into `buf`
    /// (whose origin is `region`'s top-left).
    fn stamp(&self, buf: &mut ImageBuffer, region: Rect, center: Point, ca: f32) {
        let radius = self.brush.radius();
        let r2 = radius * radius;
        let cx0 = (center.x - radius).floor() as i32;
        let cy0 = (center.y - radius).floor() as i32;
        let cx1 = (center.x + radius).ceil() as i32;
        let cy1 = (center.y + radius).ceil() as i32;
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                let dx = cx as f32 + 0.5 - center.x;
                let dy = cy as f32 + 0.5 - center.y;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let lx = cx - region.x;
                let ly = cy - region.y;
                if lx < 0 || ly < 0 || lx >= region.w as i32 || ly >= region.h as i32 {
                    continue;
                }
                let dst = buf
                    .get_pixel(lx as u32, ly as u32)
                    .unwrap_or(Color::TRANSPARENT);
                buf.set_pixel(lx as u32, ly as u32, src_over(self.brush.color, ca, dst));
            }
        }
    }

    /// Stamps dabs evenly along the segment `a`→`b`.
    fn stamp_segment(&self, buf: &mut ImageBuffer, region: Rect, a: Point, b: Point, ca: f32) {
        let dist = a.distance(b);
        // Spacing of a quarter brush size keeps strokes solid without overdraw blowup.
        let spacing = (self.brush.size as f32 * 0.25).max(1.0);
        let steps = (dist / spacing).ceil() as i32;
        if steps == 0 {
            self.stamp(buf, region, a, ca);
            return;
        }
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let p = Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
            self.stamp(buf, region, p, ca);
        }
    }
}

/// Straight-alpha source-over of `src` (with effective alpha `sa`) onto `dst`.
fn src_over(src: Color, sa: f32, dst: Color) -> Color {
    let da = dst.a as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        return Color::TRANSPARENT;
    }
    let mix = |s: u8, d: u8| -> u8 {
        let s = s as f32 / 255.0;
        let d = d as f32 / 255.0;
        let v = (s * sa + d * da * (1.0 - sa)) / out_a;
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    Color::rgba(
        mix(src.r, dst.r),
        mix(src.g, dst.g),
        mix(src.b, dst.b),
        (out_a.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;

    fn black_pencil(size: u32) -> Pencil {
        Pencil::new(Brush::new(size, Color::BLACK, 1.0))
    }

    #[test]
    fn single_dab_paints_center_pixel() {
        let mut doc = Document::new(20, 20).unwrap();
        let mut cmd = black_pencil(6)
            .stroke(0, &[Point::new(10.0, 10.0)], &doc)
            .unwrap();
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layers[0].pixels.get_pixel(10, 10), Some(Color::BLACK));
    }

    #[test]
    fn stroke_off_canvas_returns_none() {
        let doc = Document::new(10, 10).unwrap();
        assert!(black_pencil(4)
            .stroke(0, &[Point::new(-50.0, -50.0)], &doc)
            .is_none());
    }

    #[test]
    fn stroke_paints_a_connected_line() {
        let mut doc = Document::new(40, 40).unwrap();
        let mut cmd = black_pencil(4)
            .stroke(0, &[Point::new(5.0, 5.0), Point::new(35.0, 5.0)], &doc)
            .unwrap();
        cmd.apply(&mut doc).unwrap();
        // A point midway along the line is painted.
        assert_eq!(doc.layers[0].pixels.get_pixel(20, 5), Some(Color::BLACK));
    }

    #[test]
    fn stroke_is_undoable_to_transparent() {
        let mut doc = Document::new(20, 20).unwrap();
        let mut cmd = black_pencil(8)
            .stroke(0, &[Point::new(10.0, 10.0)], &doc)
            .unwrap();
        cmd.apply(&mut doc).unwrap();
        cmd.revert(&mut doc).unwrap();
        assert_eq!(
            doc.layers[0].pixels.get_pixel(10, 10),
            Some(Color::TRANSPARENT)
        );
    }

    #[test]
    fn half_opacity_over_transparent_yields_half_alpha() {
        let mut doc = Document::new(10, 10).unwrap();
        let pencil = Pencil::new(Brush::new(6, Color::BLACK, 0.5));
        let mut cmd = pencil.stroke(0, &[Point::new(5.0, 5.0)], &doc).unwrap();
        cmd.apply(&mut doc).unwrap();
        let a = doc.layers[0].pixels.get_pixel(5, 5).unwrap().a;
        assert!((126..=130).contains(&a), "got {a}");
    }
}
