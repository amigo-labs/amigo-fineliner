<script lang="ts">
  import { onMount } from 'svelte';
  import { editor, tool } from '../../stores/editor.svelte';
  import { readComposite, renderText } from '../../core/controller';
  import { drawComposite } from '../../render/canvas2d';
  import { attachTools } from '../../tools/pointer';
  import CanvasOverlay from './CanvasOverlay.svelte';

  let canvas: HTMLCanvasElement;

  // In-progress text entry (Text tool). `cx`/`cy` are canvas-space; `left`/`top`
  // position the floating textarea over the click in display pixels (spec §9.2).
  let textEntry = $state<{ cx: number; cy: number; left: number; top: number; text: string } | null>(
    null,
  );

  function redraw(): void {
    if (!canvas || editor.handle === null) {
      return;
    }
    // An open effect dialog overrides the canvas with its live preview.
    const rgba = editor.previewComposite ?? readComposite();
    if (rgba) {
      drawComposite(canvas, editor.width, editor.height, rgba);
    }
  }

  /** Display pixels per canvas pixel (the canvas is shown scaled to fit). */
  function displayScale(): number {
    return canvas && editor.width > 0 ? canvas.clientWidth / editor.width : 1;
  }

  /** Opens a text-entry box at a click, committing any prior entry first. */
  function placeText(cx: number, cy: number, e: PointerEvent): void {
    commitText();
    const rect = canvas.getBoundingClientRect();
    textEntry = { cx, cy, left: e.clientX - rect.left, top: e.clientY - rect.top, text: '' };
  }

  /** Rasterizes the current entry to pixels (no-op if empty), then clears it. */
  function commitText(): void {
    const entry = textEntry;
    if (!entry) {
      return;
    }
    textEntry = null; // clear first so a trailing blur does not double-commit
    if (entry.text.trim().length > 0) {
      // Swallow rejections (font load / WASM) so the event handler stays quiet.
      void renderText(entry.cx, entry.cy, entry.text)
        .then(redraw)
        .catch(() => {});
    }
  }

  function onEntryKeydown(e: KeyboardEvent): void {
    // Esc or Ctrl/Cmd+Enter commit; plain Enter inserts a newline (spec §9.2).
    if (e.key === 'Escape' || (e.key === 'Enter' && (e.ctrlKey || e.metaKey))) {
      e.preventDefault();
      e.stopPropagation();
      commitText();
    }
  }

  /** Focuses the textarea when it mounts. */
  function autofocus(node: HTMLTextAreaElement): void {
    node.focus();
  }

  // The core anchors center/right-aligned text about `x` (tools/text.rs), so
  // the preview box shifts by the same fraction of its own width.
  const entryShift = $derived(
    tool.textAlign === 'center' ? '-50%' : tool.textAlign === 'right' ? '-100%' : '0',
  );

  // Per-tool cursor so the canvas signals what a click will do (spec §9.1).
  const toolCursors: Record<string, string> = {
    pencil: 'crosshair',
    eraser: 'crosshair',
    fill: 'cell',
    eyedropper: 'copy',
    move: 'move',
    rect_select: 'crosshair',
    ellipse_select: 'crosshair',
    lasso: 'crosshair',
    polygon_lasso: 'crosshair',
    magic_wand: 'cell',
    shapes: 'crosshair',
    text: 'text',
  };
  const cursor = $derived(toolCursors[tool.kind] ?? 'default');

  onMount(() => attachTools(canvas, redraw, placeText));

  // Recompose whenever the document mutates.
  $effect(() => {
    void editor.revision;
    redraw();
  });
</script>

<div class="flex h-full w-full items-center justify-center overflow-auto p-8">
  {#if editor.handle === null}
    <p class="text-sm text-neutral-500">Open an image or create a new document to start.</p>
  {/if}
  <!-- The overlay is layered exactly over the main canvas; the wrapper shrinks
       to the displayed canvas so `absolute inset-0` aligns the two. -->
  <div class="relative" style={editor.handle === null ? 'display:none' : ''}>
    <canvas
      bind:this={canvas}
      class="block max-h-full max-w-full touch-none shadow-2xl shadow-black/60"
      style="image-rendering: pixelated; cursor: {cursor};"
    ></canvas>
    <CanvasOverlay />
    {#if textEntry}
      <textarea
        use:autofocus
        bind:value={textEntry.text}
        onkeydown={onEntryKeydown}
        onblur={commitText}
        spellcheck="false"
        class="absolute z-10 resize-none overflow-hidden whitespace-pre rounded border border-dashed border-[var(--fl-accent)] bg-transparent p-0 leading-none outline-none"
        style="left: {textEntry.left}px; top: {textEntry.top}px; transform: translateX({entryShift}); text-align: {tool.textAlign}; font-size: {tool.fontSize *
          displayScale()}px; color: {tool.foreground}; font-family: 'Liberation Sans', Arial, sans-serif; font-weight: {tool.textBold
          ? 'bold'
          : 'normal'}; font-style: {tool.textItalic ? 'italic' : 'normal'}; min-width: 4ch;"
      ></textarea>
    {/if}
  </div>
</div>
