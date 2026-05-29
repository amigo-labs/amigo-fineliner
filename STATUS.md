# STATUS

## Current state — M1–M5 complete (Pencil demo)

The foundational pipeline is implemented and verified: a usable image editor
that opens a PNG, paints with the Pencil, and exports a pixel-correct PNG.

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

## M6 — basic tool suite (in progress)

Milestone is L/XL, split into S/M core tasks (pure logic, test-first) plus a
later UI-wiring task. Order:

- [x] **Fill / Paint Bucket** (`tools/fill.rs`) — BFS flood-fill, tolerance
  (Euclidean RGBA8), contiguous + all-pixels modes, sample current layer /
  all layers. Emits `SetPixels`. Spec §9.2 Fill.
- [ ] **Eyedropper** (`tools/eyedropper.rs`) — sample current layer / composite,
  size 1×1 / 3×3 / 5×5 / 11×11 / 31×31 avg. No command. Spec §9.2 Eyedropper.
- [ ] **Brush engine generalization + Eraser** — add hardness + paint mode
  (Paint / Erase-to-transparent / Erase-to-bg) to the brush; Soft Round / Flat
  variants. Spec §9.2 Pencil/Eraser. (Touches tested Pencil — yield after.)
- [ ] **Move** (`tools/move_tool.rs`) — translate active-layer pixels, emit
  `SetPixels` on pointer up. Spec §9.2 Move.
- [ ] **UI + WASM wiring** — expose each tool over the WASM boundary, add
  toolbar buttons, pointer-event handlers, keyboard shortcuts. Spec §16.2.

Implement the full pointer-event `Tool` trait (spec §9.1) when wiring the UI;
core tasks above keep the "stroke/seed → command" shape from M5.

## Open questions

- **WebP lossy export** (ADR-007): the spec §13.2 asks for lossy quality 1–100,
  but the pure-Rust `image` crate only encodes lossless WebP and CLAUDE.md
  forbids system deps. Currently lossless only. Decide whether to accept a
  pure-Rust lossy encoder dependency or keep lossless.
