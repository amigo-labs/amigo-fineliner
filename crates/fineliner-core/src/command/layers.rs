//! Layer-structure commands: `AddLayer`, `RemoveLayer`, `MoveLayer` (spec §7.3).

use super::Command;
use crate::document::{Document, Layer};
use crate::error::DocumentError;
use std::any::Any;

/// Inserts a new transparent layer at a given index.
pub struct AddLayer {
    index: usize,
    /// The inserted layer, captured on first apply so redo restores it exactly.
    layer: Option<Layer>,
    prev_active: usize,
}

impl AddLayer {
    /// Adds a layer directly above layer `active`.
    pub fn above(active: usize) -> Self {
        Self {
            index: active + 1,
            layer: None,
            prev_active: active,
        }
    }
}

impl Command for AddLayer {
    fn apply(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        self.prev_active = doc.active_layer_index();
        let layer = match self.layer.take() {
            Some(l) => l,
            None => Layer::new_transparent(
                format!("Layer {}", doc.layers.len() + 1),
                doc.canvas.width(),
                doc.canvas.height(),
            ),
        };
        let restored = layer.clone();
        doc.insert_layer(self.index, layer)?;
        self.layer = Some(restored);
        Ok(())
    }

    fn revert(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        doc.remove_layer(self.index)?;
        doc.set_active_layer(self.prev_active.min(doc.layers.len() - 1))
    }

    fn label(&self) -> &str {
        "Add Layer"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Removes the layer at `index`, remembering it for undo.
pub struct RemoveLayer {
    index: usize,
    removed: Option<Layer>,
    prev_active: usize,
}

impl RemoveLayer {
    /// Removes the layer at `index`.
    pub fn at(index: usize) -> Self {
        Self {
            index,
            removed: None,
            prev_active: 0,
        }
    }
}

impl Command for RemoveLayer {
    fn apply(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        self.prev_active = doc.active_layer_index();
        self.removed = Some(doc.remove_layer(self.index)?);
        Ok(())
    }

    fn revert(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        let layer = self
            .removed
            .take()
            .ok_or(DocumentError::RegionOutOfBounds)?;
        doc.insert_layer(self.index, layer)?;
        doc.set_active_layer(self.prev_active)
    }

    fn label(&self) -> &str {
        "Delete Layer"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Reorders a layer from one index to another.
pub struct MoveLayer {
    from: usize,
    to: usize,
}

impl MoveLayer {
    /// Moves the layer at `from` to `to`.
    pub fn new(from: usize, to: usize) -> Self {
        Self { from, to }
    }
}

impl Command for MoveLayer {
    fn apply(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        doc.move_layer(self.from, self.to)
    }

    fn revert(&mut self, doc: &mut Document) -> Result<(), DocumentError> {
        doc.move_layer(self.to, self.from)
    }

    fn label(&self) -> &str {
        "Move Layer"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_layer_round_trip() {
        let mut doc = Document::new(8, 8).unwrap();
        let mut cmd = AddLayer::above(0);
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layers.len(), 2);
        cmd.revert(&mut doc).unwrap();
        assert_eq!(doc.layers.len(), 1);
    }

    #[test]
    fn remove_layer_round_trip_preserves_layer_identity() {
        let mut doc = Document::new(8, 8).unwrap();
        doc.add_layer("Layer 2").unwrap();
        let target_id = doc.layers[1].id;
        let mut cmd = RemoveLayer::at(1);
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layers.len(), 1);
        cmd.revert(&mut doc).unwrap();
        assert_eq!(doc.layers.len(), 2);
        assert_eq!(doc.layers[1].id, target_id);
    }

    #[test]
    fn move_layer_round_trip() {
        let mut doc = Document::new(8, 8).unwrap();
        doc.add_layer("Layer 2").unwrap();
        let bottom_id = doc.layers[0].id;
        let mut cmd = MoveLayer::new(0, 1);
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layers[1].id, bottom_id);
        cmd.revert(&mut doc).unwrap();
        assert_eq!(doc.layers[0].id, bottom_id);
    }
}
