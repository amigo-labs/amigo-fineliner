# CLAUDE.md — Fineliner Implementation Guide

This file is the operational contract between Claude Code and the Fineliner codebase. It exists to keep work efficient, predictable, and **scoped small enough that no single session runs into context limits or timeouts**.

Read this entire file before starting any task. Re-read Section 3 ("Session Management") at the start of every session.

---

## 1. Project Overview

Fineliner is a general-purpose image editor — a spiritual successor to paint.net — focused on simplicity, speed, and cross-platform reach. It ships as a PWA and a native Tauri app (Windows, macOS, Linux) sharing one Svelte UI and one Rust core.

**Sister project:** `amigo-pincel` is the pixel-art editor in the same org. Fineliner targets general image editing (photos, illustrations, UI assets). The two share no crates but follow the same workspace conventions.

The full design specification lives in **`docs/specs/fineliner.md`**. That document is the source of truth for *what* to build. This file is the source of truth for *how* to build it.

**Always read the relevant section of `docs/specs/fineliner.md` before starting an implementation task.** The spec is sectioned and indexed; load only what's needed.

---

## 2. Workflow Philosophy

### 2.1 Spec-Driven

Every implementation task traces back to a specific section of `docs/specs/fineliner.md`. If a task implies a design decision not in the spec, **stop and update the spec first** with a Decision Log entry. Do not improvise architecture.

### 2.2 Compiler-as-Gatekeeper

For Rust work, the compiler is the primary verification tool. The loop is:

1. State the change in plain English (one sentence)
2. Make the smallest code change that could possibly satisfy it
3. `cargo check -p <crate>` — must pass
4. `cargo test -p <crate>` — must pass for changed module
5. `cargo clippy -p <crate> -- -D warnings` — must pass
6. Commit

If step 3 fails, **fix it immediately** before moving on. Never accumulate compiler errors.

### 2.3 Test-First Where Sensible

Codecs (image I/O), command/undo logic, effects pipeline, and color math are test-first: write the test fixture and assertion, run it red, implement, run it green.

Tools, UI components, and rendering adapters are test-after: ship the implementation, then add interaction tests.

### 2.4 Micro-commits

One concept per commit. Commit messages start with the crate name in brackets:

```
[fineliner-core] add Document struct with Layer and Canvas types
[fineliner-core] implement SetPixel and PaintStroke commands
[fineliner-effects] add GaussianBlur effect with radius parameter
[ui] wire Pencil tool onPointerDown handler
[src-tauri] add native open/save file dialog commands
```

If a commit message needs an "and" between two unrelated changes, split it.

---

## 3. Session Management

**This is the most important section.** The codebase is large enough that a careless session can run out of context mid-task and leave the repo in a broken state. Follow these rules to avoid that.

### 3.1 Pre-Flight Checklist

Before writing any code in a new session, run through this checklist:

```
[ ] git status is clean (or I have a plan for the dirty state)
[ ] cargo check passes from a clean checkout
[ ] I have read the relevant docs/specs/fineliner.md section
[ ] I can state the task in one sentence
[ ] I can estimate complexity: T-shirt size XS / S / M / L
[ ] If L: STOP. Split into smaller tasks. Do not start.
```

If any line is unchecked, address it before writing code.

### 3.2 Task Sizing

| Size | Definition | Action |
| --- | --- | --- |
| **XS** | 1 file changed, ≤30 lines | Do it inline, no ceremony |
| **S** | 1–3 files changed, ≤150 lines | Single session, single commit |
| **M** | 3–6 files, ≤400 lines, multiple commits | Single session, plan commits up front |
| **L** | 6+ files, or new module / new crate | **Split into M tasks** before starting |
| **XL** | New phase milestone | **Split into L tasks**, write a sub-spec |

A session should produce one M task or two-to-three S tasks. Never a single L task.

### 3.3 Stopping Points

Stop and yield to human review at any of these triggers:

- A milestone exit criterion (Spec Section 16) is met
- A new public API surface is added (functions/types in `lib.rs` or `mod.rs` exports)
- A new dependency is added to `Cargo.toml` or `package.json`
- The spec needs a Decision Log entry
- Three consecutive `cargo check` failures with the same root cause
- A test was disabled or `#[ignore]`d
- Any `unsafe` block is added
- The session has produced ~400 lines of changes and a coherent stopping point exists

When stopping, write a short status note in `STATUS.md`:

- What was completed
- What's the next concrete task
- Any open questions

### 3.4 Resume Protocol

When starting a session that continues prior work:

1. `git log --oneline -10` to see recent commits
2. Read `STATUS.md` if present
3. Read the last commit message and diff
4. Run `cargo check && cargo test` — must be green before proceeding
5. State the next task explicitly
6. Run the Pre-Flight Checklist (§3.1)

If `cargo check` is red on a fresh checkout, **fix the build first**. Do not work on top of a broken state.

### 3.5 Context Discipline

- **Read minimally.** Don't load entire files when one function will do. Use `grep` / `rg` for navigation.
- **Don't re-read what you've just written.** Trust your own output for the duration of one session.
- **Drop irrelevant files from context.** If you opened `fineliner-core/src/document.rs` to check a type and you're now in `fineliner-effects`, you don't need that file anymore.
- **Don't echo specs back.** Reference it: `// See docs/specs/fineliner.md §3.2`.

---

## 4. Implementation Order

Implement in this order. Each milestone is a gate; do not start M(N+1) until M(N) is complete and committed.

```
M1  fineliner-core skeleton
    └─ Cargo.toml workspace, basic types:
       Document, Layer, LayerKind, Canvas, Color (RGBA8 + RGBA32f), Rect, Point
    └─ No I/O, no rendering. Pure data model + builder API.
    └─ Unit tests: type construction, layer add/remove, canvas resize

M2  fineliner-core commands + undo
    └─ Command trait, CommandBus, UndoStack (linear + tree-view capable)
    └─ Implement first 5 commands:
       SetPixels, AddLayer, RemoveLayer, MoveLayer, ResizeCanvas
    └─ Tests: apply/revert round-trip preserves state exactly
    └─ Tests: undo stack depth, clear, branch on new command after undo

M3  fineliner-core rendering pipeline — composite
    └─ compose(layers) → RGBA8 ImageBuffer
    └─ Blend modes: Normal, Multiply, Screen, Overlay, Darken, Lighten,
       Color Dodge, Color Burn, Hard Light, Soft Light, Difference, Exclusion
    └─ Layer opacity (0.0–1.0) applied before blend
    └─ Snapshot tests: known layer stack → expected RGBA bytes
    └─ Tests: each blend mode against reference values

M4  Image I/O (fineliner-core codec module)
    └─ Read + write: PNG, JPEG, WebP, BMP
    └─ Read-only: TIFF, GIF (first frame)
    └─ Write-only: export flattened composite
    └─ Round-trip tests: read fixture → write → read → assert pixel equality
    └─ Tests: edge cases (1×1, max dimensions, transparent PNG, EXIF strip on write)
    └─ Dependency: `image` crate only. No imagemagick, no system deps.

M5  fineliner-wasm + minimal UI (Pencil demo)
    └─ wasm-bindgen exports: Document, applyTool, compositeToImageData,
       importImage, exportPng
    └─ Svelte 5 + Vite scaffold under ui/
    └─ Single tool: Pencil (hard round, size 1–100, opacity, color)
    └─ Canvas rendering via Canvas2D (ImageData from WASM composite)
    └─ Demo: open PNG → paint → export PNG — pixel-perfect round-trip
    └─ No effects, no selections, no layers UI yet

M6  Basic tool suite
    └─ Eraser (respects layer alpha)
    └─ Fill (flood-fill with tolerance, contiguous + all-layers modes)
    └─ Eyedropper (sample current layer or composite)
    └─ Move (translate layer contents, with ghost preview)
    └─ Pencil brush variants: hard round, soft round, flat, textured
    └─ Each tool: Command type + input handler + UI button + keyboard shortcut
    └─ Tests: fill tolerance edge cases, eyedropper sampling modes

M7  Layer system UI
    └─ Layer panel: add, delete, duplicate, reorder (drag), visibility toggle,
       lock toggle, rename, opacity slider, blend mode dropdown
    └─ Layer thumbnails (32×32, updated on change)
    └─ Max 999 layers enforced
    └─ Merge visible, flatten image commands
    └─ Tests: merge/flatten produces correct composite

M8  Selection tools
    └─ Rectangle Select
    └─ Ellipse Select
    └─ Lasso (freehand polygon, click-to-close)
    └─ Polygonal Lasso (click-to-place vertices, dbl-click close)
    └─ Magic Wand (tolerance, contiguous, anti-alias)
    └─ Selection operations: Add, Subtract, Intersect (Shift/Alt/Shift+Alt)
    └─ Selection modifiers: Expand, Contract, Feather, Invert, Select All, Deselect
    └─ Selection as mask applied to all brush/fill operations
    └─ Marching-ants animation in UI (CSS animation, not canvas)
    └─ Tests: magic wand contiguous vs global, selection invert pixel coverage

M9  Transform tools
    └─ Free Transform: Translate, Scale (with aspect lock), Rotate (with snap)
    └─ Flip Horizontal / Vertical (layer and canvas)
    └─ Rotate 90° CW / CCW / 180°
    └─ Crop to selection / crop to canvas
    └─ Resize canvas (anchor point 9-grid)
    └─ Scale image (nearest, bilinear, bicubic)
    └─ Tests: rotate 90° × 4 = identity, scale × 2 then × 0.5 = ~identity (bicubic tolerance)

M10  Shapes + Text tools
    └─ Shapes: Line, Rectangle, Rounded Rectangle, Ellipse, Polygon (N-sided)
    └─ Shape modes: Outline, Fill, Fill+Outline
    └─ Stroke width + dash pattern
    └─ Text tool: font family, size, bold, italic, color, anti-alias
    └─ Text renders to pixels on commit (no vector layer in Phase 1)
    └─ Tests: shape outline pixel coverage, text rasterization non-empty

M11  fineliner-effects crate
    └─ Independent from fineliner-core. Takes &ImageBuffer, returns ImageBuffer.
    └─ Blur group: Gaussian, Box, Motion, Radial
    └─ Sharpen group: Unsharp Mask, Sharpen (convolution)
    └─ Distort group: Emboss, Edge Detect, Relief
    └─ Noise group: Add Noise (Gaussian/Uniform), Reduce Noise (median)
    └─ Each effect: parameter struct, apply() fn, preview() fn (downscaled)
    └─ Tests: Gaussian blur σ=0 = identity, box blur 1×1 = identity
    └─ Tests: sharpen is inverse-adjacent to blur (qualitative, not exact)

M12  Adjustments
    └─ Brightness / Contrast (linear)
    └─ Hue / Saturation / Lightness
    └─ Curves (per-channel RGB + composite, cubic spline interpolation)
    └─ Levels (black point, white point, gamma, output range)
    └─ Color Balance (shadows / midtones / highlights, per-channel)
    └─ Invert
    └─ Grayscale (with channel mixer weights)
    └─ Posterize (N levels)
    └─ Threshold
    └─ Adjustments applied destructively to current layer (Phase 1)
    └─ Tests: invert(invert(x)) = x, grayscale preserves luminance, curves identity

M13  Advanced tools
    └─ Gradient tool: Linear, Radial, Angle, Reflected, Diamond
    └─ Gradient modes: foreground→background, foreground→transparent, custom stops
    └─ Clone Stamp: sample point, brush size/hardness/opacity, aligned mode
    └─ Smudge brush: strength parameter, smears pixel color in stroke direction
    └─ Blur / Sharpen brush: local effect under cursor
    └─ Tests: gradient linear fill pixel values at endpoints, clone stamp offsets

M14  Tauri native shell
    └─ src-tauri/ with Tauri 2
    └─ fineliner-core as direct cargo dep (no WASM round-trip for file I/O)
    └─ Native commands: open_file, save_file, export_image, show_open_dialog,
       show_save_dialog, get_recent_files, app_version
    └─ File associations: .fln (Fineliner project), .png, .jpg, .webp, .bmp
    └─ .fln project format: zstd-compressed MessagePack of Document state
    └─ Menu bar: File, Edit (undo/redo/cut/copy/paste), Image, Layer, Effects,
       Adjustments, View, Help
    └─ Keyboard shortcuts match paint.net defaults where sensible
    └─ Tests: project save → load round-trip, all layers preserved

M15  PWA + Cloudflare deployment
    └─ Service worker with cache-first strategy for app shell
    └─ IndexedDB autosave (every 30 s, last 5 versions)
    └─ Recent files registry (URL handles via File System Access API)
    └─ OPFS (Origin Private File System) for project storage
    └─ wrangler.toml deployment to Cloudflare Pages
    └─ PWA manifest: icons (16, 32, 192, 512), theme color, display: standalone
    └─ Offline mode: full editing capability without network
    └─ Tests: service worker install/activate, IndexedDB read-back

M16  Performance pass
    └─ Dirty-rect compositing: only re-composite changed region
    └─ WebGPU render path (progressive enhancement over Canvas2D)
    └─ WASM memory: reuse ImageData buffers, no per-frame allocation
    └─ Target: 4000×3000 image with 10 layers composites in <16 ms
    └─ Target: Pencil stroke at 4K feels lag-free (input → render <32 ms)
    └─ Profiling with `console.time` + Rust `criterion` benchmarks
    └─ Benchmarks: compose_10_layers_4k, gaussian_blur_radius_10, flood_fill_large
```

Each milestone is a sequence of S/M tasks. Plan the tasks as a checklist in `STATUS.md` before starting the milestone.

---

## 5. Crate Conventions

### 5.1 `fineliner-core`

- **No platform dependencies.** No `tokio`, no `wasm-bindgen`, no `web-sys`, no `tauri`. Pure logic.
- **No file I/O at the system level.** Codec functions take `Read` / `Write` trait objects.
- **`std`-only.** Don't invest in `no_std`.
- **Errors:** `thiserror` for crate-level error types. One enum per submodule.
- **Public API:** re-exported from `lib.rs` only. Internal modules are `pub(crate)`.

```
crates/fineliner-core/src/
  lib.rs              re-exports only
  document/           Document, Layer, LayerKind, Canvas, History
  color/              Color, Palette, BlendMode, color space conversions
  tools/              Tool trait, individual tool impls, brush engine
  command/            Command trait, CommandBus, UndoStack
  selection/          SelectionMask, operations (add/sub/intersect)
  transform/          affine transforms, interpolation modes
  codec/              png.rs, jpeg.rs, webp.rs, bmp.rs, project.rs (.fln)
  render/             compose(), blend_layers(), dirty_rect tracking
  geometry/           Rect, Point, Size, basic geometry helpers
  effects/            (re-exports from fineliner-effects, optional feature)
```

### 5.2 `fineliner-effects`

- **Independent crate.** No `fineliner-core` dependency. Takes pixel buffers, returns pixel buffers.
- **Pure functions.** `apply(params, &ImageBuffer) -> ImageBuffer` — no side effects, no state.
- **f32 internally.** Convert RGBA8 → RGBA32f on entry, convert back on exit.
- **Benchmark every effect.** `benches/` with Criterion. Gate on M16.

```
crates/fineliner-effects/src/
  lib.rs
  blur/           gaussian.rs, box_blur.rs, motion.rs, radial.rs
  sharpen/        unsharp_mask.rs, convolution.rs
  distort/        emboss.rs, edge_detect.rs
  noise/          add_noise.rs, reduce_noise.rs
  adjust/         brightness_contrast.rs, hue_sat.rs, curves.rs,
                  levels.rs, color_balance.rs, invert.rs,
                  grayscale.rs, posterize.rs, threshold.rs
```

### 5.3 `fineliner-wasm`

- **Crate type:** `cdylib`. Built via `wasm-pack build --target web --release`.
- **Public API surface:** matches npm package spec in `docs/specs/fineliner.md §17.5`. Keep it stable.
- **Memory:** document state owned in Rust. JS receives `Uint8ClampedArray` views from composited frames.
- **Build output:** `pkg/` — gitignored, generated.
- **No effects crate in WASM Phase 1.** Effects are added in M11 when the crate is ready.

### 5.4 `ui/`

- **Framework:** Svelte 5 with runes (`$state`, `$derived`, `$effect`).
- **Styling:** Tailwind 4 utility classes; complex components via shadcn-svelte.
- **TypeScript strict mode.** No implicit `any`. No `// @ts-ignore` without an issue link.
- **State stores:** Svelte 5 runes in `src/lib/stores/`. One store per concern.
- **WASM bridge:** all calls go through `src/lib/core/` adapter. UI never imports `fineliner-wasm` directly.
- **Canvas rendering:** `src/lib/render/` — Canvas2D adapter (M5), WebGPU adapter (M16).
- **No business logic in Svelte components.** Components handle events and display state only.

```
ui/src/
  lib/
    core/           WASM adapter (the only place fineliner-wasm is imported)
    render/         Canvas2D and WebGPU renderers
    stores/         document.ts, tools.ts, ui.ts, history.ts, prefs.ts
    tools/          input handlers (pointer events → tool commands)
    components/     Svelte components
      canvas/       MainCanvas.svelte, CanvasOverlay.svelte
      panels/       LayersPanel.svelte, ColorsPanel.svelte,
                    HistoryPanel.svelte, EffectsPanel.svelte
      toolbar/      ToolBar.svelte, ToolOptions.svelte
      dialogs/      ResizeDialog.svelte, ExportDialog.svelte,
                    CurvesDialog.svelte, LevelsDialog.svelte
      ui/           shadcn-svelte components (copied, not runtime dep)
  routes/           SvelteKit pages if needed, else single App.svelte
```

### 5.5 `src-tauri/`

- **Tauri 2.** Don't introduce v1 patterns.
- **`fineliner-core` is a direct cargo dependency.** No WASM round-trip for native operations.
- **Commands are thin wrappers:** deserialize args → call core → serialize result. No business logic.
- **State management:** use Tauri's `State<Mutex<AppState>>` for the open document.

---

## 6. Code Style

### 6.1 Rust

- Edition: latest stable in workspace `Cargo.toml`
- Formatter: `cargo fmt` with default config
- Linter: `cargo clippy -- -D warnings` — must be clean
- No `unwrap()` outside tests. Use `?` or `.expect("documented invariant")`.
- No `unsafe` without a `// SAFETY:` comment. New `unsafe` must be called out in PR description.
- Prefer `&[u8]` over `Vec<u8>` for function args when ownership isn't needed.
- Color math in f32. Clamp on output, not mid-computation.
- Doc comments (`///`) on every public item.

### 6.2 TypeScript / Svelte

- Strict mode, all flags on
- No `any`. Use `unknown` and narrow.
- `import type { … }` for type-only imports
- File naming: `kebab-case.ts` for utilities, `PascalCase.svelte` for components
- Pointer events (not mouse events) for tool input — required for stylus/touch support.

### 6.3 Markdown / Specs

- Headings sentence case
- Code blocks always have a language tag
- Tables for structured comparisons
- One blank line above and below code blocks and tables

---

## 7. Testing Requirements

### 7.1 Unit Tests

Inline `#[cfg(test)] mod tests { ... }` in the same file. Name tests `<function>_<scenario>_<expected>`:

```rust
#[test]
fn compose_two_normal_layers_produces_top_layer_pixels() { … }

#[test]
fn flood_fill_with_zero_tolerance_fills_exact_color() { … }

#[test]
fn gaussian_blur_with_sigma_zero_returns_identity() { … }

#[test]
fn undo_stack_after_undo_then_new_command_clears_redo() { … }
```

### 7.2 Integration Tests

`tests/` directory at crate root. Use for cross-module flows: open image → apply effect → export, command/undo sequences across modules, project save/load round-trips.

### 7.3 Snapshot Tests

For codecs and compositing: use `insta` crate for snapshot testing. Store approved snapshots in `tests/snapshots/`. Review snapshot diffs carefully on any compositing or codec change.

### 7.4 Property-Based Tests

`proptest` for:
- Color blend mode math (commutativity where applicable, clamping invariants)
- `compose()` idempotency: compositing twice with same state = same result
- Codec round-trips: `encode(decode(bytes)) ≈ bytes` (lossy formats: perceptual threshold)

### 7.5 UI Tests

Phase 3 concern (post-M15). Use Playwright. Each tool gets a happy-path test minimum.

### 7.6 Benchmarks

`benches/` with `criterion` in `fineliner-effects` and `fineliner-core`. Run with `cargo bench`. Track regressions. Baselines stored in `benches/baselines/`.

### 7.7 Test Performance

Full test suite must run in CI under 90 seconds. Slow tests get `#[ignore]` and a separate CI job.

---

## 8. Branch & PR Conventions

- `main` is protected; all changes via PR
- Branch names: `feat/<short-desc>`, `fix/<short-desc>`, `chore/<short-desc>`, `spec/<short-desc>`
- PR title: `[crate] action` — e.g. `[fineliner-effects] add GaussianBlur`
- PR description includes:
  * Spec section reference
  * Test names covering the change
  * Screenshot or recording for any UI change
  * Open questions for review
- CI must be green before review request

---

## 9. Forbidden Patterns

These produce immediate revert. No exceptions.

- **`unwrap()` in non-test code.** Always `?` or documented `expect`.
- **Reading pixel data from a `<canvas>` element.** Document state lives in Rust; the canvas is render-only. Reading back from canvas introduces GPU round-trips and stale-state bugs. See spec §17.1.
- **`localStorage` / `sessionStorage`.** Use IndexedDB for persistence.
- **Mouse events for tool input.** Use pointer events — stylus support requires it.
- **New runtime dependencies without explicit approval.** A new `Cargo.toml` or `package.json` dependency requires a Decision Log entry or stop-and-ask.
- **`// @ts-ignore` / `#[allow(clippy::…)]` blanket suppressions.** Fix the root cause or document why it's correct in a comment.
- **Disabling a test instead of fixing it.** File an issue and link from `#[ignore]`.
- **Mixing crate concerns in one commit.** `fineliner-effects` and `fineliner-core` changes in the same commit are forbidden — separate concerns, separate commits.
- **Effects applied to composite.** Effects always target a specific layer's pixel data, never the flattened composite. This is a fundamental architecture invariant.
- **Floating point color values outside [0.0, 1.0] stored in Document.** Clamp before storage; allow intermediate OOB only inside effect pipelines.

---

## 10. Common Commands

All commands run from repo root unless noted.

### Rust workspace

```sh
cargo check                                   # all crates
cargo check -p fineliner-core                 # one crate
cargo test -p fineliner-core
cargo test -p fineliner-effects
cargo clippy --workspace -- -D warnings
cargo fmt
cargo bench -p fineliner-effects              # run benchmarks
cargo doc --workspace --no-deps --open
```

### WASM

```sh
cd crates/fineliner-wasm
wasm-pack build --target web --release        # produces pkg/
```

### UI

```sh
cd ui
pnpm install
pnpm dev                                      # Vite dev server (WASM hot-reload)
pnpm build                                    # production bundle
pnpm test                                     # Vitest unit tests
pnpm test:e2e                                 # Playwright (Phase 3)
pnpm lint
pnpm check                                    # svelte-check TypeScript
```

### Tauri

```sh
pnpm tauri dev                                # native dev (from repo root)
pnpm tauri build                              # release binary
```

### Cloudflare / PWA

```sh
pnpm build
wrangler pages deploy ui/dist                 # or via CI
```

### Full pre-commit gate

```sh
cargo fmt && \
cargo clippy --workspace -- -D warnings && \
cargo test --workspace && \
cd ui && pnpm lint && pnpm check && pnpm test
```

All steps must pass. Do not commit if any step fails.

---

## 11. Definition of Done

A task is done when **all** of the following hold:

- [ ] Code change is minimal and matches the spec
- [ ] `cargo check` and `cargo test` pass for affected crates
- [ ] `cargo clippy -- -D warnings` is clean
- [ ] New public API has doc comments
- [ ] New behavior has at least one test
- [ ] Spec references updated if design shifted
- [ ] Commit message follows convention
- [ ] No forbidden patterns introduced (§9)

For UI tasks add:

- [ ] `pnpm lint` and `pnpm check` clean
- [ ] `pnpm build` succeeds
- [ ] Visually verified in dev server
- [ ] Pointer events used (not mouse events)

For milestone-level work add:

- [ ] Exit criterion in spec §16 is met
- [ ] `STATUS.md` updated
- [ ] Screenshot or recording in PR description

---

## 12. When to Stop and Ask

**It is always better to stop than to guess.**

Stop and ask when:

- The spec is silent on a question affecting the public API
- Two reasonable implementations exist and the choice is not local
- A new dependency is needed
- The task would take more than one session
- A test fails for a reason suggesting the spec is wrong
- Performance work requires cross-crate architectural changes
- A color math decision would affect multiple effects (pick once, propagate)

Do not stop for:

- Style choices that don't affect API
- Local refactors improving clarity
- Adding tests beyond the requirement
- Documentation improvements
- Renames within a single module

---

## 13. Architecture Decisions (Log)

Record decisions here as they are made. Format: `ADR-NNN: <title> — <date> — <decision> — <rationale>`.

```
ADR-001: Document state lives in Rust, canvas is render-only — 2026-05
  Decision: All pixel data is owned by fineliner-core. Canvas2D is write-only from JS.
  Rationale: readPixels() from GPU canvas is slow and introduces stale-state bugs.
             Rust owns the source of truth; JS renders what Rust says.

ADR-002: Effects are stateless pure functions — 2026-05
  Decision: fineliner-effects functions take &ImageBuffer, return ImageBuffer.
  Rationale: Enables easy preview (apply to downscaled copy), parallelism,
             and testing without document setup.

ADR-003: Phase 1 text is rasterized, no vector layer — 2026-05
  Decision: Text tool renders to pixels on commit. No SVG or vector layer type.
  Rationale: Reduces scope significantly. Vector layers are a Phase 2 item.

ADR-004: .fln project format is zstd-compressed MessagePack — 2026-05
  Decision: Use rmp-serde for serialization, zstd for compression.
  Rationale: Compact binary, fast, no external schema, easy to version.
             PNG embedding of preview thumbnail deferred to Phase 2.

ADR-005: M1–M5 foundational dependencies — 2026-05
  Decision: fineliner-core depends on uuid (v4+serde), thiserror, image
            (png/jpeg/webp/bmp/gif/tiff), serde, serde_json; dev-deps insta,
            proptest. fineliner-wasm depends on wasm-bindgen, js-sys, web-sys,
            console_error_panic_hook, serde-wasm-bindgen.
  Rationale: All are mandated by the spec (§13 codecs, §17 WASM API) and
             CLAUDE.md (§5, §7). The `image` crate is the single codec dep
             (CLAUDE.md M4: "image crate only, no system deps").

ADR-006: Core does not read the system clock — 2026-05
  Decision: DocumentMetadata timestamps default to 0; the platform layer
            (wasm/Tauri) sets created_at/modified_at.
  Rationale: std::time::SystemTime::now() panics on wasm32-unknown-unknown.
             Keeping core clock-free preserves the "no platform deps" rule
             (CLAUDE.md §5.1) and wasm-compatibility.

ADR-007: WebP export is lossless in Phase 1 — 2026-05
  Decision: encode_webp produces lossless WebP. The spec §13.2 asks for lossy
            WebP (quality 1–100), but the pure-Rust `image` crate only encodes
            lossless WebP, and CLAUDE.md forbids system deps (libwebp).
  Rationale: Avoids a system dependency. Lossy WebP export is deferred; revisit
            if a pure-Rust lossy encoder becomes available. PNG/JPEG cover the
            lossy/lossless export needs for the M5 demo.
```

---

## 14. Skills Directory

Project-specific skills live under `.claude/skills/`. Add when you've implemented the second instance of a pattern.

Planned skills:

- `tool-impl` — recipe for adding a new tool (Tool trait impl, command emission, UI binding, shortcut, test)
- `effect-impl` — recipe for adding a new effect (params struct, apply fn, preview fn, bench, test)
- `adjustment-impl` — recipe for adding a new adjustment (subset of effect-impl, simpler)
- `command-pattern` — recipe for a new command (data shape, apply/revert, merge logic, undo test)
- `wasm-binding` — recipe for exposing a new fineliner-core API to JS

---

## 15. Quick Reference: Where Things Live

```
docs/specs/fineliner.md         The spec — what to build
CLAUDE.md                       This file — how to build it
STATUS.md                       Current session state, next task
.claude/skills/                 Project-specific skill recipes

crates/fineliner-core/          Pure logic, no I/O, no platform
crates/fineliner-effects/       Stateless image effects + adjustments
crates/fineliner-wasm/          wasm-bindgen layer, cdylib

ui/                             Svelte 5 + Vite frontend (PWA)
ui/src/lib/core/                WASM adapter (single import point)
ui/src/lib/render/              Canvas2D and WebGPU renderers
ui/src/lib/tools/               Pointer event → tool command handlers
ui/src/lib/stores/              Svelte 5 runes state
ui/src/lib/components/          Svelte components

src-tauri/                      Native shell (Tauri 2)

website/                        Landing page (separate from app)
wrangler.toml                   Cloudflare Pages deployment config
```

---

*Last updated: 2026-05. Amend in place via PR. Significant changes get a Decision Log entry (§13).*
