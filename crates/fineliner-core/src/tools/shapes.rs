//! The Shapes tool (spec §9.2 Shapes).
//!
//! Rasterizes geometric shapes — Line, Rectangle, Rounded Rectangle, Ellipse and
//! regular Polygon — onto the current layer, emitting a single [`SetPixels`] on
//! commit. Each shape is evaluated through a signed-distance field (negative
//! inside), which makes fill, centered stroke, and anti-aliasing fall out of the
//! same per-pixel coverage math. Arrow and dash patterns (spec §9.2) are
//! deferred to a follow-up task.

use super::src_over;
use crate::color::Color;
use crate::command::SetPixels;
use crate::document::Document;
use crate::geometry::{Point, Rect};

/// How a shape's interior and border are painted (spec §9.2 Shapes "Mode").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShapeMode {
    /// Stroke the border only; the interior is untouched.
    #[default]
    Outline,
    /// Fill the interior only; no border.
    Fill,
    /// Fill the interior, then stroke the border on top.
    FillAndOutline,
}

/// A geometric shape defined in canvas space (spec §9.2 Shapes).
///
/// `Rectangle`, `RoundedRectangle` and `Ellipse` are defined by the drag
/// rectangle's two opposite corners (any ordering — the rasterizer normalizes).
/// `Line` is defined by its two endpoints. `Polygon` is a regular N-gon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// A straight segment from `a` to `b` (stroke only; `Fill` modes act as `Outline`).
    Line {
        /// First endpoint.
        a: Point,
        /// Second endpoint.
        b: Point,
    },
    /// An axis-aligned rectangle spanning the corners `a` and `b`.
    Rectangle {
        /// One corner.
        a: Point,
        /// The opposite corner.
        b: Point,
    },
    /// A rectangle (corners `a`, `b`) with rounded corners of the given radius.
    RoundedRectangle {
        /// One corner.
        a: Point,
        /// The opposite corner.
        b: Point,
        /// Corner radius in pixels (clamped to half the shorter side).
        radius: f32,
    },
    /// An ellipse inscribed in the rectangle spanning the corners `a` and `b`.
    Ellipse {
        /// One corner of the bounding rectangle.
        a: Point,
        /// The opposite corner of the bounding rectangle.
        b: Point,
    },
    /// A regular polygon centered at `center` with `sides` vertices (3–100),
    /// circumradius `radius`, rotated `rotation` radians (0 puts a vertex up).
    Polygon {
        /// Center point.
        center: Point,
        /// Circumradius in pixels.
        radius: f32,
        /// Number of sides, clamped to 3–100.
        sides: u32,
        /// Rotation in radians.
        rotation: f32,
    },
}

/// Stroke + fill styling for a [`Shape`] (spec §9.2 Shapes "Options").
#[derive(Debug, Clone, Copy)]
pub struct ShapeStyle {
    /// Outline / Fill / Fill + Outline.
    pub mode: ShapeMode,
    /// Centered stroke width in pixels, clamped to 1–500.
    pub stroke_width: f32,
    /// Border color (alpha is honored).
    pub stroke_color: Color,
    /// Interior color (alpha is honored).
    pub fill_color: Color,
    /// Anti-alias the edges with a 1px coverage ramp.
    pub anti_alias: bool,
}

impl Default for ShapeStyle {
    fn default() -> Self {
        Self {
            mode: ShapeMode::Outline,
            stroke_width: 1.0,
            stroke_color: Color::BLACK,
            fill_color: Color::TRANSPARENT,
            anti_alias: true,
        }
    }
}

/// The Shapes tool — rasterizes one [`Shape`] with a [`ShapeStyle`] (spec §9.2).
#[derive(Debug, Clone, Copy)]
pub struct Shapes {
    /// The shape to draw.
    pub shape: Shape,
    /// Stroke/fill styling.
    pub style: ShapeStyle,
}

impl Shapes {
    /// Creates a Shapes tool for the given shape and style.
    pub fn new(shape: Shape, style: ShapeStyle) -> Self {
        Self { shape, style }
    }

    /// Rasterizes the shape into a [`SetPixels`] command on `layer_index`.
    ///
    /// Returns `None` if the layer index is invalid or the shape's painted
    /// region misses the canvas. The active selection (if any) constrains where
    /// pixels may be written, matching the brush/fill convention. The command's
    /// region is the shape's clamped bounding box; prior pixels are captured for
    /// undo.
    pub fn draw(&self, layer_index: usize, doc: &Document) -> Option<SetPixels> {
        let layer = doc.layers.get(layer_index)?;
        let cw = doc.canvas.width();
        let ch = doc.canvas.height();

        let half_stroke = (self.style.stroke_width.clamp(1.0, 500.0)) / 2.0;
        let region = self.region(half_stroke, cw, ch)?;
        let selection = doc.selection.as_ref();

        let geom = ShapeGeom::build(&self.shape);
        let mut after = layer.pixels.copy_region(region).ok()?;

        for ly in 0..region.h {
            for lx in 0..region.w {
                let cx = region.x + lx as i32;
                let cy = region.y + ly as i32;
                let px = cx as f32 + 0.5;
                let py = cy as f32 + 0.5;

                let (fill_cov, stroke_cov) =
                    geom.coverage(px, py, half_stroke, self.style.mode, self.style.anti_alias);
                if fill_cov <= 0.0 && stroke_cov <= 0.0 {
                    continue;
                }

                // Selection scales every write so unselected pixels are untouched.
                let sel =
                    selection.map_or(1.0, |s| s.coverage(cx as u32, cy as u32) as f32 / 255.0);
                if sel <= 0.0 {
                    continue;
                }

                let dst = after.get_pixel(lx, ly).unwrap_or(Color::TRANSPARENT);
                let mut out = dst;
                if fill_cov > 0.0 {
                    let sa = self.style.fill_color.a as f32 / 255.0 * fill_cov * sel;
                    out = src_over(self.style.fill_color, sa, out);
                }
                if stroke_cov > 0.0 {
                    let sa = self.style.stroke_color.a as f32 / 255.0 * stroke_cov * sel;
                    out = src_over(self.style.stroke_color, sa, out);
                }
                after.set_pixel(lx, ly, out);
            }
        }

        Some(SetPixels::new(layer_index, region, after).with_label(self.shape.label()))
    }

    /// The painted region: the shape's bounding box expanded by the stroke half
    /// width plus 1px for the anti-alias ramp, clamped to the canvas.
    fn region(&self, half_stroke: f32, cw: u32, ch: u32) -> Option<Rect> {
        let (min_x, min_y, max_x, max_y) = self.shape.aabb();
        let pad = half_stroke + 1.0;
        let x0 = ((min_x - pad).floor() as i32).clamp(0, cw as i32);
        let y0 = ((min_y - pad).floor() as i32).clamp(0, ch as i32);
        let x1 = ((max_x + pad).ceil() as i32).clamp(0, cw as i32);
        let y1 = ((max_y + pad).ceil() as i32).clamp(0, ch as i32);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        Some(Rect::new(x0, y0, (x1 - x0) as u32, (y1 - y0) as u32))
    }
}

impl Shape {
    /// Axis-aligned bounding box `(min_x, min_y, max_x, max_y)` of the path
    /// (before stroke expansion).
    fn aabb(&self) -> (f32, f32, f32, f32) {
        match *self {
            Shape::Line { a, b } | Shape::Rectangle { a, b } | Shape::Ellipse { a, b } => {
                (a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y))
            }
            Shape::RoundedRectangle { a, b, .. } => {
                (a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y))
            }
            Shape::Polygon { center, radius, .. } => (
                center.x - radius,
                center.y - radius,
                center.x + radius,
                center.y + radius,
            ),
        }
    }

    /// Undo label for this shape's command.
    fn label(&self) -> &'static str {
        match self {
            Shape::Line { .. } => "Draw Line",
            Shape::Rectangle { .. } => "Draw Rectangle",
            Shape::RoundedRectangle { .. } => "Draw Rounded Rectangle",
            Shape::Ellipse { .. } => "Draw Ellipse",
            Shape::Polygon { .. } => "Draw Polygon",
        }
    }
}

/// Precomputed shape geometry used to evaluate per-pixel coverage.
enum ShapeGeom {
    /// A segment — stroke only (no interior).
    Line { a: Point, b: Point },
    /// A signed-distance area shape (negative inside): rect, rounded rect or ellipse.
    Sdf(SdfShape),
    /// A regular polygon evaluated by its vertices.
    Polygon { verts: Vec<Point> },
}

/// The area-shape variants that share box/ellipse SDF math.
enum SdfShape {
    /// Centered box with half-extents `(hx, hy)`.
    Box { cx: f32, cy: f32, hx: f32, hy: f32 },
    /// Centered rounded box, half-extents `(hx, hy)`, corner radius `r`.
    RoundedBox {
        cx: f32,
        cy: f32,
        hx: f32,
        hy: f32,
        r: f32,
    },
    /// Centered ellipse with semi-axes `(ax, ay)`.
    Ellipse { cx: f32, cy: f32, ax: f32, ay: f32 },
}

impl ShapeGeom {
    fn build(shape: &Shape) -> ShapeGeom {
        match *shape {
            Shape::Line { a, b } => ShapeGeom::Line { a, b },
            Shape::Rectangle { a, b } => {
                let (cx, cy, hx, hy) = center_half(a, b);
                ShapeGeom::Sdf(SdfShape::Box { cx, cy, hx, hy })
            }
            Shape::RoundedRectangle { a, b, radius } => {
                let (cx, cy, hx, hy) = center_half(a, b);
                let r = radius.clamp(0.0, hx.min(hy));
                ShapeGeom::Sdf(SdfShape::RoundedBox { cx, cy, hx, hy, r })
            }
            Shape::Ellipse { a, b } => {
                let (cx, cy, hx, hy) = center_half(a, b);
                ShapeGeom::Sdf(SdfShape::Ellipse {
                    cx,
                    cy,
                    ax: hx.max(1e-3),
                    ay: hy.max(1e-3),
                })
            }
            Shape::Polygon {
                center,
                radius,
                sides,
                rotation,
            } => {
                let n = sides.clamp(3, 100);
                let mut verts = Vec::with_capacity(n as usize);
                for i in 0..n {
                    // Start with a vertex pointing up (-y) and wind clockwise.
                    let ang = rotation - std::f32::consts::FRAC_PI_2
                        + std::f32::consts::TAU * i as f32 / n as f32;
                    verts.push(Point::new(
                        center.x + radius * ang.cos(),
                        center.y + radius * ang.sin(),
                    ));
                }
                ShapeGeom::Polygon { verts }
            }
        }
    }

    /// Returns `(fill_coverage, stroke_coverage)` in `[0,1]` for the pixel center
    /// `(px, py)`, honoring the requested `mode` and anti-aliasing.
    fn coverage(
        &self,
        px: f32,
        py: f32,
        half_stroke: f32,
        mode: ShapeMode,
        aa: bool,
    ) -> (f32, f32) {
        match self {
            // A line has no interior: it always strokes, whatever the mode.
            ShapeGeom::Line { a, b } => {
                let d = dist_segment(px, py, *a, *b);
                (0.0, band_coverage(d, half_stroke, aa))
            }
            ShapeGeom::Sdf(s) => {
                let d = s.sdf(px, py);
                Self::area_coverage(d, half_stroke, mode, aa)
            }
            ShapeGeom::Polygon { verts } => {
                let d = sd_polygon(px, py, verts);
                Self::area_coverage(d, half_stroke, mode, aa)
            }
        }
    }

    /// Splits a signed distance into fill/stroke coverage per `mode`.
    fn area_coverage(d: f32, half_stroke: f32, mode: ShapeMode, aa: bool) -> (f32, f32) {
        let fill = if matches!(mode, ShapeMode::Fill | ShapeMode::FillAndOutline) {
            inside_coverage(d, aa)
        } else {
            0.0
        };
        let stroke = if matches!(mode, ShapeMode::Outline | ShapeMode::FillAndOutline) {
            band_coverage(d, half_stroke, aa)
        } else {
            0.0
        };
        (fill, stroke)
    }
}

impl SdfShape {
    /// Signed distance to the shape (negative inside, positive outside).
    fn sdf(&self, px: f32, py: f32) -> f32 {
        match *self {
            SdfShape::Box { cx, cy, hx, hy } => sd_box(px - cx, py - cy, hx, hy),
            SdfShape::RoundedBox { cx, cy, hx, hy, r } => {
                sd_box(px - cx, py - cy, hx - r, hy - r) - r
            }
            SdfShape::Ellipse { cx, cy, ax, ay } => sd_ellipse(px - cx, py - cy, ax, ay),
        }
    }
}

/// Center and half-extents of the rectangle spanning corners `a` and `b`.
fn center_half(a: Point, b: Point) -> (f32, f32, f32, f32) {
    let cx = (a.x + b.x) / 2.0;
    let cy = (a.y + b.y) / 2.0;
    let hx = (a.x - b.x).abs() / 2.0;
    let hy = (a.y - b.y).abs() / 2.0;
    (cx, cy, hx, hy)
}

/// Fill coverage from a signed distance: `aa` gives a 1px ramp across the edge.
fn inside_coverage(d: f32, aa: bool) -> f32 {
    if aa {
        (0.5 - d).clamp(0.0, 1.0)
    } else if d <= 0.0 {
        1.0
    } else {
        0.0
    }
}

/// Centered-stroke coverage: a band of total width `2 * half` around the path.
fn band_coverage(d: f32, half: f32, aa: bool) -> f32 {
    // Distance outside the band's surface (negative inside the band).
    let edge = d.abs() - half;
    if aa {
        (0.5 - edge).clamp(0.0, 1.0)
    } else if edge <= 0.0 {
        1.0
    } else {
        0.0
    }
}

/// Signed distance from `(px, py)` to the centered box with half-extents `(hx, hy)`.
fn sd_box(px: f32, py: f32, hx: f32, hy: f32) -> f32 {
    let dx = px.abs() - hx;
    let dy = py.abs() - hy;
    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    let inside = dx.max(dy).min(0.0);
    outside + inside
}

/// Approximate signed distance from `(px, py)` to the centered ellipse with
/// semi-axes `(ax, ay)` (Quilez's gradient approximation; negative inside).
fn sd_ellipse(px: f32, py: f32, ax: f32, ay: f32) -> f32 {
    let k1 = ((px / ax).powi(2) + (py / ay).powi(2)).sqrt();
    if k1 == 0.0 {
        // Center: distance is the shorter semi-axis (always interior).
        return -ax.min(ay);
    }
    let k2 = ((px / (ax * ax)).powi(2) + (py / (ay * ay)).powi(2)).sqrt();
    k1 * (k1 - 1.0) / k2
}

/// Distance from `(px, py)` to the segment `a`→`b`.
fn dist_segment(px: f32, py: f32, a: Point, b: Point) -> f32 {
    let ex = b.x - a.x;
    let ey = b.y - a.y;
    let wx = px - a.x;
    let wy = py - a.y;
    let len2 = ex * ex + ey * ey;
    let t = if len2 <= 0.0 {
        0.0
    } else {
        ((wx * ex + wy * ey) / len2).clamp(0.0, 1.0)
    };
    let dx = wx - ex * t;
    let dy = wy - ey * t;
    (dx * dx + dy * dy).sqrt()
}

/// Signed distance from `(px, py)` to the simple polygon `verts` (negative
/// inside). Standard min-edge-distance with a winding-number sign test.
fn sd_polygon(px: f32, py: f32, verts: &[Point]) -> f32 {
    let n = verts.len();
    let mut d = {
        let dx = px - verts[0].x;
        let dy = py - verts[0].y;
        dx * dx + dy * dy
    };
    let mut sign = 1.0_f32;
    let mut j = n - 1;
    for i in 0..n {
        let vi = verts[i];
        let vj = verts[j];
        let ex = vj.x - vi.x;
        let ey = vj.y - vi.y;
        let wx = px - vi.x;
        let wy = py - vi.y;
        let len2 = ex * ex + ey * ey;
        let t = if len2 <= 0.0 {
            0.0
        } else {
            ((wx * ex + wy * ey) / len2).clamp(0.0, 1.0)
        };
        let bx = wx - ex * t;
        let by = wy - ey * t;
        d = d.min(bx * bx + by * by);

        // Crossing-number style sign flip (winding test).
        let c1 = py >= vi.y;
        let c2 = py < vj.y;
        let c3 = ex * wy > ey * wx;
        if (c1 && c2 && c3) || (!c1 && !c2 && !c3) {
            sign = -sign;
        }
        j = i;
    }
    sign * d.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use crate::geometry::Rect;
    use crate::selection::SelectionMask;

    fn fill_style(color: Color) -> ShapeStyle {
        ShapeStyle {
            mode: ShapeMode::Fill,
            fill_color: color,
            anti_alias: false,
            ..ShapeStyle::default()
        }
    }

    fn outline_style(color: Color, width: f32) -> ShapeStyle {
        ShapeStyle {
            mode: ShapeMode::Outline,
            stroke_color: color,
            stroke_width: width,
            anti_alias: false,
            ..ShapeStyle::default()
        }
    }

    #[test]
    fn rectangle_fill_covers_interior_and_corners() {
        let mut doc = Document::new(20, 20).unwrap();
        let shape = Shape::Rectangle {
            a: Point::new(5.0, 5.0),
            b: Point::new(15.0, 15.0),
        };
        let mut cmd = Shapes::new(shape, fill_style(Color::BLACK))
            .draw(0, &doc)
            .unwrap();
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layers[0].pixels.get_pixel(10, 10), Some(Color::BLACK));
        assert_eq!(doc.layers[0].pixels.get_pixel(6, 6), Some(Color::BLACK));
        // Outside the rectangle stays transparent.
        assert_eq!(
            doc.layers[0].pixels.get_pixel(2, 2),
            Some(Color::TRANSPARENT)
        );
    }

    #[test]
    fn rectangle_outline_strokes_border_not_center() {
        let mut doc = Document::new(40, 40).unwrap();
        let shape = Shape::Rectangle {
            a: Point::new(8.0, 8.0),
            b: Point::new(32.0, 32.0),
        };
        let mut cmd = Shapes::new(shape, outline_style(Color::BLACK, 2.0))
            .draw(0, &doc)
            .unwrap();
        cmd.apply(&mut doc).unwrap();
        // A pixel on the left border is painted; the interior center is not.
        assert_eq!(doc.layers[0].pixels.get_pixel(8, 20), Some(Color::BLACK));
        assert_eq!(
            doc.layers[0].pixels.get_pixel(20, 20),
            Some(Color::TRANSPARENT)
        );
    }

    #[test]
    fn ellipse_fill_covers_center_not_corner() {
        let mut doc = Document::new(40, 40).unwrap();
        let shape = Shape::Ellipse {
            a: Point::new(5.0, 5.0),
            b: Point::new(35.0, 35.0),
        };
        let mut cmd = Shapes::new(shape, fill_style(Color::BLACK))
            .draw(0, &doc)
            .unwrap();
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layers[0].pixels.get_pixel(20, 20), Some(Color::BLACK));
        // A bounding-box corner falls outside the ellipse.
        assert_eq!(
            doc.layers[0].pixels.get_pixel(6, 6),
            Some(Color::TRANSPARENT)
        );
    }

    #[test]
    fn line_strokes_along_its_path_regardless_of_mode() {
        let mut doc = Document::new(40, 40).unwrap();
        let shape = Shape::Line {
            a: Point::new(5.0, 5.0),
            b: Point::new(35.0, 35.0),
        };
        // Even in Fill mode a line only strokes.
        let style = ShapeStyle {
            mode: ShapeMode::Fill,
            stroke_color: Color::BLACK,
            stroke_width: 3.0,
            anti_alias: false,
            ..ShapeStyle::default()
        };
        let mut cmd = Shapes::new(shape, style).draw(0, &doc).unwrap();
        cmd.apply(&mut doc).unwrap();
        // A point on the diagonal is painted; an off-diagonal point is not.
        assert_eq!(doc.layers[0].pixels.get_pixel(20, 20), Some(Color::BLACK));
        assert_eq!(
            doc.layers[0].pixels.get_pixel(20, 5),
            Some(Color::TRANSPARENT)
        );
    }

    #[test]
    fn polygon_triangle_fill_covers_centroid() {
        let mut doc = Document::new(40, 40).unwrap();
        let shape = Shape::Polygon {
            center: Point::new(20.0, 20.0),
            radius: 15.0,
            sides: 3,
            rotation: 0.0,
        };
        let mut cmd = Shapes::new(shape, fill_style(Color::BLACK))
            .draw(0, &doc)
            .unwrap();
        cmd.apply(&mut doc).unwrap();
        // The centroid of a triangle is filled.
        assert_eq!(doc.layers[0].pixels.get_pixel(20, 20), Some(Color::BLACK));
        // A bounding-box corner is outside the triangle.
        assert_eq!(
            doc.layers[0].pixels.get_pixel(6, 6),
            Some(Color::TRANSPARENT)
        );
    }

    #[test]
    fn rounded_rectangle_clips_its_corner() {
        let mut doc = Document::new(40, 40).unwrap();
        let shape = Shape::RoundedRectangle {
            a: Point::new(5.0, 5.0),
            b: Point::new(35.0, 35.0),
            radius: 10.0,
        };
        let mut cmd = Shapes::new(shape, fill_style(Color::BLACK))
            .draw(0, &doc)
            .unwrap();
        cmd.apply(&mut doc).unwrap();
        // Center is filled; the extreme corner is rounded away.
        assert_eq!(doc.layers[0].pixels.get_pixel(20, 20), Some(Color::BLACK));
        assert_eq!(
            doc.layers[0].pixels.get_pixel(6, 6),
            Some(Color::TRANSPARENT)
        );
    }

    #[test]
    fn shape_off_canvas_returns_none() {
        let doc = Document::new(20, 20).unwrap();
        let shape = Shape::Rectangle {
            a: Point::new(-50.0, -50.0),
            b: Point::new(-30.0, -30.0),
        };
        assert!(Shapes::new(shape, fill_style(Color::BLACK))
            .draw(0, &doc)
            .is_none());
    }

    #[test]
    fn fill_is_undoable_to_transparent() {
        let mut doc = Document::new(20, 20).unwrap();
        let shape = Shape::Rectangle {
            a: Point::new(5.0, 5.0),
            b: Point::new(15.0, 15.0),
        };
        let mut cmd = Shapes::new(shape, fill_style(Color::BLACK))
            .draw(0, &doc)
            .unwrap();
        cmd.apply(&mut doc).unwrap();
        cmd.revert(&mut doc).unwrap();
        assert_eq!(
            doc.layers[0].pixels.get_pixel(10, 10),
            Some(Color::TRANSPARENT)
        );
    }

    #[test]
    fn fill_respects_active_selection() {
        // Select the left half; fill a rectangle spanning both halves.
        let mut doc = Document::new(20, 20).unwrap();
        doc.selection = Some(SelectionMask::rectangle(20, 20, Rect::new(0, 0, 10, 20)));
        let shape = Shape::Rectangle {
            a: Point::new(2.0, 2.0),
            b: Point::new(18.0, 18.0),
        };
        let mut cmd = Shapes::new(shape, fill_style(Color::BLACK))
            .draw(0, &doc)
            .unwrap();
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layers[0].pixels.get_pixel(5, 10), Some(Color::BLACK));
        assert_eq!(
            doc.layers[0].pixels.get_pixel(15, 10),
            Some(Color::TRANSPARENT)
        );
    }

    #[test]
    fn fill_and_outline_paints_both_colors() {
        let mut doc = Document::new(40, 40).unwrap();
        let shape = Shape::Rectangle {
            a: Point::new(8.0, 8.0),
            b: Point::new(32.0, 32.0),
        };
        let style = ShapeStyle {
            mode: ShapeMode::FillAndOutline,
            fill_color: Color::rgba(255, 0, 0, 255),
            stroke_color: Color::rgba(0, 0, 255, 255),
            stroke_width: 2.0,
            anti_alias: false,
        };
        let mut cmd = Shapes::new(shape, style).draw(0, &doc).unwrap();
        cmd.apply(&mut doc).unwrap();
        // Interior is the fill color; the border is the stroke color.
        assert_eq!(
            doc.layers[0].pixels.get_pixel(20, 20),
            Some(Color::rgba(255, 0, 0, 255))
        );
        assert_eq!(
            doc.layers[0].pixels.get_pixel(8, 20),
            Some(Color::rgba(0, 0, 255, 255))
        );
    }
}
