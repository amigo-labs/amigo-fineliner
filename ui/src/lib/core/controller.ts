// Application logic bridging the WASM core and reactive state. Components call
// these functions on events; no business logic lives in components (CLAUDE.md §5.4).
import { core, initCore, type PencilStrokeCommand } from './wasm';
import { editor, tool } from '../stores/editor.svelte';

/** Parses a #RRGGBB string into RGB bytes, defaulting to black on bad input. */
function hexToRgb(hex: string): [number, number, number] {
  const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex.trim());
  if (!m) {
    return [0, 0, 0];
  }
  return [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)];
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
 * single undo step; a fresh id per drag keeps distinct strokes separate. */
export function paintStroke(points: Array<[number, number]>, strokeId: number): void {
  if (editor.handle === null || points.length === 0) {
    return;
  }
  const [r, g, b] = hexToRgb(tool.foreground);
  const cmd: PencilStrokeCommand = {
    type: 'pencil_stroke',
    layer: editor.activeLayer,
    size: tool.size,
    color: [r, g, b, 255],
    opacity: Math.min(1, Math.max(0, tool.opacity / 100)),
    points,
    stroke_id: strokeId,
  };
  core.applyCommand(editor.handle, cmd);
  syncInfo();
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
