# Deep Fixup — execution plan (2026-06-10)

Session record for the deep-fixup pass. Analysis: 3 parallel audits (Rust crates,
Svelte UI, drift/DX) plus line-level verification of every claim. The repo is
healthy — Rust baseline fully green, no forbidden patterns, no dead code, M1–M10
claims accurate, deferred items all documented (ADRs / STATUS known-limitations).
Surviving findings: 1 real robustness bug, 3 minor UI state bugs, small UX/DX gaps.

## Baseline

- `cargo fmt --check` / `cargo clippy --workspace --all-targets -- -D warnings` /
  `cargo test --workspace` — all green at session start.
- `ui/`: `pnpm check|build` need wasm-pack + the `wasm32-unknown-unknown` target
  (environment setup, T0 — not a repo defect).

Full-verification gate:

```sh
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cd ui && pnpm check && pnpm build
```

## Findings dropped after verification (no tasks)

- "Text entry coordinate-scale bug" — false; textarea is positioned in display px
  inside a wrapper sized to the displayed canvas; consistent with `toCanvasPoint`.
- "Opacity slider floods undo" — false; `SetLayerOpacity::merge_with`
  (command/properties.rs) coalesces slider drags.
- "Ctrl+Z during rename triggers global undo" — false; App.svelte guards
  `HTMLInputElement | HTMLTextAreaElement`.
- Marching-ants O(W×H) rebuild + overlay DPR — documented STATUS known-limitations (M16).
- `rmp-serde`/`zstd` workspace deps unused — deliberate (M14, ADR-004).
- Rust crates: dedicated audit found no demonstrable bugs.

## Tasks

- [ ] T0: Make the UI verification gate runnable (environment, no commit)
      Files: none (environment only)
      Change: `rustup target add wasm32-unknown-unknown`; install wasm-pack; run
      `cd ui && pnpm check && pnpm build` once to establish green.
      Verify: both commands exit 0.

- [ ] T1: Fix dangling document handle when opening a corrupt image
      Files: ui/src/lib/core/controller.ts (`newDocument`, `openFile`)
      Change: create/open the NEW document first; only after success close the old
      handle and assign the new one. Today `openFile` closes the current doc, then
      `core.openImage` throws on a bad file → document destroyed + `editor.handle`
      points at a closed slot → every later command throws.
      Verify: `pnpm check` green.

- [ ] T2: Make in-flight pointer gestures immune to mid-drag tool switches; abort on pointercancel
      Files: ui/src/lib/tools/pointer.ts
      Change: (a) record `gestureKind = tool.kind` at `onDown`, use it in the drag
      branches of `onMove`/`onUp` (polygon-lasso click logic stays on live `tool.kind`);
      (b) `pointercancel` gets an abort handler: clear gesture state and previews,
      release capture, commit nothing.
      Verify: `pnpm check && pnpm build` green.

- [ ] T3: Reset layer drag state on dragend
      Files: ui/src/lib/components/panels/LayersPanel.svelte
      Change: `ondragend` resets `draggingIndex` so an abandoned drag can't turn a
      later external drop into a spurious reorder.
      Verify: `pnpm check` green.

- [ ] T4: Export menu — PNG / JPEG / WebP
      Files: ui/src/App.svelte, ui/src/lib/core/controller.ts
      Change: generalize `exportPng()` to `exportImage(format)` using the
      already-wired `core.exportJpeg` (quality 90) / `core.exportWebp` (lossless,
      ADR-007); Export button becomes a small dropdown (TransformMenu idiom).
      Verify: `pnpm check && pnpm build` green.

- [ ] T5: Text-entry preview honors the alignment option
      Files: ui/src/lib/components/canvas/MainCanvas.svelte
      Change: set `text-align` from `tool.textAlign` and shift the textarea with
      `translateX(0 | -50% | -100%)` so the preview anchors like the committed text
      (core anchors center/right about `x`, tools/text.rs).
      Verify: `pnpm check` green.

- [ ] T6: DX — add README.md, sync CLAUDE.md §10 to reality
      Files: README.md (new), CLAUDE.md §10
      Change: README with prerequisites (Rust, wasm32 target, wasm-pack, pnpm),
      clone→run steps, verification commands, repo layout. CLAUDE.md §10: note
      `pnpm lint` aliases `check` (svelte-check; no ESLint in Phase 1) and
      `pnpm test`/`test:e2e` are Phase 3 (§7.5).
      Verify: proofread; paths exist.

- [ ] T7: Final gate + wrap-up
      Change: run the full verification gate, check off PLAN.md, push
      `claude/deep-fixup-ut9874`, open a draft PR.
      Verify: gate green; PR exists.

## Not this session

- Move-tool ghost preview (CLAUDE.md M6 text vs STATUS deferral) — needs a new
  WASM per-layer pixel API; M-sized.
- New-document size dialog (New is fixed 800×600) and zoom/pan — feature work.
- ESLint/Prettier for ui/ — new deps require explicit approval (CLAUDE.md §9).
- `[workspace.lints]` polish; marching-ants perf (M16); lossy WebP (ADR-007).
