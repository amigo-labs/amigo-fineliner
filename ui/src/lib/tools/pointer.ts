// Pointer-event handling for the M6 tool suite. Pointer events (not mouse
// events) are required for stylus/touch support (CLAUDE.md §9). The active tool
// (editor store) selects the behavior; all mutation goes through the controller.
import { paintStroke, eraseStroke, fillAt, moveLayer, sampleColor } from '../core/controller';
import { tool } from '../stores/editor.svelte';

/** Converts a pointer event into canvas-pixel coordinates. */
function toCanvasPoint(canvas: HTMLCanvasElement, e: PointerEvent): [number, number] {
  const rect = canvas.getBoundingClientRect();
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;
  return [(e.clientX - rect.left) * scaleX, (e.clientY - rect.top) * scaleY];
}

/**
 * Attaches the active tool's pointer behavior to a canvas. Returns a teardown.
 *
 * Pencil/Eraser paint incrementally over a drag (one undo step per drag via a
 * shared stroke id). Fill and Eyedropper act on click; Eyedropper also scrubs
 * while held. Move translates the layer once on pointer up by the drag delta.
 * A redraw callback runs after each mutation so the canvas stays live.
 */
export function attachTools(canvas: HTMLCanvasElement, redraw: () => void): () => void {
  let active = false;
  let last: [number, number] | null = null;
  let start: [number, number] | null = null;
  let useBackground = false;
  // Monotonic id per pointer drag so the core merges a drag's segments into one
  // undo step but keeps separate strokes separate.
  let nextStrokeId = 1;
  let strokeId = 0;

  const onDown = (e: PointerEvent): void => {
    // Left (0) and right (2) buttons act; right paints/samples the background.
    if (e.button !== 0 && e.button !== 2) {
      return;
    }
    active = true;
    useBackground = e.button === 2;
    const point = toCanvasPoint(canvas, e);
    last = point;
    start = point;
    strokeId = nextStrokeId++;
    canvas.setPointerCapture(e.pointerId);

    switch (tool.kind) {
      case 'pencil':
        paintStroke([point], strokeId, useBackground);
        redraw();
        break;
      case 'eraser':
        eraseStroke([point], strokeId);
        redraw();
        break;
      case 'fill':
        fillAt(point[0], point[1], useBackground);
        redraw();
        break;
      case 'eyedropper':
        sampleColor(point[0], point[1], useBackground);
        break;
      case 'move':
        // Translation is applied once on pointer up.
        break;
    }
  };

  const onMove = (e: PointerEvent): void => {
    if (!active || !last) {
      return;
    }
    const point = toCanvasPoint(canvas, e);
    switch (tool.kind) {
      case 'pencil':
        paintStroke([last, point], strokeId, useBackground);
        redraw();
        break;
      case 'eraser':
        eraseStroke([last, point], strokeId);
        redraw();
        break;
      case 'eyedropper':
        sampleColor(point[0], point[1], useBackground);
        break;
      case 'fill':
      case 'move':
        break;
    }
    last = point;
  };

  const onUp = (e: PointerEvent): void => {
    if (!active) {
      return;
    }
    if (tool.kind === 'move' && start && last) {
      const dx = Math.round(last[0] - start[0]);
      const dy = Math.round(last[1] - start[1]);
      moveLayer(dx, dy);
      redraw();
    }
    active = false;
    last = null;
    start = null;
    if (canvas.hasPointerCapture(e.pointerId)) {
      canvas.releasePointerCapture(e.pointerId);
    }
  };

  // Suppress the context menu so right-button paint/sample works.
  const onContextMenu = (e: Event): void => e.preventDefault();

  canvas.addEventListener('pointerdown', onDown);
  canvas.addEventListener('pointermove', onMove);
  canvas.addEventListener('pointerup', onUp);
  canvas.addEventListener('pointercancel', onUp);
  canvas.addEventListener('contextmenu', onContextMenu);

  return () => {
    canvas.removeEventListener('pointerdown', onDown);
    canvas.removeEventListener('pointermove', onMove);
    canvas.removeEventListener('pointerup', onUp);
    canvas.removeEventListener('pointercancel', onUp);
    canvas.removeEventListener('contextmenu', onContextMenu);
  };
}
