<script lang="ts">
  import { onMount } from 'svelte';
  import { editor } from '../../stores/editor.svelte';
  import { readComposite } from '../../core/controller';
  import { drawComposite } from '../../render/canvas2d';
  import { attachPencil } from '../../tools/pointer';

  let canvas: HTMLCanvasElement;

  function redraw(): void {
    if (!canvas || editor.handle === null) {
      return;
    }
    const rgba = readComposite();
    if (rgba) {
      drawComposite(canvas, editor.width, editor.height, rgba);
    }
  }

  onMount(() => attachPencil(canvas, redraw));

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
  <canvas
    bind:this={canvas}
    class="max-h-full max-w-full touch-none shadow-2xl shadow-black/60"
    style="image-rendering: pixelated; {editor.handle === null ? 'display:none' : ''}"
  ></canvas>
</div>
