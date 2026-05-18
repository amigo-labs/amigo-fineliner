# fineliner.md — Fineliner Design Specification

This document is the source of truth for **what** Fineliner builds. It is read by Claude Code before every implementation task. The companion file `CLAUDE.md` covers **how** to build it.

All implementation decisions that are not covered here must be resolved by adding a Decision Log entry (§15) before writing code.

---

## Table of Contents

1. [Vision & Scope](#1-vision--scope)
2. [Target Users](#2-target-users)
3. [Document Model](#3-document-model)
4. [Color System](#4-color-system)
5. [Layer System](#5-layer-system)
6. [Canvas & Rendering](#6-canvas--rendering)
7. [Command & History System](#7-command--history-system)
8. [Selection System](#8-selection-system)
9. [Tool Definitions](#9-tool-definitions)
10. [Transform Operations](#10-transform-operations)
11. [Effects Pipeline](#11-effects-pipeline)
12. [Adjustments](#12-adjustments)
13. [Image I/O & File Formats](#13-image-io--file-formats)
14. [Project Format (.fln)](#14-project-format-fln)
15. [Keyboard Shortcuts](#15-keyboard-shortcuts)
16. [UI Layout](#16-ui-layout)
17. [WASM API Surface](#17-wasm-api-surface)
18. [Milestone Exit Criteria](#18-milestone-exit-criteria)
19. [Decision Log](#19-decision-log)

---

## 1. Vision & Scope

### 1.1 Mission

Fineliner is a **simple, fast, cross-platform image editor** for everyday tasks: retouching photos, compositing UI assets, creating illustrations, and editing screenshots. It occupies the space between MS Paint (too limited) and Photoshop/GIMP (too complex).

Primary inspiration: **paint.net** — its simplicity, speed, and approachable UX. Fineliner's differentiators are:

- Web-first: runs in the browser with no install
- Native app via Tauri (Windows, macOS, Linux) from the same codebase
- Modern UI without GTK or Electron overhead
- Clean Rust core for correctness and performance

### 1.2 Non-Goals (Phase 1)

The following are explicitly out of scope for Phase 1:

- Vector layers or SVG editing
- RAW photo format support (CR2, ARW, NEF, etc.)
- Animation / timeline
- Non-destructive adjustment layers (adjustments are destructive in Phase 1)
- Collaborative / multiplayer editing
- Plugin system
- CMYK color mode
- 16-bit or 32-bit per channel editing
- Batch processing
- Scripting / macros

---

## 2. Target Users

**Primary:** Digital creators doing quick image tasks — cropping screenshots, annotating images, compositing assets, basic photo retouching. They want something that "just works" without a learning curve.

**Secondary:** Web and UI designers editing assets for products. They value precision tools (selection, transform) and lossless export.

**Not targeted:** Professional photographers needing RAW workflow, motion designers, print production artists.

---

## 3. Document Model

### 3.1 Document

A `Document` represents one open image file. It contains:

```
Document {
  id:           Uuid,
  title:        String,                  // filename or "Untitled N"
  canvas:       CanvasSize,              // width × height in pixels
  layers:       Vec<Layer>,              // ordered bottom to top
  active_layer: usize,                   // index into layers
  selection:    Option<SelectionMask>,   // global selection, None = all selected
  history:      UndoStack,
  metadata:     DocumentMetadata,
}

CanvasSize {
  width:  u32,   // 1–32767
  height: u32,   // 1–32767
}

DocumentMetadata {
  dpi:           f32,           // default 96.0
  color_profile: ColorProfile,  // sRGB only in Phase 1
  created_at:    u64,           // Unix timestamp
  modified_at:   u64,
}
```

### 3.2 Constraints

- Maximum canvas size: 32767 × 32767 pixels
- Minimum canvas size: 1 × 1 pixel
- Maximum layers: 999
- Maximum undo history: 100 steps (configurable in preferences, 10–500)
- A document always has at least one layer

---

## 4. Color System

### 4.1 Color Representation

All pixel data is stored as **RGBA8** (4 bytes per pixel, u8 each, premultiplied alpha is **not** used in storage — straight alpha only).

Internal computation (effects, blending) uses **RGBA32f** (4 × f32, range [0.0, 1.0]). Convert on entry and exit.

```
Color {
  r: u8,
  g: u8,
  b: u8,
  a: u8,
}
```

### 4.2 Active Colors

The editor maintains two active colors at all times:

- **Foreground color** (primary) — used by pencil, shapes, fill, text
- **Background color** (secondary) — used for canvas background on new document, eraser background mode

Both colors are stored as RGBA8. Default: foreground = black `(0,0,0,255)`, background = white `(255,255,255,255)`.

Swap foreground ↔ background: `X` key.
Reset to black/white defaults: `D` key.

### 4.3 Color Picker

Supports:
- HSV wheel + SV square
- RGB sliders (0–255)
- Hex input (#RRGGBB, #RRGGBBAA)
- Opacity (alpha) slider
- Recent colors (last 16, persisted in preferences)

### 4.4 Blend Modes

Blend modes are applied per-layer during composite. All math is in linear light (linearize sRGB input, apply mode, gamma-encode output).

Supported blend modes:

| ID | Name | Formula (src over dst) |
|---|---|---|
| `Normal` | Normal | src_a × src + (1 − src_a) × dst |
| `Multiply` | Multiply | src × dst |
| `Screen` | Screen | 1 − (1 − src)(1 − dst) |
| `Overlay` | Overlay | Hard Light with src/dst swapped |
| `Darken` | Darken | min(src, dst) |
| `Lighten` | Lighten | max(src, dst) |
| `ColorDodge` | Color Dodge | dst / (1 − src) |
| `ColorBurn` | Color Burn | 1 − (1 − dst) / src |
| `HardLight` | Hard Light | Overlay with src/dst swapped |
| `SoftLight` | Soft Light | Pegtop formula |
| `Difference` | Difference | abs(src − dst) |
| `Exclusion` | Exclusion | src + dst − 2 × src × dst |

All blend mode math references: https://www.w3.org/TR/compositing-1/

---

## 5. Layer System

### 5.1 Layer

```
Layer {
  id:         Uuid,
  name:       String,            // user-editable, default "Layer N"
  kind:       LayerKind,
  pixels:     ImageBuffer,       // RGBA8, same dimensions as canvas
  opacity:    f32,               // 0.0–1.0, default 1.0
  blend_mode: BlendMode,         // default Normal
  visible:    bool,              // default true
  locked:     bool,              // default false — prevents pixel edits
  clip:       bool,              // clipping mask to layer below (Phase 2)
}

LayerKind {
  Raster,   // standard pixel layer (Phase 1 only kind)
}
```

### 5.2 Layer Operations

All operations are Commands (undoable):

| Operation | Description |
|---|---|
| Add layer | Insert new transparent layer above active |
| Delete layer | Remove layer (blocked if only layer) |
| Duplicate | Copy layer above original |
| Merge down | Flatten layer onto layer below |
| Merge visible | Flatten all visible layers into one |
| Flatten image | Merge all layers onto white background |
| Move up / down | Change layer order |
| Rename | Edit layer name |
| Set opacity | Change opacity (live preview) |
| Set blend mode | Change blend mode (live preview) |
| Toggle visible | Show / hide layer |
| Toggle locked | Lock / unlock pixel editing |

### 5.3 Thumbnail

Each layer maintains a 32 × 32 thumbnail updated after each command that modifies pixels. The thumbnail uses the layer's actual pixels (not composited).

---

## 6. Canvas & Rendering

### 6.1 Composite

The final image displayed is the **composite** of all visible layers composited bottom-to-top using each layer's blend mode and opacity. The canvas background is transparent (checkerboard pattern shown in UI).

`compose(layers: &[Layer]) -> ImageBuffer` — pure function, takes visible layers only.

### 6.2 Dirty-Rect Tracking (M16)

Phase 1: full composite on every change.
M16: track which pixel region was modified by the last command; recomposite only the dirty rect plus a 1-pixel border.

### 6.3 Coordinate System

- Origin (0, 0) is top-left
- X increases rightward, Y increases downward
- Pixel (x, y) occupies the unit square from (x, y) to (x+1, y+1)
- Canvas coordinates are integers; tool input coordinates are f32 (sub-pixel precision for anti-aliasing)

### 6.4 Zoom & Pan

| Level | Description |
|---|---|
| Fit to window | Default on open |
| 1:1 (100%) | `Ctrl+0` or `Numpad 0` |
| Zoom in | `Ctrl++` or `Ctrl+scroll up` |
| Zoom out | `Ctrl+-` or `Ctrl+scroll down` |
| Min zoom | 1% |
| Max zoom | 6400% |

Pan: Space + drag, or middle mouse button drag.

At zoom ≥ 800%, show pixel grid (1px lines between pixels, color `rgba(128,128,128,0.4)`).

### 6.5 Canvas Background

The transparent canvas area is rendered with a checkerboard pattern: 8×8 px squares alternating `#808080` and `#a0a0a0` (customizable in preferences).

---

## 7. Command & History System

### 7.1 Command Trait

```rust
trait Command: Send + Sync {
  fn apply(&mut self, doc: &mut Document) -> Result<(), DocumentError>;
  fn revert(&mut self, doc: &mut Document) -> Result<(), DocumentError>;
  fn label(&self) -> &str;       // shown in history panel, e.g. "Pencil Stroke"
  fn merge_with(&mut self, _newer: &dyn Command) -> bool { false }
}
```

`merge_with` returns true if `newer` was absorbed into `self` (used for stroke merging — multiple `SetPixels` during one pointer drag merge into one undo step).

### 7.2 Undo Stack

Linear undo stack. No tree-view in Phase 1 (differs from rs-paint; tree-view is Phase 2).

- `Ctrl+Z` → undo
- `Ctrl+Y` or `Ctrl+Shift+Z` → redo
- Capacity: configurable (default 100, range 10–500)
- New command after undo → redo branch is discarded
- History panel shows command labels in order; click to navigate (Phase 2)

### 7.3 Commands Reference

Complete list of commands. Each is a Rust struct implementing `Command`.

**Document commands:**
`ResizeCanvas`, `ScaleImage`, `CropToSelection`, `FlattenImage`, `MergeVisible`

**Layer commands:**
`AddLayer`, `DeleteLayer`, `DuplicateLayer`, `MoveLayer`, `RenameLayer`,
`SetLayerOpacity`, `SetLayerBlendMode`, `SetLayerVisible`, `SetLayerLocked`,
`MergeDown`

**Pixel commands:**
`SetPixels(region: Rect, before: ImageBuffer, after: ImageBuffer)` — stores before/after for undo.
All drawing tools emit this command. The `before` buffer is captured lazily (only when first applying).

**Selection commands:**
`SetSelection(before: Option<Mask>, after: Option<Mask>)`

**Transform commands:**
`TransformLayer(layer_id, transform: AffineTransform, interpolation: Interpolation)`

---

## 8. Selection System

### 8.1 Selection Mask

A selection is a grayscale mask the same dimensions as the canvas:
- `255` = fully selected
- `0` = not selected
- 1–254 = partial selection (used by feathering and anti-aliasing)

When no selection is active (`None`), all pixels are considered fully selected.

### 8.2 Selection Modes

When creating or modifying a selection, four modes apply:

| Mode | Key modifier | Behavior |
|---|---|---|
| Replace | (none) | New selection replaces old |
| Add | Shift | Union with existing |
| Subtract | Alt | Remove from existing |
| Intersect | Shift+Alt | Keep only overlap |

### 8.3 Selection Tools

Defined fully in §9. Tools: Rectangle Select, Ellipse Select, Lasso, Polygonal Lasso, Magic Wand.

### 8.4 Selection Modifiers (menu / keyboard)

| Action | Shortcut | Description |
|---|---|---|
| Select All | `Ctrl+A` | Set all pixels to 255 |
| Deselect | `Ctrl+D` | Set selection to None |
| Invert | `Ctrl+Shift+I` | 255−x for all pixels |
| Expand | — | Grow by N pixels (integer) |
| Contract | — | Shrink by N pixels |
| Feather | — | Gaussian blur the mask edges by N pixels |
| Select by Color | — | Like Magic Wand but whole canvas |

### 8.5 Marching Ants

The selection outline is displayed as an animated dashed line (CSS animation, 8px dash / 8px gap, animates at 1 cycle/second). Rendered in the canvas overlay layer, not in pixel data.

---

## 9. Tool Definitions

### 9.1 Tool Trait

```rust
trait Tool {
  fn on_pointer_down(&mut self, pos: Point, mods: Modifiers, doc: &Document) -> Vec<Box<dyn Command>>;
  fn on_pointer_move(&mut self, pos: Point, mods: Modifiers, doc: &Document) -> Vec<Box<dyn Command>>;
  fn on_pointer_up  (&mut self, pos: Point, mods: Modifiers, doc: &Document) -> Vec<Box<dyn Command>>;
  fn cursor(&self) -> CursorKind;
  fn options(&self) -> ToolOptions;
}
```

Tools emit zero or more commands per pointer event. The app applies commands immediately and adds them to history.

### 9.2 Tool List

#### Pencil (`B`)

Freehand pixel painting.

Options:
- **Brush size:** 1–500 px
- **Brush shape:** Hard Round, Soft Round, Flat (45°), custom (Phase 2)
- **Opacity:** 1–100%
- **Hardness:** 0–100% (only for Soft Round and Flat)
- **Blend mode:** any blend mode from §4.4 (defaults to Normal)
- **Anti-alias:** on/off

Behavior:
- Left button: paint with foreground color
- Right button: paint with background color
- Shift+click: draw straight line from last point to current point
- Pointer pressure (stylus): modulates opacity if `use_pressure` is on

Emits: `SetPixels` per stroke segment. Multiple segments within one pointer-down/up cycle are merged into one undo step via `merge_with`.

#### Eraser (`E`)

Clears pixels to transparent (alpha = 0) or to background color (mode option).

Options:
- **Mode:** To Transparent / To Background Color
- **Size:** 1–500 px
- **Hardness:** 0–100%
- **Opacity:** 1–100%

Emits: `SetPixels`.

#### Fill / Paint Bucket (`G`)

Flood-fills a region with the foreground color.

Options:
- **Tolerance:** 0–255 (color similarity threshold)
- **Contiguous:** on = fill connected region, off = fill all matching pixels in layer
- **Sample:** Current Layer / All Layers (for color sampling only; always fills current layer)
- **Anti-alias:** on/off (Phase 2)

Algorithm: BFS flood-fill. Tolerance compares Euclidean distance in RGBA8 space.

Emits: `SetPixels`.

#### Eyedropper (`I`)

Samples a color from the canvas and sets it as the active color.

Options:
- **Sample:** Current Layer / Composite
- **Size:** 1×1 / 3×3 avg / 5×5 avg / 11×11 avg / 31×31 avg

Left click: set foreground color.
Right click: set background color.
Hold `Alt` while any drawing tool is active to temporarily activate Eyedropper.

Emits: no command (color change is not undoable).

#### Move (`V`)

Translates the content of the active layer.

Options:
- **Auto-select:** if on, clicking selects the topmost non-transparent layer under cursor

Behavior:
- Drag: translate layer pixels
- Arrow keys: nudge 1 px; Shift+arrow = nudge 10 px
- Ghost preview shown during drag

Emits: `SetPixels` on pointer up (after computing translated pixels).

#### Rectangle Select (`M`)

Draws a rectangular selection.

Options:
- **Mode:** Replace / Add / Subtract / Intersect
- **Feather:** 0–250 px
- **Fixed ratio:** free / square / custom W:H
- **Fixed size:** free / custom W×H px

Shift+drag: constrain to square.

Emits: `SetSelection`.

#### Ellipse Select (`M` twice or long-press)

Same as Rectangle Select but elliptical.

Options: identical to Rectangle Select.

Shift+drag: constrain to circle.

Emits: `SetSelection`.

#### Lasso (`L`)

Freehand selection by dragging. Closing the path (releasing pointer) connects last point to first.

Options:
- **Mode:** Replace / Add / Subtract / Intersect
- **Feather:** 0–250 px
- **Anti-alias:** on/off

Emits: `SetSelection`.

#### Polygonal Lasso (`L` twice)

Click-by-click polygon selection. Double-click or click on start point to close.

Options: same as Lasso.

Emits: `SetSelection`.

#### Magic Wand (`W`)

Selects a region of similar color by clicking.

Options:
- **Tolerance:** 0–255
- **Contiguous:** on/off
- **Sample:** Current Layer / All Layers
- **Anti-alias:** on/off
- **Mode:** Replace / Add / Subtract / Intersect

Emits: `SetSelection`.

#### Gradient (`G` shift cycle)

Draws a gradient fill on the current layer (within selection if active).

Options:
- **Type:** Linear / Radial / Angle / Reflected / Diamond
- **Colors:** Foreground→Background / Foreground→Transparent / Background→Foreground / Custom (two color pickers)
- **Repeat:** None / Forward / Reverse / Mirror
- **Opacity:** 1–100%
- **Blend mode:** any

Drag defines start and end points. Preview shown live during drag.

Emits: `SetPixels`.

#### Clone Stamp (`S`)

Paints pixels sampled from another area.

Options:
- **Size:** 1–500 px
- **Hardness:** 0–100%
- **Opacity:** 1–100%
- **Aligned:** on = sample point follows stroke offset; off = always sample from original point
- **Sample:** Current Layer / All Layers

Set sample point: `Alt+click`.
Must set sample point before first stroke; if not set, tool shows warning cursor.

Emits: `SetPixels`.

#### Smudge (`R`)

Smears pixel colors in the direction of the stroke.

Options:
- **Size:** 1–500 px
- **Strength:** 1–100%
- **Hardness:** 0–100%

Emits: `SetPixels`.

#### Blur Brush (`R` shift cycle)

Locally blurs pixels under the brush.

Options:
- **Size:** 1–500 px
- **Strength:** 1–100%

Emits: `SetPixels`.

#### Sharpen Brush (`R` shift cycle)

Locally sharpens pixels under the brush.

Options: same as Blur Brush.

Emits: `SetPixels`.

#### Shapes (`U`)

Draws geometric shapes.

Shapes available (cycle with `U`): Line, Rectangle, Rounded Rectangle, Ellipse, Polygon (N-sided), Arrow.

Options:
- **Mode:** Outline / Fill / Fill + Outline
- **Stroke width:** 1–500 px
- **Stroke color:** foreground / background / custom
- **Fill color:** foreground / background / custom / transparent
- **Dash pattern:** solid / dashed / dotted
- **Corner radius** (Rounded Rectangle): 0–999 px
- **Sides** (Polygon): 3–100
- **Anti-alias:** on/off

Shift+drag: constrain proportions (square, circle, 45° line, equilateral polygon).

Emits: `SetPixels` on pointer up.

#### Text (`T`)

Rasterizes text onto the current layer.

Options:
- **Font family:** system fonts + web-safe fallbacks
- **Font size:** 6–999 pt
- **Bold / Italic / Underline**
- **Color:** foreground color at time of placement
- **Anti-alias:** on/off (None / ClearType-style / Standard)
- **Alignment:** left / center / right

Behavior:
- Click to place a text cursor; type text; text is live-previewed on canvas
- Click elsewhere or press `Esc` to commit (rasterize to pixels)
- Press `Enter` for new line
- Text is committed as `SetPixels`; it cannot be re-edited after commit

Emits: `SetPixels` on commit.

---

## 10. Transform Operations

All transform operations act on the **active layer** unless otherwise noted. They are accessed via the Image and Layer menus, and via the Free Transform tool.

### 10.1 Free Transform (`Ctrl+T`)

Shows a bounding box around the layer content with handles:
- **Corner handles:** scale (hold Shift to constrain aspect ratio)
- **Edge handles:** scale one axis
- **Rotate handle** (above top-center): rotate (hold Shift to snap to 15° increments)
- **Inside box:** drag to translate

Press `Enter` or double-click inside: apply.
Press `Esc`: cancel.

Interpolation: Bilinear (default) / Nearest Neighbor (when pixel-art mode is on in preferences).

Emits: `TransformLayer`.

### 10.2 Flip

| Action | Menu | Shortcut |
|---|---|---|
| Flip layer horizontal | Layer > Flip Horizontal | — |
| Flip layer vertical | Layer > Flip Vertical | — |
| Flip canvas horizontal | Image > Flip Horizontal | — |
| Flip canvas vertical | Image > Flip Vertical | — |

Canvas flip affects all layers.

Emits: `TransformLayer` (layer flip) or multiple `TransformLayer` (canvas flip).

### 10.3 Rotate

| Action |
|---|
| Rotate layer 90° CW |
| Rotate layer 90° CCW |
| Rotate layer 180° |
| Rotate canvas 90° CW |
| Rotate canvas 90° CCW |
| Rotate canvas 180° |
| Rotate canvas arbitrary angle (dialog, –180° to 180°, 0.01° precision) |

Canvas rotation at arbitrary angles also resizes the canvas to fit the rotated content (with a "crop to original size" option).

### 10.4 Resize Canvas

Dialog: width, height, anchor point (9-position grid), background fill (transparent / background color).

Does not scale layer content. Layers smaller than new canvas are padded with transparency.

### 10.5 Scale Image

Dialog: width, height, constrain proportions checkbox, interpolation (Nearest / Bilinear / Bicubic), resample DPI.

All layers are scaled together.

### 10.6 Crop

**Crop to selection:** resizes canvas to the bounding rect of the current selection. Pixels outside selection are not clipped (they remain but are outside the canvas).

**Crop tool** (Phase 2): interactive drag-to-crop.

---

## 11. Effects Pipeline

Effects are accessed via the **Effects** menu. They are applied **destructively** to the current layer's pixels in Phase 1.

Each effect opens a dialog with:
- Parameter controls (sliders, number inputs)
- Live preview on canvas (300ms debounce)
- OK / Cancel buttons

### 11.1 Blur

#### Gaussian Blur
- **Radius:** 0.1–250 px (f32)
- Algorithm: separable Gaussian kernel, σ = radius / 3

#### Box Blur
- **Width:** 1–500 px (odd only)
- **Height:** 1–500 px (odd only)

#### Motion Blur
- **Distance:** 1–500 px
- **Angle:** 0–360°

#### Radial Blur
- **Amount:** 1–100
- **Center:** x, y (default canvas center)
- **Type:** Spin / Zoom

### 11.2 Sharpen

#### Unsharp Mask
- **Amount:** 1–500%
- **Radius:** 0.1–250 px
- **Threshold:** 0–255

#### Sharpen
Single-step convolution sharpen. No parameters. Equivalent to Unsharp Mask with fixed preset.

### 11.3 Distort

#### Emboss
- **Angle:** 0–360°
- **Elevation:** 0–90°
- **Relief:** 1–10

#### Edge Detect
- **Algorithm:** Sobel / Prewitt / Laplacian
- **Amount:** 0–100%

### 11.4 Noise

#### Add Noise
- **Amount:** 0–100%
- **Type:** Uniform / Gaussian
- **Channels:** RGB / Monochromatic
- **Seed:** integer (for reproducibility)

#### Reduce Noise (Median Filter)
- **Radius:** 1–10 px

---

## 12. Adjustments

Adjustments are accessed via the **Adjustments** menu. They open a dialog and apply destructively to the current layer.

### 12.1 Brightness / Contrast
- **Brightness:** −150 to +150
- **Contrast:** −150 to +150
- Mode: Legacy (linear) / Enhanced (S-curve)

### 12.2 Hue / Saturation / Lightness
- **Hue:** −180 to +180°
- **Saturation:** −100 to +100
- **Lightness:** −100 to +100
- **Colorize mode:** on/off (replaces hue with single hue for all pixels)

### 12.3 Curves
Per-channel curves editor.

- Channels: Composite (RGB), Red, Green, Blue, Alpha
- Curve type: Cubic spline with up to 16 control points
- Input / output range: 0–255
- Presets: None, Increase Contrast, Decrease Contrast, Lighten, Darken, S-Curve

### 12.4 Levels
- **Input:** Black point (0–253), Gamma (0.01–9.99), White point (2–255)
- **Output:** Black point (0–254), White point (1–255)
- Per-channel mode: composite or individual R/G/B
- Auto button: sets black/white points to darkest/brightest pixels (ignoring top/bottom 0.5%)

### 12.5 Color Balance
- Three tone ranges: Shadows, Midtones, Highlights
- Per range: Cyan↔Red, Magenta↔Green, Yellow↔Blue (each −100 to +100)
- Preserve Luminosity: on/off

### 12.6 Invert
No parameters. `Ctrl+I`. Each channel: 255 − x (alpha preserved).

### 12.7 Grayscale
- **Method:** Luminosity (default) / Average / BT.709 / Channel Mixer
- Channel mixer weights (if chosen): R, G, B weights summing to 1.0

### 12.8 Posterize
- **Levels:** 2–255

### 12.9 Threshold
- **Threshold:** 0–255
- Pixels with luminance above threshold → white; below → black. Alpha preserved.

---

## 13. Image I/O & File Formats

### 13.1 Import (Open)

| Format | Extension | Notes |
|---|---|---|
| PNG | `.png` | Full RGBA support, strips metadata |
| JPEG | `.jpg`, `.jpeg` | Converts to RGBA (no alpha in JPEG) |
| WebP | `.webp` | Lossy and lossless |
| BMP | `.bmp` | 24-bit and 32-bit |
| GIF | `.gif` | First frame only |
| TIFF | `.tif`, `.tiff` | 8-bit and 16-bit (downconverted to 8-bit) |

On import, the image is placed as a single raster layer filling the canvas.

### 13.2 Export (Save As / Export)

Export always flattens the document composite to a single image.

| Format | Quality Options |
|---|---|
| PNG | Compression 0–9 |
| JPEG | Quality 1–100 |
| WebP | Lossy (quality 1–100) / Lossless |
| BMP | None |

Export does not modify the document's "saved" state.

### 13.3 Save / Save As

Saves in `.fln` project format (§14). Preserves all layers, history is **not** saved (too large).

"Save" overwrites the current `.fln` file. "Save As" shows file picker.

The title bar shows `*` prefix when there are unsaved changes.

### 13.4 New Document

Dialog:
- **Preset:** Web (1920×1080), HD (1280×720), Square (1000×1000), A4 @ 96dpi, Custom
- **Width:** 1–32767
- **Height:** 1–32767
- **Background:** Transparent / White / Black / Foreground Color / Background Color
- **DPI:** 72 / 96 / 150 / 300 / Custom

### 13.5 Clipboard

| Action | Shortcut | Behavior |
|---|---|---|
| Copy | `Ctrl+C` | Copy selection (or full layer if no selection) to clipboard as PNG |
| Copy Merged | `Ctrl+Shift+C` | Copy composited selection to clipboard |
| Cut | `Ctrl+X` | Copy + clear selection region to transparent |
| Paste | `Ctrl+V` | Paste clipboard image as new layer, centered |
| Paste in Place | `Ctrl+Shift+V` | Paste at top-left (0,0) |

Clipboard uses the OS clipboard API (Tauri: `tauri-plugin-clipboard-manager`; Web: `navigator.clipboard`).

---

## 14. Project Format (.fln)

A `.fln` file is a binary container:

```
Header (8 bytes):
  Magic:   b"FINELINER" (9 bytes, no null)
  Version: u8  (current: 1)

Body:
  zstd-compressed MessagePack blob of DocumentData struct
```

`DocumentData` (MessagePack schema):

```
{
  "version":  1,
  "title":    string,
  "width":    u32,
  "height":   u32,
  "dpi":      f32,
  "layers": [
    {
      "id":         string (UUID),
      "name":       string,
      "opacity":    f32,
      "blend_mode": string,
      "visible":    bool,
      "locked":     bool,
      "pixels":     bytes  // raw RGBA8, width*height*4 bytes, uncompressed
                           // (outer zstd handles compression)
    },
    …
  ],
  "active_layer": u32,  // index
  "selection":    null | bytes  // grayscale mask, width*height bytes
}
```

History is NOT saved. On load, undo stack is empty.

Thumbnail: not stored in Phase 1.

Version compatibility: reader must reject files with version > 1 and display a friendly error ("This file was created with a newer version of Fineliner").

---

## 15. Keyboard Shortcuts

### Tools

| Key | Tool |
|---|---|
| `B` | Pencil |
| `E` | Eraser |
| `G` | Fill / Gradient (shift-cycle) |
| `I` | Eyedropper |
| `V` | Move |
| `M` | Rectangle Select / Ellipse Select (shift-cycle) |
| `L` | Lasso / Polygonal Lasso (shift-cycle) |
| `W` | Magic Wand |
| `S` | Clone Stamp |
| `R` | Smudge / Blur Brush / Sharpen Brush (shift-cycle) |
| `U` | Shapes (cycle with shift) |
| `T` | Text |
| `Ctrl+T` | Free Transform |
| `Alt` (hold) | Eyedropper (temporary) |
| `Space` (hold) | Pan (temporary) |

### File

| Shortcut | Action |
|---|---|
| `Ctrl+N` | New |
| `Ctrl+O` | Open |
| `Ctrl+S` | Save |
| `Ctrl+Shift+S` | Save As |
| `Ctrl+Shift+E` | Export |
| `Ctrl+W` | Close tab |

### Edit

| Shortcut | Action |
|---|---|
| `Ctrl+Z` | Undo |
| `Ctrl+Y` / `Ctrl+Shift+Z` | Redo |
| `Ctrl+X` | Cut |
| `Ctrl+C` | Copy |
| `Ctrl+Shift+C` | Copy Merged |
| `Ctrl+V` | Paste |
| `Ctrl+Shift+V` | Paste in Place |
| `Ctrl+A` | Select All |
| `Ctrl+D` | Deselect |
| `Ctrl+Shift+I` | Invert Selection |
| `Ctrl+I` | Invert Colors (Adjustment) |

### View

| Shortcut | Action |
|---|---|
| `Ctrl++` | Zoom In |
| `Ctrl+-` | Zoom Out |
| `Ctrl+0` | Zoom to Fit |
| `Ctrl+1` | Zoom 100% |
| `Tab` | Toggle UI panels (hide/show) |
| `F11` | Fullscreen (native only) |

### Colors

| Shortcut | Action |
|---|---|
| `D` | Reset to default colors (black/white) |
| `X` | Swap foreground / background |

---

## 16. UI Layout

### 16.1 Main Layout

```
┌─────────────────────────────────────────────────────────┐
│  Menu Bar (native app only)                             │
├──────────┬──────────────────────────────────┬───────────┤
│          │  Tab Bar                          │           │
│  Tool    ├──────────────────────────────────┤  Layers   │
│  Bar     │  Tool Options Bar                │  Panel    │
│  (left)  ├──────────────────────────────────┤           │
│          │                                  ├───────────┤
│          │                                  │  Colors   │
│          │         Canvas Area              │  Panel    │
│          │                                  │           │
│          │                                  ├───────────┤
│          │                                  │  History  │
│          │                                  │  Panel    │
│          ├──────────────────────────────────┤  (Phase2) │
│          │  Status Bar                      │           │
└──────────┴──────────────────────────────────┴───────────┘
```

### 16.2 Tool Bar

Vertical strip on the left. Shows tool icons; active tool is highlighted.
Tools grouped: Selection group / Drawing group / Shape group / Transform group.

### 16.3 Tool Options Bar

Horizontal bar below tabs. Shows options for the currently active tool. Updates when tool changes.

### 16.4 Canvas Area

- Infinite scroll (pan beyond canvas edges)
- Canvas centered on open
- Rulers on top and left (shown at zoom ≥ 50%, hideable)
- Canvas drop shadow to distinguish from app background

### 16.5 Layers Panel

- Scroll list of layers, bottom-to-top display order
- Each row: visibility eye icon, lock icon, thumbnail, name, blend mode dropdown, opacity slider
- Add / Delete / Duplicate / Merge Down buttons at bottom
- Drag-to-reorder

### 16.6 Colors Panel

- Two color swatches (foreground / background), click to open color picker
- Swap button between swatches
- Recent colors grid (16 swatches)
- Opacity slider (quick access to foreground opacity)

### 16.7 Tab Bar

One tab per open document. Tab shows title + unsaved indicator (`*`). Middle-click to close.

### 16.8 Status Bar

Left: current tool name, context hint (e.g. "Click to place sample point").
Center: canvas dimensions, color mode.
Right: zoom level, cursor coordinates, color under cursor (RGBA8 hex).

### 16.9 Themes

Phase 1: Dark theme only (matches amigo-pincel aesthetic). Light theme Phase 2.

Dark palette:
- App background: `#1e1e1e`
- Panel background: `#252526`
- Panel border: `#3c3c3c`
- Active tool highlight: `#0078d4`
- Canvas checkerboard: `#808080` / `#a0a0a0`

---

## 17. WASM API Surface

The `fineliner-wasm` crate exports this public API to JavaScript. Additions require a Decision Log entry.

```typescript
// Document lifecycle
export function create_document(width: number, height: number): DocumentHandle;
export function open_image(data: Uint8Array, mime_type: string): DocumentHandle;
export function close_document(handle: DocumentHandle): void;

// Composite rendering
export function composite(handle: DocumentHandle): Uint8ClampedArray;
// Returns RGBA8 flat array, width*height*4 bytes. Caller wraps in ImageData.

// Commands
export function apply_command(handle: DocumentHandle, cmd: SerializedCommand): void;
export function undo(handle: DocumentHandle): boolean;
export function redo(handle: DocumentHandle): boolean;

// Export
export function export_png(handle: DocumentHandle): Uint8Array;
export function export_jpeg(handle: DocumentHandle, quality: number): Uint8Array;
export function export_webp(handle: DocumentHandle, quality: number): Uint8Array;

// Project
export function save_project(handle: DocumentHandle): Uint8Array;
export function load_project(data: Uint8Array): DocumentHandle;

// Effects (added in M11)
export function apply_effect(handle: DocumentHandle, layer_id: string, effect: SerializedEffect): void;
export function preview_effect(handle: DocumentHandle, layer_id: string, effect: SerializedEffect, max_dim: number): Uint8ClampedArray;

// State queries
export function get_document_info(handle: DocumentHandle): DocumentInfo;
export function get_layer_thumbnail(handle: DocumentHandle, layer_id: string): Uint8ClampedArray;
```

`SerializedCommand` and `SerializedEffect` are JSON strings. `DocumentHandle` is an opaque u32 index into a Rust-side arena.

---

## 18. Milestone Exit Criteria

A milestone is complete when **all** criteria below are met and committed to `main`.

### M1 — Core skeleton
- [ ] `cargo check -p fineliner-core` passes
- [ ] `Document`, `Layer`, `Canvas`, `Color`, `BlendMode`, `Rect`, `Point` types exist with builder APIs
- [ ] Unit tests for construction, layer add/remove, canvas dimensions pass

### M2 — Commands + undo
- [ ] `Command` trait, `CommandBus`, `UndoStack` implemented
- [ ] `SetPixels`, `AddLayer`, `RemoveLayer`, `MoveLayer`, `ResizeCanvas` implemented
- [ ] Apply/revert round-trip tests pass for all 5 commands
- [ ] Undo depth, clear, and branch-on-new-command tests pass

### M3 — Rendering pipeline
- [ ] `compose()` produces correct RGBA8 output
- [ ] All 12 blend modes implemented and tested against reference values
- [ ] Opacity applied correctly per-layer
- [ ] Snapshot tests pass

### M4 — Image I/O
- [ ] PNG, JPEG, WebP, BMP read and write
- [ ] Round-trip tests pass for all formats
- [ ] Edge case tests (1×1, transparent, large) pass

### M5 — WASM + Pencil demo
- [ ] `wasm-pack build` succeeds
- [ ] Svelte app loads in browser
- [ ] Can open PNG, paint with Pencil, export PNG
- [ ] Exported PNG is pixel-correct

### M6 — Basic tool suite
- [ ] Eraser, Fill, Eyedropper, Move implemented
- [ ] Pencil brush variants (hard/soft/flat) work
- [ ] All tools have keyboard shortcuts
- [ ] Fill tolerance and eyedropper tests pass

### M7 — Layer system UI
- [ ] Layer panel shows all layers with thumbnails
- [ ] Add, delete, duplicate, reorder, rename, opacity, blend mode all work
- [ ] Merge visible and flatten pass round-trip test

### M8 — Selection tools
- [ ] Rectangle, Ellipse, Lasso, Polygonal Lasso, Magic Wand implemented
- [ ] Add/Subtract/Intersect modes work
- [ ] Expand, Contract, Feather, Invert work
- [ ] Marching ants displayed
- [ ] Selection constrains Pencil and Fill operations

### M9 — Transform tools
- [ ] Free Transform (translate, scale, rotate) works
- [ ] Flip H/V (layer and canvas) work
- [ ] Rotate 90/180 (layer and canvas) work
- [ ] Resize canvas dialog works
- [ ] Scale image dialog works
- [ ] Rotate 90° × 4 = identity test passes

### M10 — Shapes + Text
- [ ] All shape types render correctly
- [ ] Text rasterizes at correct size and font
- [ ] Text commit is undoable

### M11 — Effects crate
- [ ] `cargo check -p fineliner-effects` passes
- [ ] All blur, sharpen, distort, noise effects implemented
- [ ] Identity tests pass (σ=0, 1×1 kernel)
- [ ] Effects accessible from UI via Effects menu
- [ ] Live preview works in dialog

### M12 — Adjustments
- [ ] All 9 adjustments implemented
- [ ] Invert round-trip test passes
- [ ] Curves identity test passes
- [ ] All accessible via Adjustments menu with live preview

### M13 — Advanced tools
- [ ] Gradient (all 5 types) implemented
- [ ] Clone Stamp works with sample point
- [ ] Smudge, Blur Brush, Sharpen Brush work

### M14 — Tauri native shell
- [ ] App builds on Windows, macOS, Linux
- [ ] File open/save dialogs work
- [ ] File associations for .fln work
- [ ] Menu bar complete
- [ ] Project save/load round-trip test passes

### M15 — PWA + Cloudflare
- [ ] Service worker installs and caches app shell
- [ ] IndexedDB autosave works
- [ ] App runs fully offline
- [ ] Deployed to Cloudflare Pages

### M16 — Performance pass
- [ ] Dirty-rect compositing implemented
- [ ] WebGPU render path works (with Canvas2D fallback)
- [ ] 4K image, 10 layers composites in < 16 ms
- [ ] Criterion benchmarks checked in and passing

---

## 19. Decision Log

Format: `DL-NNN: <title> — <date> — <decision> — <rationale>`

```
DL-001: Document state lives in Rust, canvas is render-only — 2026-05
  All pixel data owned by fineliner-core. Canvas2D is write-only.
  Rationale: readPixels() is slow; Rust owns the source of truth.

DL-002: Effects are stateless pure functions — 2026-05
  fineliner-effects functions take &ImageBuffer, return ImageBuffer.
  Rationale: Easy preview (apply to downscaled copy), parallelism, test isolation.

DL-003: Phase 1 text is rasterized on commit — 2026-05
  No vector layer type. Text → pixels on Enter/Esc.
  Rationale: Reduces scope significantly. Vector layers are Phase 2.

DL-004: .fln format is zstd-compressed MessagePack — 2026-05
  rmp-serde for serialization, zstd for compression.
  Rationale: Compact binary, fast, no external schema, easy to version.

DL-005: No tree-view undo in Phase 1 — 2026-05
  Linear undo stack only. Tree-view (like rs-paint) is Phase 2.
  Rationale: Linear undo covers 95% of user needs with far less complexity.

DL-006: Blend mode math in linear light — 2026-05
  Linearize sRGB input before blend, gamma-encode on output.
  Rationale: Photoshop-compatible results. Perceptually correct compositing.

DL-007: Selection mask is RGBA8 grayscale (u8 per pixel) — 2026-05
  255 = fully selected, 0 = not selected, 1–254 = partial.
  Rationale: Uniform format with image pixels, enables feathering naturally.
```

---

*Last updated: 2026-05. Amend in place via PR with a DL entry for significant decisions.*
