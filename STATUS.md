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

## M9 — transform tools (complete bar optional Free Transform)

Milestone is XL; split into M tasks (core test-first, then commands, then UI):

- [x] **9A — transform buffer primitives** (`transform/mod.rs`): `flip_horizontal`/
  `flip_vertical`, `rotate_90_cw`/`rotate_90_ccw`/`rotate_180` (exact index
  permutations), `Interpolation` (Nearest/Bilinear/Bicubic), and `scale` (per-
  channel f32 sampling, Catmull-Rom bicubic). 12 tests (rotate 90×4 = identity,
  flip twice = identity, scale ×2→×0.5 bicubic ≈ identity, nearest exact).
- [x] **9B — discrete flip/rotate commands** (core, `command/transform.rs`):
  `TransformLayer` (active-layer FlipHorizontal/FlipVertical/Rotate180,
  dimension-preserving, self-inverse) and the canvas ops `FlipCanvas` +
  `RotateCanvas` (Cw90/Ccw90/Rotate180, all layers; 90° swaps canvas dims;
  selection cleared and restored on undo). All lossless → invertible by
  re-application, no snapshot. 5 tests.
- [x] **9C — scale / crop / resize-anchor / layer 90°** (core): `ResizeCanvas`
  gained a 9-grid `Anchor` (default TopLeft); `ScaleImage` (all layers via
  `transform::scale`, resizes canvas, snapshot undo); `CropToSelection` (canvas
  ← selection bbox, layers cropped, snapshot undo, ADR-010 clips outside pixels);
  `RotateLayer90` (rotate + center-fit into canvas dims, snapshot undo). 9 tests.
- [ ] **9D — Free Transform math** (core, optional for Phase 1): compose
  translate/scale/rotate into one affine applied to a layer with chosen
  interpolation (spec §10.1 interactive). Can be deferred — the discrete
  transforms + scale cover the M9 exit criteria; Free Transform is the
  interactive handle UI's backing math.
- [x] **9E — WASM bindings** (ADR-011): `apply_command`'s `CommandSpec` gained
  `TransformLayer` (flip_h/flip_v/rotate_180), `RotateLayer90` (ccw),
  `FlipCanvas` (horizontal), `RotateCanvas` (cw90/ccw90/rotate_180), `ScaleImage`
  (width/height/interpolation), `ResizeCanvas` (width/height/anchor),
  `CropToSelection`. Interpolation + anchor are snake_case strings.
- [x] **9F — UI** (`menus/TransformMenu.svelte`, `dialogs/ResizeDialog.svelte`,
  `dialogs/ScaleDialog.svelte`): header Image menu (Flip H/V, Rotate 90 CW/CCW,
  Rotate 180, Resize Canvas…, Scale Image…, Crop to Selection) and Layer menu
  (Flip H/V, Rotate 90 CW/CCW, Rotate 180); Resize dialog with a 9-grid anchor
  picker; Scale dialog with constrain-proportions + interpolation. Controller +
  `wasm.ts` `TransformCommand` union wired. Crop disabled without a selection.

### Verification (9A–9C)

- `cargo test --workspace` green (174 core tests); `cargo clippy --workspace
  --all-targets -- -D warnings` clean; `cargo fmt --check` clean.

The transform core is complete enough for the M9 exit criteria: flips, rotate
90/180 (layer + canvas), scale image (nearest/bilinear/bicubic), resize canvas
(9-grid anchor), and crop to selection are all implemented and undoable. Only
the WASM bindings and UI remain (plus optional interactive Free Transform).

### Verification (M9)

- `cargo test --workspace` green (174 core tests); `cargo clippy --workspace
  --all-targets -- -D warnings` clean; `cargo fmt --check` clean.
- `wasm-pack build --target web --release` succeeds; `pnpm check` (svelte-check
  0 errors/0 warnings), `pnpm build` green.
- **Not yet done by a human:** visual browser run of the transform menus and
  dialogs. To verify: `cd ui && pnpm dev`, then exercise Image/Layer flip and
  rotate, Resize Canvas (try each anchor), Scale Image (each interpolation), and
  Crop to Selection (with a selection active); confirm undo restores each.

### Known limitations / follow-ups

- **Free Transform (spec §10.1, task 9D)** — interactive translate/scale/rotate
  handles (Ctrl+T) are not implemented. The discrete transforms + Scale dialog
  cover the M9 exit criteria; the affine math + handle UI is a follow-up.
- Arbitrary-angle canvas rotation (spec §10.3) is deferred with Free Transform.

## M10 — shapes + text (complete)

Milestone was XL; split into core → WASM → UI tasks. The font dependency was
approved (ab_glyph) with the UI supplying font bytes (Option B), per ADR-012.

- [x] **10A — shapes rasterizer** (core, `tools/shapes.rs`): `Shape` (Line,
  Rectangle, RoundedRectangle, Ellipse, regular Polygon), `ShapeMode`
  (Outline/Fill/FillAndOutline), `ShapeStyle` (stroke width/color, fill color,
  anti-alias) and `Shapes::draw` → one `SetPixels`. SDF-based per-pixel coverage
  unifies fill, centered stroke and AA; the active selection mask is honored.
  Spec §9.2 Shapes.
- [x] **10B — shapes WASM binding** (`apply_command` `DrawShape`): `shape`
  discriminator + geometry (`points`/`center`/`radius`/`sides`/`rotation`/
  `corner_radius`), `mode`, stroke width/color, fill color, `anti_alias`, `dash`.
- [x] **10C — Shapes tool UI**: toolbar button + `U` shortcut, rubber-band drag
  with Shift constrain, options bar (shape/mode/stroke/dash/sides/corner radius/
  opacity/anti-alias), live overlay preview, `draw_shape` on pointer up.
- [x] **10D — Text tool** (core `tools/text.rs` via ab_glyph, ADR-012): line
  layout (kerning, L/C/R alignment), faux-bold/italic, AA, selection-masked,
  one undoable `SetPixels`. WASM `register_font` + `DrawText`. UI Text tool
  (`T`): floating text-entry box committed on Esc/Ctrl+Enter/blur, options bar
  (size/bold/italic/align/anti-alias). Default font Liberation Sans (SIL OFL
  1.1) shipped in `ui/public/fonts` and a core test fixture.
- [x] **Dash patterns** (core + WASM + UI): solid/dashed/dotted outlines,
  stepped along the perimeter polyline.

### Verification (M10)

- `cargo test --workspace` green (194 core tests incl. 13 shapes + 7 text);
  `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt
  --check` clean; `cargo build -p fineliner-wasm --target
  wasm32-unknown-unknown --release` succeeds.
- `pnpm run wasm` (wasm-pack) + `svelte-check` (0 errors) + `vite build` succeed.
- Node smoke tests through the real WASM boundary: a filled rectangle paints
  4500 px (= 90×50); "Hello" text paints 898 px and undo clears to 0; dashed
  (854) and dotted (769) outlines are sparser than solid (1268). This exercises
  the M10 exit criteria (spec §16): all shape types render, text rasterizes,
  text commit is undoable.
- **Not yet done by a human:** visual browser run. `cd ui && pnpm dev`, then
  draw each shape (Outline/Fill/Fill+Outline, dashes, Shift-constrain), and use
  the Text tool (click, type, Esc to commit; try bold/italic/align/size).

### Known limitations / follow-ups

- **Arrow shape** (spec §9.2) is deferred — not in CLAUDE.md's M10 shape list.
- Bold/italic are synthesized (faux); real font-family selection and multiple
  faces are Phase 2 (ADR-003, ADR-012).
- The text-entry overlay is a single textarea (foreground-colored live preview);
  rich on-canvas caret/IME is a later polish item.
- Dash stepping is O(pixels × on-segments); fine for Phase 1, an M16 concern.

## Deep fixup — round 2 (2026-07)

Three parallel audits (UI, core/WASM, DX) with line-level verification, then a
full fixup pass. All findings fixed and verified end-to-end (cargo gate, node
WASM smoke tests, Playwright browser run):

- **Core bugs:** `MergeDown` now keeps the lower layer's blend mode/opacity
  (was silently reset to Normal/1.0, visibly changing the composite);
  `ResizeCanvas` drops the selection mask and restores it on undo (a stale
  old-sized mask silently broke add/subtract gestures, the overlay, and
  painting in grown regions); JPEG export flattens over white in linear light
  (was black in gamma space, inconsistent with FlattenImage).
- **Wire-tag bug:** serde's `rename_all` yields `rotate_layer90`, the UI sends
  `rotate_layer_90` — Layer ▸ Rotate 90° was rejected as an unknown variant.
  Found by the new ts-rs codegen; fixed with an explicit rename + test.
- **WASM hardening:** document handles are never reused (stale handles error
  instead of aliasing a newer document); `register_font` dedupes; selection
  modifier radii are clamped (wasm32 overflow).
- **New command:** `delete_selection` (core + WASM + Delete/Backspace) erases
  the selected pixels of the active layer, coverage-scaled.
- **UI gesture fixes:** in-flight gestures are owned by their pointerId
  (multi-touch/second-button no longer corrupts them); Escape aborts a
  gesture; selection/shapes/text respond to the primary button only; the New
  button reports errors.
- **UX:** shared `Modal` (Escape/Enter/autofocus, global-shortcut suppression),
  per-tool cursors, beforeunload guard, `[`/`]` brush size, Ctrl+E export
  menu, hex color entry, export filenames from the opened file's stem,
  marching-ants RAF idles when the overlay is empty.
- **DX:** GitHub Actions CI (full §10 gate + generated-bindings freshness);
  `pnpm dev` builds WASM with `--dev`; ts-rs-generated `CommandSpec.ts` +
  type-level drift check (ADR-014); insta snapshots for compose/codec; codec
  round-trip proptests; core dedupe (tolerance test on `Color`,
  `ImageBuffer::offset_copy`, one `DocSnapshot`).

## Backlog — known open items (post fixup round 2)

Everything below is known, deliberate, and waiting for its milestone or a
decision. Consolidated from the round-2 audits so nothing lives only in a PR
description.

### Feature gaps (spec-mandated, deferred)

- ~~**Zoom / pan** (spec §16.1)~~ — **DONE** (2026-07): view transform in the
  store; wheel-to-cursor zoom, middle-drag pan, corner control (−/+/fit/1:1),
  auto-fit on open + resize. Follow-ups: keyboard shortcuts (Ctrl+0/±) and a
  space-drag hand, status-bar zoom %/cursor coords.
- **New-document size dialog** — `New` is fixed 800×600; the `Modal` shell
  from round 2 makes this an S task now.
- **Move-tool ghost preview** (spec §9.2) — needs a per-layer pixel read API
  across the WASM boundary (only `composite` and 32×32 thumbnails cross it
  today). M-sized; the API is the prerequisite.
- **Free Transform** (spec §10.1, task 9D; Ctrl+T) — interactive
  translate/scale/rotate handles + the affine math. Arbitrary-angle canvas
  rotation (spec §10.3) rides along with it.
- **Anti-aliased selection edges** (spec §9.3) — rect/ellipse/lasso/wand masks
  are hard-edged; the "anti-alias" option is unimplemented (M8 deferral).
- **Arrow shape** (spec §9.2) — not in CLAUDE.md's M10 shape list; decide
  whether it enters Phase 1 at all.
- **Lossy WebP export** (ADR-007 open question) — pure-Rust `image` only
  encodes lossless; accepting a new encoder dependency needs a decision.

### UX polish (small, unscheduled)

- Status-bar live cursor coordinates (and zoom % once zoom exists).
- Brush-size ring cursor preview (the CSS cursor from round 2 is static).
- Recent-colors row / swatch palette in the Colors panel (hex entry exists).
- Rich text-entry caret/IME on canvas (today: styled textarea overlay).

### DX / infra

- **ESLint + Prettier for `ui/`** — new dev dependencies, needs explicit
  approval per CLAUDE.md §9; svelte-check is the only linter today.
- **UI tests (Vitest/Playwright)** — Phase 3 (§7.5). The round-2 Playwright
  verification script is a session artifact, not checked in; it would be the
  seed for `pnpm test:e2e`.
- **Criterion benchmarks** (§7.6) — land with fineliner-effects (M11) and the
  M16 performance pass; `benches/` does not exist yet.
- `cw90`/`ccw90` wire strings are inconsistent with `rotate_180` — cosmetic;
  normalizing is cross-boundary churn, batch it with the next WASM API change.
- `canvas2d.ts` copies the composite an extra time per redraw
  (`new Uint8ClampedArray(rgba)`); unverified whether it is avoidable —
  check during M16.

### Performance (M16 pass)

- Marching-ants overlay: O(canvas) boundary rebuild per mutation and per-pixel
  edge stroking (the RAF loop now idles, but rebuild cost remains).
- Dirty-rect compositing, WebGPU path, WASM buffer reuse — all M16 as planned.

### Deliberate non-goals (documented decisions)

- `properties.rs` command boilerplate stays explicit (generic/macro rejected
  for clarity).
- `[workspace.lints] warnings = deny` skipped — CI enforces `-D warnings`;
  a manifest deny hurts local iteration.
- Liberation Sans committed twice (ui asset + core test fixture, 404 KB each)
  — test-fixture isolation is intentional.

## M11 — fineliner-effects crate (effects core COMPLETE; WASM+UI remain)

New crate `fineliner-effects`, independent of core (ADR-002). All four effect
groups are implemented, tested, and committed (43 tests). **ADR-015** pins the
colour-space/alpha convention (premultiplied, gamma/sRGB space).

- [x] **Crate skeleton** (`image.rs`, `effect.rs`, `error.rs`, `kernel.rs`,
  `lib.rs`): `EffectImage` (owned RGBA8, byte layout matching core's) with
  premultiplied-alpha f32 conversions and a bilinear `resized`; the `Effect`
  trait (`apply` + `scaled`, provided `preview`); `EffectError`; shared
  `convolve_3x3`. std-only, no new deps.
- [x] **Blur group** (`blur/`): Gaussian (σ=radius/3), Box (odd w/h), Motion
  (distance/angle), Radial (Spin/Zoom). Shared `convolve_axis` +
  `sample_bilinear`.
- [x] **Sharpen group** (`sharpen/`): `Sharpen` (fixed 3×3) and `UnsharpMask`
  (amount/radius/threshold, Gaussian low-pass, luminance-gated).
- [x] **Distort group** (`distort/`): `Emboss` (angle/elevation/relief),
  `EdgeDetect` (Sobel/Prewitt/Laplacian + amount), `Relief` (colour-preserving
  directional emboss). Shared `luma_at` + normalised `gradient`.
- [x] **Noise group** (`noise/`): `AddNoise` (Uniform/Gaussian, RGB/Mono,
  seeded xorshift64* PRNG — no `rand` dep) and `ReduceNoise` (median filter).

### Verification (M11 effects core)

- `cargo test -p fineliner-effects` green (43 tests, incl. mandated identities
  Gaussian σ=0 / Box 1×1); `cargo test --workspace` green; `cargo clippy
  --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean.

### M11 wiring — DONE

- [x] **WASM bindings** (ADR-017, spec §17.5): `fineliner-effects` added as a
  wasm dep; `apply_effect(handle, layer, effect)` runs the effect on the target
  layer's pixels as one undoable `SetPixels`; `preview_effect(handle, layer,
  effect)` composites the doc with that layer replaced and returns canvas-sized
  RGBA8 (no mutation). `EffectSpec` JSON (11 variants) with a ts-rs-generated
  mirror + drift check. Layer by index; preview full-size (max_dim → M16).
- [x] **UI** (spec §11): Effects menu (`EffectsMenu.svelte`, grouped
  Blur/Sharpen/Distort/Noise) + shared `EffectDialog.svelte` (per-effect
  parameter controls from `effects.ts`, 120 ms-debounced live preview via a
  `previewComposite` override on the canvas, Apply/Cancel). Controller gains
  `applyEffect`/`previewEffect`/`clearEffectPreview`.

### Verification (M11 wiring)

- `cargo test -p fineliner-wasm` green (ts-rs mirror regenerated); `cargo clippy
  --workspace --all-targets -- -D warnings` clean.
- `pnpm` (wasm:dev build) + `svelte-check` = 0 errors / 0 warnings; `vite build`
  succeeds.
- Node smoke test through the real WASM boundary: preview returns a canvas-sized
  buffer and does not mutate; Gaussian blur changes the composite and spreads
  alpha; undo restores the pre-effect composite exactly; sharpen/emboss/
  edge-detect/add-noise/reduce-noise all apply + undo cleanly.
- **Not yet done by a human:** visual browser run of the Effects menu/dialog.

**M11 is complete.** Deferred with M16 (per backlog): Criterion benches
(`benches/` does not exist yet); premultiplied preview downscale (currently
straight bilinear); radial-blur centre picking in the UI (defaults to canvas
centre).

## M12 — adjustments (COMPLETE)

All 9 adjustments (spec §12), routed through the same layer-targeting
apply_effect/preview_effect path as effects (EffectSpec gained the variants).

- [x] **Core** (`fineliner-effects/src/adjust/`): Brightness/Contrast (Legacy +
  Enhanced S-curve), Hue/Saturation/Lightness (+colorize, HSL), Curves (monotone
  cubic spline → LUT, per channel incl. alpha), Levels (in/gamma/out → LUT),
  Color Balance (per-tone-range shifts + optional preserve-luminosity), Invert,
  Grayscale (Luminosity/Average/BT.709/Channel Mixer), Posterize, Threshold.
  Per-pixel, gamma space, alpha preserved; `scaled` = identity. 25 tests incl.
  the mandated invert∘invert=id, curves identity, grayscale luma.
- [x] **WASM**: 9 EffectSpec variants + ts-rs mirror; channel/method as
  snake_case strings.
- [x] **UI**: Adjustments menu (grouped Tone/Color/Stylize) reusing the effect
  dialog. The dialog's field system gained boolean toggles and array-element
  (index) fields (Color Balance, enhanced/colorize/preserve flags). Curves are
  exposed as presets (Increase Contrast / Lighten / Darken) in Phase 1.

### Verification (M12)

- `cargo test --workspace` green (68 effects tests); `clippy`/`fmt` clean;
  `svelte-check` 0/0; `vite build` ok. Node WASM smoke test: invert∘invert =
  identity, grayscale → R=G=B, curves identity no-op, levels preview non-mutating
  canvas-sized, and color-balance/brightness/hue/posterize/threshold apply+undo
  back to the exact original.
- **Deferred (Phase 2 / follow-up):** a graphical Curves editor (presets only
  now); a custom Grayscale channel-mixer UI (fixed methods now); larger
  Color-Balance layout polish. All reachable via the WASM API regardless.
- **Not yet done by a human:** visual browser run of the Adjustments menu.

## Release automation + Cloudflare deploy (in progress)

- [x] **Auto-publish on push to main** (ADR-016, `.github/workflows/
  release.yml`): tag-based patch increment (highest `vX.Y.Z` + 1, seeded
  v0.1.0), builds the PWA bundle, cuts a GitHub Release with generated notes +
  `fineliner-web-<version>.zip`. Works with the built-in `GITHUB_TOKEN`.
- [x] **Cloudflare Workers deploy repo-side** (`wrangler.jsonc` serves
  `./ui/dist` as an SPA; `scripts/cf-build.sh` installs Rust/wasm-pack +
  builds). Deploy runs through Cloudflare's Workers Builds Git integration
  (user's choice, not GitHub Actions).
- [ ] **USER ACTION REQUIRED (Cloudflare dashboard):** the Workers project's
  build command must be set to `bash scripts/cf-build.sh` (root dir `/`, deploy
  command default `npx wrangler deploy`). Until then the Cloudflare build keeps
  failing because its image has no Rust/wasm-pack. Also consider limiting
  Git-integration deploys to the `main` branch (it currently builds PR branches
  as "production").

The full pointer-event `Tool` trait (spec §9.1) is still deferred; tools keep
the "stroke/seed → command" shape — fold the trait in when a tool needs richer
modifier/cursor state.

## Open questions

- **WebP lossy export** (ADR-007): the spec §13.2 asks for lossy quality 1–100,
  but the pure-Rust `image` crate only encodes lossless WebP and CLAUDE.md
  forbids system deps. Currently lossless only. Decide whether to accept a
  pure-Rust lossy encoder dependency or keep lossless.
