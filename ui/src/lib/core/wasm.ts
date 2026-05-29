// The single entry point to the WASM core. No other module imports the
// generated bindings directly (CLAUDE.md §5.4). Document state lives in Rust;
// JS only receives composited pixels and exported bytes (ADR-001).
import initWasm, {
  create_document,
  open_image,
  close_document,
  composite,
  apply_command,
  pick_color,
  undo,
  redo,
  export_png,
  export_jpeg,
  export_webp,
  get_document_info,
} from '../wasm/pkg/fineliner_wasm.js';

export interface DocumentInfo {
  width: number;
  height: number;
  layer_count: number;
  active_layer: number;
  can_undo: boolean;
  can_redo: boolean;
}

export type Rgba = [number, number, number, number];
/** Brush tip shapes (spec §9.2 Pencil). */
export type BrushShape = 'hard_round' | 'soft_round' | 'flat';
/** Color sampling source shared by Fill and Eyedropper (spec §9.2). */
export type SampleSource = 'current_layer' | 'all_layers';
/** Eraser behavior (spec §9.2 Eraser). */
export type EraserMode = 'to_transparent' | 'to_background';

export interface PencilStrokeCommand {
  type: 'pencil_stroke';
  layer: number;
  size: number;
  color: Rgba;
  opacity: number;
  shape: BrushShape;
  hardness: number;
  points: Array<[number, number]>;
  /** Identifies the pointer drag; segments sharing it merge into one undo step. */
  stroke_id: number;
}

export interface EraserStrokeCommand {
  type: 'eraser_stroke';
  layer: number;
  size: number;
  opacity: number;
  shape: BrushShape;
  hardness: number;
  mode: EraserMode;
  background: Rgba;
  points: Array<[number, number]>;
  stroke_id: number;
}

export interface FillBucketCommand {
  type: 'fill_bucket';
  layer: number;
  color: Rgba;
  opacity: number;
  tolerance: number;
  contiguous: boolean;
  sample: SampleSource;
  x: number;
  y: number;
}

export interface TranslateLayerCommand {
  type: 'translate_layer';
  layer: number;
  dx: number;
  dy: number;
}

/** Any command the tools emit to the core. */
export type ToolCommand =
  | PencilStrokeCommand
  | EraserStrokeCommand
  | FillBucketCommand
  | TranslateLayerCommand;

let initialized: Promise<unknown> | null = null;

/** Initializes the WASM module exactly once. Safe to call repeatedly. */
export async function initCore(): Promise<void> {
  if (!initialized) {
    initialized = initWasm();
  }
  await initialized;
}

export const core = {
  createDocument: (width: number, height: number): number => create_document(width, height),
  openImage: (data: Uint8Array, mime: string): number => open_image(data, mime),
  closeDocument: (handle: number): void => close_document(handle),
  composite: (handle: number): Uint8ClampedArray => composite(handle),
  applyCommand: (handle: number, command: ToolCommand): void =>
    apply_command(handle, JSON.stringify(command)),
  /** Samples a color; returns RGBA bytes, or an empty array if off-canvas. */
  pickColor: (handle: number, x: number, y: number, sample: SampleSource, size: number): Uint8Array =>
    pick_color(handle, x, y, sample, size),
  undo: (handle: number): boolean => undo(handle),
  redo: (handle: number): boolean => redo(handle),
  exportPng: (handle: number, compression: number): Uint8Array => export_png(handle, compression),
  exportJpeg: (handle: number, quality: number): Uint8Array => export_jpeg(handle, quality),
  exportWebp: (handle: number): Uint8Array => export_webp(handle),
  documentInfo: (handle: number): DocumentInfo => get_document_info(handle) as DocumentInfo,
};
