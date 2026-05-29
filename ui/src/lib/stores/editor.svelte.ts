// Reactive editor state (Svelte 5 runes). One store per concern (CLAUDE.md §5.4).
import type { BrushShape, EraserMode, SampleSource } from '../core/wasm';

/** The open document and its derived state. `handle` is the Rust-side index. */
export const editor = $state({
  handle: null as number | null,
  width: 0,
  height: 0,
  activeLayer: 0,
  canUndo: false,
  canRedo: false,
  /** Bumped after every mutation so the canvas knows to recomposite. */
  revision: 0,
});

/** The selectable tools (spec §9.2 / §16.2). */
export type ToolKind = 'pencil' | 'eraser' | 'fill' | 'eyedropper' | 'move';

/** Tool options across the M6 tool suite (spec §9.2, §16.3). */
export const tool = $state({
  /** The active tool. */
  kind: 'pencil' as ToolKind,
  /** Brush diameter in pixels, 1–500 (Pencil/Eraser). */
  size: 8,
  /** Stroke opacity, 1–100 (%). */
  opacity: 100,
  /** Brush tip shape (Pencil/Eraser). */
  shape: 'hard_round' as BrushShape,
  /** Edge hardness, 0–100 (%); applies to soft/flat tips. */
  hardness: 100,
  /** Eraser behavior. */
  eraserMode: 'to_transparent' as EraserMode,
  /** Fill color-similarity threshold, 0–255. */
  tolerance: 32,
  /** Fill connected region only (vs every matching pixel). */
  contiguous: true,
  /** Fill color sample source. */
  fillSample: 'current_layer' as SampleSource,
  /** Eyedropper sample source ('all_layers' = composite). */
  eyedropperSample: 'all_layers' as SampleSource,
  /** Eyedropper averaging edge length (1/3/5/11/31). */
  sampleSize: 1,
  /** Foreground color as #RRGGBB (spec §4.2 default black). */
  foreground: '#000000',
  /** Background color as #RRGGBB (spec §4.2 default white). */
  background: '#ffffff',
});

/** Swaps foreground and background colors (spec §4.2, `X`). */
export function swapColors(): void {
  const fg = tool.foreground;
  tool.foreground = tool.background;
  tool.background = fg;
}

/** Resets colors to black/white defaults (spec §4.2, `D`). */
export function resetColors(): void {
  tool.foreground = '#000000';
  tool.background = '#ffffff';
}
