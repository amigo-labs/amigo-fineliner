// Reactive editor state (Svelte 5 runes). One store per concern (CLAUDE.md §5.4).

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

/** Pencil tool options (spec §9.2). */
export const tool = $state({
  /** Brush diameter in pixels, 1–500. */
  size: 8,
  /** Stroke opacity, 1–100 (%). */
  opacity: 100,
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
