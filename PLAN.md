# Deep Fixup — round 2, execution plan (2026-07-09)

Session record for the second deep-fixup pass (round 1: 2026-06-10, PR #5).
Analysis: 3 parallel audits (Svelte UI, Rust core/WASM, DX/build/docs) with
line-level verification of every claim before acting. Unlike round 1, this
pass found two real core correctness bugs plus a live wire-tag bug.

## Baseline

- `cargo fmt --check` / `cargo clippy --workspace --all-targets -- -D warnings`
  / `cargo test --workspace` (198 tests) — green at session start.
- `pnpm check` / `pnpm build` — green after installing wasm32 target + wasm-pack.

## Tasks (all complete)

- [x] A1: MergeDown keeps the lower layer's blend mode/opacity/visibility/lock
      (was reset to Normal/1.0 — Multiply relationships silently discarded).
- [x] A2: ResizeCanvas snapshots + clears the selection, restores on undo
      (stale old-sized mask broke combine gestures, overlay, painting).
- [x] A3: JPEG export flattens over white in linear light via a shared
      `over_background` helper (was black in gamma space).
- [x] B1: WASM document handles are monotonic, never reused (stale handle →
      error instead of silently aliasing a newer document).
- [x] B2: register_font dedupes byte-identical re-registrations.
- [x] B3: selection modifier radii clamped to the mask extent (wasm32 overflow).
- [x] C: delete_selection command — core (`tools/delete.rs`, coverage-scaled
      erase), WASM `delete_selection`, UI Delete/Backspace.
- [x] D1: pointer gestures owned by their starting pointerId; bystander
      down/move/up/cancel ignored (multi-touch corruption fixed).
- [x] D2: Escape aborts the in-flight gesture (`gestureControl.cancel`).
- [x] D3: selection/shapes/text gated to the primary button.
- [x] D4: New button catches failures into the status bar.
- [x] E: shared `Modal` (Esc/Enter/autofocus, `ui.modalOpen` suppresses global
      shortcuts), per-tool cursors, beforeunload guard, `[`/`]` + Ctrl+E
      shortcuts, hex color entry, export filename stem, marching-ants RAF
      idles when empty, dead `selectionBounds` adapter removed.
- [x] F: core dedupe — `Color::within_tolerance` (Fill/Wand),
      `ImageBuffer::offset_copy` (resize/center-fit), one `DocSnapshot`
      (merge + transform).
- [x] G: ts-rs codegen (ADR-014) — test-gated derive exports
      `ui/src/lib/core/generated/CommandSpec.ts`; `generated-check.ts` fails
      `pnpm check` on discriminant/field/type drift. First export caught the
      `rotate_layer_90` wire-tag bug (serde snake_case yields
      `rotate_layer90`); fixed with an explicit rename + regression test.
- [x] H: GitHub Actions CI (§10 gate + generated-diff check); `pnpm dev`
      builds WASM `--dev` (stale-WASM note in README); insta snapshots for
      compose/codec (`tests/snapshots/`); PNG/BMP round-trip proptests.
- [x] I: CLAUDE.md §15 planned-path markers, ADR-014, STATUS.md round-2
      section, this PLAN.md, push + draft PR.

## Verification

- Full gate green: fmt, clippy `-D warnings`, `cargo test --workspace`
  (214 tests), `pnpm check` (0 errors), `pnpm build`.
- Node smoke tests through the real WASM boundary: MergeDown composite
  preservation, resize+selection undo, delete_selection, stale-handle error,
  rotate_layer_90 tag.
- Playwright browser run (11 checks green): per-tool cursors, Escape abort,
  right-click gating, paint + Ctrl+A + Delete, modal autofocus/Escape, hex
  entry, no page errors.
- Drift check negative-tested: an injected field typo fails `pnpm check`.

## Not this session

- Zoom/pan, New-document size dialog, Move ghost, Free Transform — feature work.
- `cw90`/`ccw90` string normalization — cosmetic cross-boundary churn.
- ESLint/Prettier — separate dependency decision.
- Marching-ants contour tracing / dirty rects — M16 performance pass.
