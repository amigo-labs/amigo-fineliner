# STATUS

## Current state — M1–M6 complete (basic tool suite)

The foundational pipeline plus the M6 tool suite are implemented and verified:
open/export, undo/redo, and Pencil (hard/soft/flat), Eraser, Fill, Eyedropper
and Move tools wired end-to-end through WASM into the Svelte UI.

### Milestones done

- [x] **M1 — core skeleton.** `geometry` (Point/Size/Rect), `color` (Color +
  12-variant BlendMode), `document` (ImageBuffer, CanvasSize, Layer, Document
  with single-layer invariant and 999-layer limit), `error`.
- [x] **M2 — commands + undo.** `Command` trait, `UndoStack` (10–500 capacity,
  redo-branch discard), `CommandBus`; `SetPixels` (lazy before-capture + stroke
  merge), `AddLayer`, `RemoveLayer`, `MoveLayer`, `ResizeCanvas`.
- [x] **M3 — compositing.** `compose()` in linear light, all 12 blend modes
  with reference tests, opacity, determinism proptests.
- [x] **M4 — codecs.** `decode` (PNG/JPEG/WebP/BMP/GIF/TIFF), encode PNG/JPEG/
  BMP/WebP(lossless, ADR-007); round-trip + edge-case tests.
- [x] **M5 — WASM + UI.** `fineliner-wasm` bindings (§17 API); Svelte 5 + Vite +
  Tailwind 4 UI with Pencil, open/export PNG, undo/redo.

### Verification

- `cargo test --workspace` green; `cargo clippy --workspace -- -D warnings` clean.
- `cargo test -p fineliner-core --test pencil_round_trip` proves the M5 exit
  criterion (decode → paint → compose → encode → decode is pixel-correct).
- `wasm-pack build --target web` succeeds; node smoke test through the WASM
  boundary passes (composite size, valid PNG signature, undo/redo).
- `pnpm check` and `pnpm build` succeed.
- **Not yet done by a human:** visual browser run. To verify:
  `cd crates/fineliner-wasm && wasm-pack build --target web --release --out-dir ../../ui/src/lib/wasm/pkg`,
  then `cd ui && pnpm install && pnpm dev`, open an image, paint, export.

## M6 — basic tool suite (complete)

Milestone was L/XL, split into S/M core tasks (pure logic, test-first) plus the
UI-wiring task. Order:

- [x] **Fill / Paint Bucket** (`tools/fill.rs`) — BFS flood-fill, tolerance
  (Euclidean RGBA8), contiguous + all-pixels modes, sample current layer /
  all layers. Emits `SetPixels`. Spec §9.2 Fill.
- [x] **Eyedropper** (`tools/eyedropper.rs`) — sample current layer / composite
  (reuses `SampleSource`), size 1×1 / 3×3 / 5×5 / 11×11 / 31×31 avg, edge-clamped
  neighborhood. Returns a `Color`, no command. Spec §9.2 Eyedropper.
- [x] **Brush engine generalization** — `BrushShape` (HardRound/SoftRound/Flat)
  + hardness with linear edge falloff; reusable `Brush::rasterize(op)` extracted
  from Pencil. Hard round unchanged (default). Spec §9.2 Pencil brush shapes.
  Textured/custom tip deferred to Phase 2 (spec §9.2).
- [x] **Eraser** (`tools/eraser.rs`) — ToTransparent (reduces alpha) and
  ToBackground (composites bg color) on the shared rasterizer. Spec §9.2 Eraser.
- [x] **Move** (`tools/move_tool.rs`) — integer translate of a layer's pixels,
  drops off-canvas content, clears the vacated area; emits `SetPixels` over the
  whole layer. Auto-select / ghost / arrow-nudge are UI concerns. Spec §9.2 Move.
- [x] **UI + WASM wiring** — `fineliner-wasm` gained `eraser_stroke`,
  `fill_bucket`, `translate_layer` commands, optional brush shape/hardness on
  `pencil_stroke`, and a `pick_color` query; the Svelte UI gained a tool-aware
  pointer dispatcher, toolbar buttons, a per-tool options bar, and B/E/G/I/V
  shortcuts. Spec §16.2, §16.3.

### Verification (M6)

- `cargo test --workspace` green (98 core tests); `cargo clippy --workspace
  --all-targets -- -D warnings` clean.
- `pnpm check`, `pnpm lint`, `pnpm build` all green (svelte-check 0 errors;
  Vite production build succeeds, WASM bundled).
- **Not yet done by a human:** visual browser run of the new tools. To verify:
  `cd ui && pnpm dev`, then exercise Pencil/Eraser/Fill/Eyedropper/Move.

## M7 — layer system UI (complete)

Milestone was L/XL; split into M tasks (core test-first, then WASM, then UI):

- [x] **A — layer property + duplicate commands** (core, `command/properties.rs`,
  `command/layers.rs`): `RenameLayer`, `SetLayerOpacity` (slider-drag merge via
  `merge_with`), `SetLayerBlendMode`, `SetLayerVisible`, `SetLayerLocked`,
  `DuplicateLayer`. Apply/revert round-trip tests. Spec §5.2 / §7.3.
- [x] **B — merge / flatten commands** (core, `command/merge.rs` +
  `render::compose_over`): `MergeDown`, `MergeVisible`, `FlattenImage` (onto
  white). Snapshot-based undo; composite-preserving round-trip tests. Spec §5.2.
- [x] **C — WASM bindings**: `CommandSpec` gained the layer commands +
  `MoveLayer`; `get_document_info` now carries per-layer state (id/name/opacity/
  blend_mode/visible/locked, bottom-to-top); added `get_layer_thumbnail` and the
  non-undoable `set_active_layer` setter. ADR-008.
- [x] **D — Layers panel UI** (`LayersPanel.svelte`, `LayerThumbnail.svelte`,
  controller + store): row per layer (eye, lock, 32×32 thumbnail, name, blend
  dropdown, opacity slider), add/delete/duplicate/merge-down/merge-visible/
  flatten buttons, drag-to-reorder, double-click rename, click-to-select. §16.5.

### Verification (M7)

- `cargo test --workspace` green (118 core tests incl. 8 merge + 8 property/
  duplicate); `cargo clippy --workspace --all-targets -- -D warnings` clean;
  `cargo fmt --check` clean.
- `wasm-pack build --target web --release` succeeds; new exports present
  (`get_layer_thumbnail`, `set_active_layer`).
- `pnpm check` (svelte-check 0 errors), `pnpm lint`, `pnpm build` all green.
- **Not yet done by a human:** visual browser run of the layers panel. To
  verify: `cd ui && pnpm dev`, then add/duplicate/reorder layers, toggle
  visibility/lock, edit opacity/blend, rename, merge down / merge visible /
  flatten, and confirm thumbnails update and undo/redo restores each step.

## M8 — selection tools (complete)

Milestone was XL; split into M tasks (core test-first, then WASM, then UI):

- [x] **8A — selection mask foundation** (`selection/mod.rs`): `SelectionMask`
  (single-channel coverage), `SelectionMode` (Replace/Add/Subtract/Intersect)
  with `combine` + `apply_mode`, rectangle & ellipse rasterizers, `invert`,
  `new_full`/`new_empty`, `selected_count`. `Document.selection` retyped from
  `Option<ImageBuffer>` to `Option<SelectionMask>`. 12 tests. Spec §8.1–§8.4.
- [x] **8B — magic wand, lasso, modifiers** (core): `selection::magic_wand`
  (BFS flood, contiguous + global, tolerance, current-layer/composite sample);
  `SelectionMask::polygon` (even-odd scanline) for Lasso / Polygonal Lasso;
  `expand` / `contract` (separable square dilation/erosion) and `feather`
  (triple box blur). 13 tests incl. wand contiguous vs global. Spec §8.4, §9.3.
- [x] **8C — SetSelection command + mask constraint** (core): `SetSelection`
  (lazy `before` capture, `replace`/`clear` + `with_label`); the brush
  rasterizer (`StrokeCtx`) and Fill now scale each written pixel by the active
  mask's coverage (None = fully selected). 6 tests (round-trip, stroke outside /
  straddling selection, fill within selection). Spec §7.3.
- [x] **8D — WASM bindings** (ADR-009): `CommandSpec` gains `SelectRectangle`/
  `SelectEllipse`/`SelectPolygon` (mode + feather), `SelectWand` (layer, seed,
  tolerance, contiguous, sample, mode), and the modifiers `SelectAll` /
  `Deselect` / `InvertSelection` / `ExpandSelection` / `ContractSelection` /
  `FeatherSelection`. Masks build + combine (`apply_mode`) in Rust → one
  `SetSelection`. `get_document_info` gains `has_selection`; new
  `get_selection_bounds` and `get_selection_mask` queries for the overlay.
- [x] **8E — UI**: five selection `ToolKind`s with toolbar buttons + M/L/W
  shortcuts (M/L cycle their pair); pointer gestures in `pointer.ts` (rubber-band
  rect/ellipse with Shift = square/circle, freehand Lasso, click-to-place +
  double-click/near-start close for Polygonal Lasso, click for Magic Wand);
  Shift/Alt → add/subtract/intersect mode; `CanvasOverlay.svelte` marching-ants
  overlay (animated dashed boundary traced from `get_selection_mask`, plus the
  in-progress gesture); Ctrl+A/Ctrl+D/Ctrl+Shift+I and Expand/Contract/Feather/
  Invert/Deselect buttons in the tool options bar.

### Verification (M8 complete)

- `cargo test --workspace` green (151 core tests); `cargo clippy --workspace
  --all-targets -- -D warnings` clean; `cargo fmt --check` clean.
- `wasm-pack build --target web --release` succeeds; `pnpm check` (svelte-check
  0 errors), `pnpm lint`, `pnpm build` all green.
- **Not yet done by a human:** visual browser run of the selection tools and
  marching-ants overlay. To verify: `cd ui && pnpm dev`, then draw rect/ellipse/
  lasso/polygon/wand selections (with Shift/Alt for add/subtract/intersect),
  confirm marching ants animate around the boundary and the in-progress shape,
  paint/fill inside vs outside the selection, and exercise Ctrl+A / Ctrl+D /
  Ctrl+Shift+I and the Expand/Contract/Feather buttons.

### Known limitations / follow-ups

- Marching ants stroke the mask's per-pixel boundary edges with an animated dash
  offset (not a single traced contour); good enough for Phase 1, revisit for
  large selections in the M16 performance pass.
- Selection shapes are hard-edged in the rasterizer; the Lasso/Wand "anti-alias"
  option (spec §9.3) and the rect/ellipse anti-aliased edges are deferred.
- Overlay rebuilds the boundary path on every document mutation (O(canvas));
  fine for Phase 1, a dirty-rect optimization belongs to M16.

## Next concrete task — M9 (transform tools)

Free Transform (translate/scale/rotate), Flip H/V (layer + canvas), Rotate
90/180, Crop to selection / canvas, Resize canvas (9-grid anchor), Scale image
(nearest/bilinear/bicubic). Spec §10 / M9. `transform/` is a new core module
(CLAUDE.md §5.1); start with the affine + interpolation core (test-first:
rotate 90×4 = identity, scale ×2 then ×0.5 ≈ identity) before the UI.

The full pointer-event `Tool` trait (spec §9.1) is still deferred; tools keep
the "stroke/seed → command" shape — fold the trait in when a tool needs richer
modifier/cursor state.

## Open questions

- **WebP lossy export** (ADR-007): the spec §13.2 asks for lossy quality 1–100,
  but the pure-Rust `image` crate only encodes lossless WebP and CLAUDE.md
  forbids system deps. Currently lossless only. Decide whether to accept a
  pure-Rust lossy encoder dependency or keep lossless.
