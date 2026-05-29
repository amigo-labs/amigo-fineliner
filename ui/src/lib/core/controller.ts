// Application logic bridging the WASM core and reactive state. Components call
// these functions on events; no business logic lives in components (CLAUDE.md §5.4).
import {
  core,
  initCore,
  type PencilStrokeCommand,
  type EraserStrokeCommand,
  type FillBucketCommand,
  type TranslateLayerCommand,
  type Rgba,
} from './wasm';
import { editor, tool } from '../stores/editor.svelte';

/** Parses a #RRGGBB string into RGB bytes, defaulting to black on bad input. */
function hexToRgb(hex: string): [number, number, number] {
  const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex.trim());
  if (!m) {
    return [0, 0, 0];
  }
  return [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)];
}

/** Formats RGB bytes as a #RRGGBB string. */
function rgbToHex(r: number, g: number, b: number): string {
  return '#' + [r, g, b].map((v) => v.toString(16).padStart(2, '0')).join('');
}

/** Opacity as a 0–1 fraction, clamped. */
function opacity01(): number {
  return Math.min(1, Math.max(0, tool.opacity / 100));
}

/** The active foreground (or background) color as opaque RGBA. */
function activeColor(useBackground: boolean): Rgba {
  const [r, g, b] = hexToRgb(useBackground ? tool.background : tool.foreground);
  return [r, g, b, 255];
}

/** Refreshes derived document state from the core after a mutation. */
function syncInfo(): void {
  if (editor.handle === null) {
    return;
  }
  const info = core.documentInfo(editor.handle);
  editor.width = info.width;
  editor.height = info.height;
  editor.activeLayer = info.active_layer;
  editor.canUndo = info.can_undo;
  editor.canRedo = info.can_redo;
  editor.revision += 1;
}

/** Creates a blank document and makes it the active one. */
export async function newDocument(width: number, height: number): Promise<void> {
  await initCore();
  if (editor.handle !== null) {
    core.closeDocument(editor.handle);
  }
  editor.handle = core.createDocument(width, height);
  syncInfo();
}

/** Opens an encoded image file as a new single-layer document. */
export async function openFile(file: File): Promise<void> {
  await initCore();
  const bytes = new Uint8Array(await file.arrayBuffer());
  if (editor.handle !== null) {
    core.closeDocument(editor.handle);
  }
  editor.handle = core.openImage(bytes, file.type || 'image/png');
  syncInfo();
}

/** Applies a pencil stroke over the given canvas-space points.
 *
 * `strokeId` ties segments of one pointer drag together so they collapse into a
 * single undo step; a fresh id per drag keeps distinct strokes separate.
 * `useBackground` paints the background color (right-button, spec §9.2). */
export function paintStroke(
  points: Array<[number, number]>,
  strokeId: number,
  useBackground = false,
): void {
  if (editor.handle === null || points.length === 0) {
    return;
  }
  const cmd: PencilStrokeCommand = {
    type: 'pencil_stroke',
    layer: editor.activeLayer,
    size: tool.size,
    color: activeColor(useBackground),
    opacity: opacity01(),
    shape: tool.shape,
    hardness: Math.min(1, Math.max(0, tool.hardness / 100)),
    points,
    stroke_id: strokeId,
  };
  core.applyCommand(editor.handle, cmd);
  syncInfo();
}

/** Applies an eraser stroke over the given canvas-space points. */
export function eraseStroke(points: Array<[number, number]>, strokeId: number): void {
  if (editor.handle === null || points.length === 0) {
    return;
  }
  const cmd: EraserStrokeCommand = {
    type: 'eraser_stroke',
    layer: editor.activeLayer,
    size: tool.size,
    opacity: opacity01(),
    shape: tool.shape,
    hardness: Math.min(1, Math.max(0, tool.hardness / 100)),
    mode: tool.eraserMode,
    background: activeColor(false), // background-color erase target
    points,
    stroke_id: strokeId,
  };
  core.applyCommand(editor.handle, cmd);
  syncInfo();
}

/** Flood-fills from the given canvas-space point with the active color. */
export function fillAt(x: number, y: number, useBackground = false): void {
  if (editor.handle === null) {
    return;
  }
  const cmd: FillBucketCommand = {
    type: 'fill_bucket',
    layer: editor.activeLayer,
    color: activeColor(useBackground),
    opacity: opacity01(),
    tolerance: tool.tolerance,
    contiguous: tool.contiguous,
    sample: tool.fillSample,
    x,
    y,
  };
  core.applyCommand(editor.handle, cmd);
  syncInfo();
}

/** Translates the active layer's contents by `(dx, dy)` pixels. */
export function moveLayer(dx: number, dy: number): void {
  if (editor.handle === null || (dx === 0 && dy === 0)) {
    return;
  }
  const cmd: TranslateLayerCommand = {
    type: 'translate_layer',
    layer: editor.activeLayer,
    dx,
    dy,
  };
  core.applyCommand(editor.handle, cmd);
  syncInfo();
}

/** Samples a color at the given point and sets it as the active color.
 *
 * Left button sets the foreground, right button the background (spec §9.2). */
export function sampleColor(x: number, y: number, toBackground = false): void {
  if (editor.handle === null) {
    return;
  }
  const rgba = core.pickColor(editor.handle, x, y, tool.eyedropperSample, tool.sampleSize);
  if (rgba.length < 3) {
    return; // off-canvas
  }
  const hex = rgbToHex(rgba[0], rgba[1], rgba[2]);
  if (toBackground) {
    tool.background = hex;
  } else {
    tool.foreground = hex;
  }
}

/** Reads the current composite as RGBA8 for rendering. */
export function readComposite(): Uint8ClampedArray | null {
  if (editor.handle === null) {
    return null;
  }
  return core.composite(editor.handle);
}

/** Undoes the last command. */
export function undo(): void {
  if (editor.handle !== null && core.undo(editor.handle)) {
    syncInfo();
  }
}

/** Redoes the last undone command. */
export function redo(): void {
  if (editor.handle !== null && core.redo(editor.handle)) {
    syncInfo();
  }
}

/** Exports the composite as a PNG and triggers a browser download. */
export function exportPng(): void {
  if (editor.handle === null) {
    return;
  }
  const bytes = core.exportPng(editor.handle, 6);
  // Copy into a fresh ArrayBuffer so the Blob owns standalone memory.
  const blob = new Blob([bytes.slice()], { type: 'image/png' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'fineliner-export.png';
  document.body.appendChild(a);
  a.click();
  // Defer cleanup so the browser has started the download (avoids a WebKit
  // race where revoking synchronously cancels it).
  setTimeout(() => {
    a.remove();
    URL.revokeObjectURL(url);
  }, 0);
}
