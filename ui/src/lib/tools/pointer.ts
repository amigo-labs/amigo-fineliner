// Pointer-event handling for the tool suite. Pointer events (not mouse events)
// are required for stylus/touch support (CLAUDE.md §9). The active tool (editor
// store) selects the behavior; all mutation goes through the controller.
import {
  paintStroke,
  eraseStroke,
  fillAt,
  moveLayer,
  sampleColor,
  selectRectangle,
  selectEllipse,
  selectPolygon,
  selectWand,
} from '../core/controller';
import { tool, selectionPreview, type ToolKind } from '../stores/editor.svelte';
import type { SelectionMode } from '../core/wasm';

/** Converts a pointer event into canvas-pixel coordinates. */
function toCanvasPoint(canvas: HTMLCanvasElement, e: PointerEvent): [number, number] {
  const rect = canvas.getBoundingClientRect();
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;
  return [(e.clientX - rect.left) * scaleX, (e.clientY - rect.top) * scaleY];
}

/** Selection combine mode from keyboard modifiers (spec §8.2). */
function selectionModeOf(e: PointerEvent): SelectionMode {
  if (e.shiftKey && e.altKey) return 'intersect';
  if (e.shiftKey) return 'add';
  if (e.altKey) return 'subtract';
  return 'replace';
}

/** Tools that drag out a selection shape (rubber band / freehand). */
const DRAG_SELECT: ReadonlySet<ToolKind> = new Set(['rect_select', 'ellipse_select', 'lasso']);

/**
 * Normalizes two corners into a positive-size rect.
 *
 * Shift is reserved for the selection combine mode (add), so rectangle/ellipse
 * do not also constrain to a square on Shift — that would conflict with the
 * documented modifier mapping (spec §8.2 vs §9.3).
 */
function rectFromCorners(
  a: [number, number],
  b: [number, number],
): { x: number; y: number; w: number; h: number } {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const x = Math.round(Math.min(a[0], a[0] + dx));
  const y = Math.round(Math.min(a[1], a[1] + dy));
  return { x, y, w: Math.round(Math.abs(dx)), h: Math.round(Math.abs(dy)) };
}

/**
 * Attaches the active tool's pointer behavior to a canvas. Returns a teardown.
 *
 * Paint tools draw incrementally over a drag (one undo step per drag). Fill and
 * Eyedropper act on click. Selection tools drag a rubber band (rect/ellipse),
 * trace a freehand path (lasso), click vertices (polygonal lasso), or click a
 * region (magic wand); Shift/Alt set the combine mode. A redraw callback runs
 * after each mutation so the canvas stays live.
 */
export function attachTools(canvas: HTMLCanvasElement, redraw: () => void): () => void {
  let active = false;
  let last: [number, number] | null = null;
  let start: [number, number] | null = null;
  let useBackground = false;
  let selMode: SelectionMode = 'replace';
  // Lasso freehand path accumulated during a drag.
  let lassoPath: Array<[number, number]> = [];
  // Polygonal-lasso vertices placed across separate clicks.
  let polygonPoints: Array<[number, number]> = [];
  let polygonMode: SelectionMode = 'replace';
  // Monotonic id per pointer drag so the core merges a drag's segments into one
  // undo step but keeps separate strokes separate.
  let nextStrokeId = 1;
  let strokeId = 0;

  /** Commits the placed polygon if it has enough vertices, then resets. */
  const commitPolygon = (): void => {
    if (polygonPoints.length >= 3) {
      selectPolygon(polygonPoints, polygonMode);
    }
    polygonPoints = [];
    selectionPreview.value = null;
  };

  const onDown = (e: PointerEvent): void => {
    if (e.button !== 0 && e.button !== 2) {
      return;
    }
    // Abandon any in-progress polygon when switching away from that tool.
    if (tool.kind !== 'polygon_lasso' && polygonPoints.length > 0) {
      polygonPoints = [];
      selectionPreview.value = null;
    }

    const point = toCanvasPoint(canvas, e);

    if (tool.kind === 'magic_wand') {
      selectWand(point[0], point[1], selectionModeOf(e));
      return;
    }

    if (tool.kind === 'polygon_lasso') {
      // Click near the first vertex closes the polygon.
      if (polygonPoints.length >= 3) {
        const first = polygonPoints[0];
        const near = Math.hypot(point[0] - first[0], point[1] - first[1]) < 8;
        if (near) {
          commitPolygon();
          return;
        }
      }
      if (polygonPoints.length === 0) {
        polygonMode = selectionModeOf(e);
      }
      polygonPoints = [...polygonPoints, point];
      selectionPreview.value = { shape: 'polygon', points: [...polygonPoints, point] };
      return;
    }

    if (DRAG_SELECT.has(tool.kind)) {
      active = true;
      start = point;
      last = point;
      selMode = selectionModeOf(e);
      canvas.setPointerCapture(e.pointerId);
      if (tool.kind === 'lasso') {
        lassoPath = [point];
        selectionPreview.value = { shape: 'lasso', points: lassoPath };
      } else {
        const shape = tool.kind === 'ellipse_select' ? 'ellipse' : 'rect';
        selectionPreview.value = { shape, points: [point, point] };
      }
      return;
    }

    active = true;
    useBackground = e.button === 2;
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
        break;
    }
  };

  const onMove = (e: PointerEvent): void => {
    const point = toCanvasPoint(canvas, e);

    // Polygonal lasso tracks a rubber line to the cursor between clicks.
    if (tool.kind === 'polygon_lasso' && polygonPoints.length > 0) {
      selectionPreview.value = { shape: 'polygon', points: [...polygonPoints, point] };
      return;
    }

    if (!active || !last) {
      return;
    }

    if (DRAG_SELECT.has(tool.kind) && start) {
      if (tool.kind === 'lasso') {
        lassoPath = [...lassoPath, point];
        selectionPreview.value = { shape: 'lasso', points: lassoPath };
      } else {
        const shape = tool.kind === 'ellipse_select' ? 'ellipse' : 'rect';
        selectionPreview.value = { shape, points: [start, point] };
      }
      last = point;
      return;
    }

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
      default:
        break;
    }
    last = point;
  };

  const onUp = (e: PointerEvent): void => {
    if (!active) {
      return;
    }
    if (DRAG_SELECT.has(tool.kind) && start && last) {
      if (tool.kind === 'lasso') {
        if (lassoPath.length >= 3) {
          selectPolygon(lassoPath, selMode);
        }
        lassoPath = [];
      } else {
        const r = rectFromCorners(start, last);
        if (r.w > 0 && r.h > 0) {
          if (tool.kind === 'ellipse_select') {
            selectEllipse(r.x, r.y, r.w, r.h, selMode);
          } else {
            selectRectangle(r.x, r.y, r.w, r.h, selMode);
          }
        }
      }
      selectionPreview.value = null;
    } else if (tool.kind === 'move' && start && last) {
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

  // Double-click closes a polygonal-lasso selection.
  const onDblClick = (): void => {
    if (tool.kind === 'polygon_lasso') {
      commitPolygon();
    }
  };

  // Suppress the context menu so right-button paint/sample works.
  const onContextMenu = (e: Event): void => e.preventDefault();

  canvas.addEventListener('pointerdown', onDown);
  canvas.addEventListener('pointermove', onMove);
  canvas.addEventListener('pointerup', onUp);
  canvas.addEventListener('pointercancel', onUp);
  canvas.addEventListener('dblclick', onDblClick);
  canvas.addEventListener('contextmenu', onContextMenu);

  return () => {
    canvas.removeEventListener('pointerdown', onDown);
    canvas.removeEventListener('pointermove', onMove);
    canvas.removeEventListener('pointerup', onUp);
    canvas.removeEventListener('pointercancel', onUp);
    canvas.removeEventListener('dblclick', onDblClick);
    canvas.removeEventListener('contextmenu', onContextMenu);
  };
}
