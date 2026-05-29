// Pointer-event handling for the Pencil tool. Pointer events (not mouse events)
// are required for stylus/touch support (CLAUDE.md §9).
import { paintStroke } from '../core/controller';

/** Converts a pointer event into canvas-pixel coordinates. */
function toCanvasPoint(canvas: HTMLCanvasElement, e: PointerEvent): [number, number] {
  const rect = canvas.getBoundingClientRect();
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;
  return [(e.clientX - rect.left) * scaleX, (e.clientY - rect.top) * scaleY];
}

/**
 * Attaches Pencil painting to a canvas. Returns a teardown function.
 *
 * Each drag paints incrementally: every move sends the segment from the last
 * point to the current one, which the core merges into a single undo step. A
 * redraw callback runs after each stroke segment so painting feels live.
 */
export function attachPencil(canvas: HTMLCanvasElement, redraw: () => void): () => void {
  let drawing = false;
  let last: [number, number] | null = null;
  // Monotonic id per pointer drag so the core merges a drag's segments into one
  // undo step but keeps separate strokes separate.
  let nextStrokeId = 1;
  let strokeId = 0;

  const onDown = (e: PointerEvent): void => {
    if (e.button !== 0) {
      return;
    }
    drawing = true;
    strokeId = nextStrokeId++;
    last = toCanvasPoint(canvas, e);
    canvas.setPointerCapture(e.pointerId);
    paintStroke([last], strokeId); // initial dab
    redraw();
  };

  const onMove = (e: PointerEvent): void => {
    if (!drawing || !last) {
      return;
    }
    const point = toCanvasPoint(canvas, e);
    paintStroke([last, point], strokeId);
    last = point;
    redraw();
  };

  const onUp = (e: PointerEvent): void => {
    if (!drawing) {
      return;
    }
    drawing = false;
    last = null;
    if (canvas.hasPointerCapture(e.pointerId)) {
      canvas.releasePointerCapture(e.pointerId);
    }
  };

  canvas.addEventListener('pointerdown', onDown);
  canvas.addEventListener('pointermove', onMove);
  canvas.addEventListener('pointerup', onUp);
  canvas.addEventListener('pointercancel', onUp);

  return () => {
    canvas.removeEventListener('pointerdown', onDown);
    canvas.removeEventListener('pointermove', onMove);
    canvas.removeEventListener('pointerup', onUp);
    canvas.removeEventListener('pointercancel', onUp);
  };
}
