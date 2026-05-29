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

## Next concrete task — M7 (layer system UI)

Layer panel (add/delete/duplicate/reorder/visibility/lock/rename/opacity/blend),
32×32 thumbnails, 999-layer cap, merge-visible / flatten. Spec §16 / M7.
The full pointer-event `Tool` trait (spec §9.1) is still deferred; tools keep
the "stroke/seed → command" shape — fold the trait in when a tool needs richer
modifier/cursor state.

## Open questions

- **WebP lossy export** (ADR-007): the spec §13.2 asks for lossy quality 1–100,
  but the pure-Rust `image` crate only encodes lossless WebP and CLAUDE.md
  forbids system deps. Currently lossless only. Decide whether to accept a
  pure-Rust lossy encoder dependency or keep lossless.
