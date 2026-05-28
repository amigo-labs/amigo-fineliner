// The single entry point to the WASM core. No other module imports the
// generated bindings directly (CLAUDE.md §5.4). Document state lives in Rust;
// JS only receives composited pixels and exported bytes (ADR-001).
import initWasm, {
  create_document,
  open_image,
  close_document,
  composite,
  apply_command,
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

export interface PencilStrokeCommand {
  type: 'pencil_stroke';
  layer: number;
  size: number;
  color: [number, number, number, number];
  opacity: number;
  points: Array<[number, number]>;
}

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
  applyCommand: (handle: number, command: PencilStrokeCommand): void =>
    apply_command(handle, JSON.stringify(command)),
  undo: (handle: number): boolean => undo(handle),
  redo: (handle: number): boolean => redo(handle),
  exportPng: (handle: number, compression: number): Uint8Array => export_png(handle, compression),
  exportJpeg: (handle: number, quality: number): Uint8Array => export_jpeg(handle, quality),
  exportWebp: (handle: number): Uint8Array => export_webp(handle),
  documentInfo: (handle: number): DocumentInfo => get_document_info(handle) as DocumentInfo,
};
