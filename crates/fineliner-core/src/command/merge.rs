//! Layer-flattening commands: `MergeDown`, `MergeVisible`, `FlattenImage`
//! (spec §5.2 / §7.3).
//!
//! Each bakes several layers into one. The result is a plain raster layer
//! (Normal blend, opacity 1.0, visible) since its pixels already encode the
//! composite of its inputs. For undo, the full prior layer stack and active
//! index are snapshotted and restored verbatim — structural merges are
//! infrequent, so the clone cost is acceptable (spec §7.1).

use super::Command;
use crate::color::Color;
use crate::document::{Document, Layer};
use crate::error::DocumentError;
use crate::render::{compose, compose_over};
use std::any::Any;

/// The prior layer stack, captured on first apply for exact reversal.
struct Snapshot {
    layers: Vec<Layer>,
    active: usize,
}

/// Captures the current layer stack and active index.
fn snapshot(doc: &Document) -> Snapshot {
    Snapshot {
        layers: doc.layers().to_vec(),
        active: doc.active_layer_index(),
    }
}

/// Restores a previously captured layer stack.
fn restore(doc: &mut Document, snap: Snapshot) -> Result<(), DocumentError> {
    doc.layers = snap.layers;
    doc.set_active_layer(snap.active)
}

/// Merges the layer at `index` onto the layer directly below it.
///
/// The two layers are composited as displayed (respecting opacity and blend
/// mode) into a single layer that takes the lower layer's name and id. Errors
/// if `index` is 0 (no layer below) or out of range.
pub struct MergeDown {
    index: usize,
    before: Option<Snapshot>,
}

impl MergeDown {
    /// Merges the layer at `index` down onto `index - 1`.
    pub fn at(index: usize) -> Self {
        Self {
            index,
            before: None,
        }
    }
}

impl Command for MergeDown {
    fn apply(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        let len = doc.layer_count();
        if self.index == 0 || self.index >= len {
            return Err(DocumentError::LayerIndexOutOfBounds {
                index: self.index,
                len,
            });
        }
        self.before = Some(snapshot(doc));

        let lower = doc.layers()[self.index - 1].clone();
        let upper = doc.layers()[self.index].clone();
        let merged_pixels = compose(&[lower.clone(), upper]);
        let mut merged = Layer::from_pixels(lower.name.clone(), merged_pixels);
        merged.id = lower.id; // the merged layer "is" the lower layer

        doc.layers.remove(self.index);
        doc.layers[self.index - 1] = merged;
        doc.set_active_layer(self.index - 1)
    }

    fn revert(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        let snap = self.before.take().ok_or(DocumentError::RegionOutOfBounds)?;
        restore(doc, snap)
    }

    fn label(&self) -> &str {
        "Merge Down"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Flattens all visible layers into a single layer, leaving hidden layers
/// untouched.
///
/// The merged layer is placed at the position of the lowest visible layer. A
/// no-op if no layer is visible.
pub struct MergeVisible {
    before: Option<Snapshot>,
}

impl MergeVisible {
    /// Creates the command.
    pub fn new() -> Self {
        Self { before: None }
    }
}

impl Default for MergeVisible {
    fn default() -> Self {
        Self::new()
    }
}

impl Command for MergeVisible {
    fn apply(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        let visible: Vec<usize> = doc
            .layers()
            .iter()
            .enumerate()
            .filter(|(_, l)| l.visible)
            .map(|(i, _)| i)
            .collect();
        if visible.is_empty() {
            return Ok(()); // nothing visible to merge
        }
        self.before = Some(snapshot(doc));

        let lowest = visible[0];
        let visible_layers: Vec<Layer> = visible.iter().map(|&i| doc.layers()[i].clone()).collect();
        let merged = Layer::from_pixels("Merged", compose(&visible_layers));

        // Remove visible layers high-to-low so earlier indices stay valid, then
        // insert the merged layer where the lowest visible layer was. All layers
        // below `lowest` are hidden (it is the minimum visible index), so they
        // keep positions 0..lowest and the insert index is exactly `lowest`.
        for &i in visible.iter().rev() {
            doc.layers.remove(i);
        }
        doc.layers.insert(lowest, merged);
        doc.set_active_layer(lowest)
    }

    fn revert(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        match self.before.take() {
            Some(snap) => restore(doc, snap),
            None => Ok(()), // apply was a no-op
        }
    }

    fn label(&self) -> &str {
        "Merge Visible"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Flattens every layer onto an opaque white background, producing a single
/// fully opaque "Background" layer (spec §5.2).
pub struct FlattenImage {
    before: Option<Snapshot>,
}

impl FlattenImage {
    /// Creates the command.
    pub fn new() -> Self {
        Self { before: None }
    }
}

impl Default for FlattenImage {
    fn default() -> Self {
        Self::new()
    }
}

impl Command for FlattenImage {
    fn apply(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        self.before = Some(snapshot(doc));
        let flattened = compose_over(doc.layers(), Color::WHITE);
        doc.layers.clear();
        doc.layers.push(Layer::from_pixels("Background", flattened));
        doc.set_active_layer(0)
    }

    fn revert(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        let snap = self.before.take().ok_or(DocumentError::RegionOutOfBounds)?;
        restore(doc, snap)
    }

    fn label(&self) -> &str {
        "Flatten Image"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(doc: &mut Document, index: usize, c: Color) {
        let buf = &mut doc.layers[index].pixels;
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                buf.set_pixel(x, y, c);
            }
        }
    }

    fn composites_equal(a: &Document, b: &Document) -> bool {
        compose(a.layers()).data() == compose(b.layers()).data()
    }

    #[test]
    fn merge_down_combines_two_layers_into_one() {
        let mut doc = Document::new(2, 2).unwrap();
        solid(&mut doc, 0, Color::rgba(0, 0, 255, 255)); // blue bottom
        doc.add_layer("Top").unwrap();
        solid(&mut doc, 1, Color::rgba(255, 0, 0, 255)); // opaque red top

        let mut cmd = MergeDown::at(1);
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layer_count(), 1);
        // Opaque red over blue → red.
        assert_eq!(
            doc.layers()[0].pixels.get_pixel(0, 0),
            Some(Color::rgba(255, 0, 0, 255))
        );
        assert_eq!(doc.active_layer_index(), 0);
    }

    #[test]
    fn merge_down_round_trip_restores_stack() {
        let mut doc = Document::new(2, 2).unwrap();
        doc.add_layer("Top").unwrap();
        let before = compose(doc.layers()).into_raw();
        let mut cmd = MergeDown::at(1);
        cmd.apply(&mut doc).unwrap();
        cmd.revert(&mut doc).unwrap();
        assert_eq!(doc.layer_count(), 2);
        assert_eq!(compose(doc.layers()).into_raw(), before);
    }

    #[test]
    fn merge_down_rejects_bottom_layer() {
        let mut doc = Document::new(2, 2).unwrap();
        assert!(MergeDown::at(0).apply(&mut doc).is_err());
    }

    #[test]
    fn merge_visible_preserves_composite_and_keeps_hidden_layers() {
        let mut doc = Document::new(2, 2).unwrap();
        solid(&mut doc, 0, Color::rgba(0, 0, 255, 255));
        doc.add_layer("Hidden").unwrap();
        solid(&mut doc, 1, Color::rgba(0, 255, 0, 255));
        doc.layers[1].visible = false; // hidden middle layer
        doc.add_layer("Top").unwrap();
        solid(&mut doc, 2, Color::rgba(255, 0, 0, 128)); // 50% red top

        let expected = doc.clone();
        let mut cmd = MergeVisible::new();
        cmd.apply(&mut doc).unwrap();
        // Two visible layers merged into one; the hidden layer survives.
        assert_eq!(doc.layer_count(), 2);
        assert!(doc.layers().iter().any(|l| !l.visible));
        assert!(composites_equal(&doc, &expected));
    }

    #[test]
    fn merge_visible_no_op_when_all_hidden() {
        let mut doc = Document::new(2, 2).unwrap();
        doc.layers[0].visible = false;
        let mut cmd = MergeVisible::new();
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layer_count(), 1);
    }

    #[test]
    fn flatten_produces_single_opaque_layer() {
        let mut doc = Document::new(2, 2).unwrap();
        solid(&mut doc, 0, Color::rgba(255, 0, 0, 128)); // 50% red over white
        let mut cmd = FlattenImage::new();
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layer_count(), 1);
        // Every pixel is fully opaque after flattening onto white.
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(doc.layers()[0].pixels.get_pixel(x, y).unwrap().a, 255);
            }
        }
    }

    #[test]
    fn flatten_round_trip_restores_stack() {
        let mut doc = Document::new(2, 2).unwrap();
        doc.add_layer("Top").unwrap();
        solid(&mut doc, 1, Color::rgba(0, 255, 0, 200));
        let before = compose(doc.layers()).into_raw();
        let mut cmd = FlattenImage::new();
        cmd.apply(&mut doc).unwrap();
        cmd.revert(&mut doc).unwrap();
        assert_eq!(doc.layer_count(), 2);
        assert_eq!(compose(doc.layers()).into_raw(), before);
    }

    #[test]
    fn flatten_composite_matches_flattened_layer() {
        // A flattened doc's composite equals its single opaque layer's pixels.
        let mut doc = Document::new(2, 2).unwrap();
        solid(&mut doc, 0, Color::rgba(128, 64, 32, 180));
        let mut cmd = FlattenImage::new();
        cmd.apply(&mut doc).unwrap();
        assert_eq!(compose(doc.layers()).data(), doc.layers()[0].pixels.data());
    }
}
