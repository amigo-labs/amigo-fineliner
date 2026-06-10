<script lang="ts">
  import { onMount } from 'svelte';
  import { editor, tool, resetColors, swapColors, type ToolKind } from './lib/stores/editor.svelte';
  import {
    newDocument,
    openFile,
    exportImage,
    undo,
    redo,
    selectAll,
    deselect,
    invertSelection,
    type ExportFormat,
  } from './lib/core/controller';
  import MainCanvas from './lib/components/canvas/MainCanvas.svelte';
  import ToolBar from './lib/components/toolbar/ToolBar.svelte';
  import ToolOptions from './lib/components/toolbar/ToolOptions.svelte';
  import ColorsPanel from './lib/components/panels/ColorsPanel.svelte';
  import LayersPanel from './lib/components/panels/LayersPanel.svelte';
  import TransformMenu from './lib/components/menus/TransformMenu.svelte';

  let fileInput: HTMLInputElement;
  let loadError = $state<string | null>(null);
  let exportOpen = $state(false);

  // Export formats (spec §13.2; WebP is lossless per ADR-007).
  const exportFormats: Array<{ format: ExportFormat; label: string }> = [
    { format: 'png', label: 'PNG' },
    { format: 'jpeg', label: 'JPEG' },
    { format: 'webp', label: 'WebP (lossless)' },
  ];

  function runExport(format: ExportFormat): void {
    exportOpen = false;
    exportImage(format);
  }

  // Single-key tool shortcuts (spec §9.2, §16.2). M and L cycle their pair.
  const toolShortcuts: Record<string, ToolKind> = {
    b: 'pencil',
    e: 'eraser',
    g: 'fill',
    i: 'eyedropper',
    v: 'move',
    w: 'magic_wand',
    u: 'shapes',
    t: 'text',
  };
  const toolLabels: Record<ToolKind, string> = {
    pencil: 'Pencil',
    eraser: 'Eraser',
    fill: 'Fill',
    eyedropper: 'Eyedropper',
    move: 'Move',
    rect_select: 'Rectangle Select',
    ellipse_select: 'Ellipse Select',
    lasso: 'Lasso',
    polygon_lasso: 'Polygonal Lasso',
    magic_wand: 'Magic Wand',
    shapes: 'Shapes',
    text: 'Text',
  };

  onMount(() => {
    // Start with a blank white-ish canvas so the demo is immediately usable.
    newDocument(800, 600).catch((e) => (loadError = String(e)));
  });

  async function onFileChosen(e: Event): Promise<void> {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) {
      return;
    }
    loadError = null;
    try {
      await openFile(file);
    } catch (err) {
      loadError = `Could not open image: ${String(err)}`;
    }
    input.value = '';
  }

  function onKeydown(e: KeyboardEvent): void {
    // Ignore shortcuts while typing in a field (text-entry overlay included).
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
      return;
    }
    const ctrl = e.ctrlKey || e.metaKey;
    const key = e.key.toLowerCase();
    if (ctrl && key === 'z' && !e.shiftKey) {
      e.preventDefault();
      undo();
    } else if (ctrl && (key === 'y' || (key === 'z' && e.shiftKey))) {
      e.preventDefault();
      redo();
    } else if (ctrl && key === 'a') {
      e.preventDefault();
      selectAll();
    } else if (ctrl && key === 'd') {
      e.preventDefault();
      deselect();
    } else if (ctrl && e.shiftKey && key === 'i') {
      e.preventDefault();
      invertSelection();
    } else if (!ctrl && key === 'x') {
      swapColors();
    } else if (!ctrl && key === 'd') {
      resetColors();
    } else if (!ctrl && key === 'm') {
      tool.kind = tool.kind === 'rect_select' ? 'ellipse_select' : 'rect_select';
    } else if (!ctrl && key === 'l') {
      tool.kind = tool.kind === 'lasso' ? 'polygon_lasso' : 'lasso';
    } else if (!ctrl && toolShortcuts[key]) {
      tool.kind = toolShortcuts[key];
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="flex h-screen flex-col bg-[var(--fl-app-bg)] text-neutral-200">
  <!-- Top action bar -->
  <header
    class="flex items-center gap-2 border-b border-[var(--fl-panel-border)] bg-[var(--fl-panel-bg)] px-3 py-1.5 text-sm"
  >
    <span class="mr-3 font-semibold text-[var(--fl-accent)]">Fineliner</span>
    <button class="rounded px-2 py-1 hover:bg-neutral-700" onclick={() => newDocument(800, 600)}>
      New
    </button>
    <button class="rounded px-2 py-1 hover:bg-neutral-700" onclick={() => fileInput.click()}>
      Open…
    </button>
    <div class="relative">
      <button
        class="rounded px-2 py-1 hover:bg-neutral-700"
        class:bg-neutral-700={exportOpen}
        onclick={() => (exportOpen = !exportOpen)}
      >
        Export…
      </button>
      {#if exportOpen}
        <!-- Backdrop closes the menu on an outside click. -->
        <button
          class="fixed inset-0 z-40 cursor-default"
          aria-label="Close menu"
          onclick={() => (exportOpen = false)}
        ></button>
        <div
          class="absolute left-0 top-8 z-50 w-40 rounded border border-[var(--fl-panel-border)] bg-[var(--fl-panel-bg)] py-1 text-sm shadow-xl"
        >
          {#each exportFormats as f (f.format)}
            <button
              class="block w-full px-3 py-1 text-left hover:bg-neutral-700"
              onclick={() => runExport(f.format)}
            >
              {f.label}
            </button>
          {/each}
        </div>
      {/if}
    </div>
    <div class="mx-2 h-5 w-px bg-[var(--fl-panel-border)]"></div>
    <button
      class="rounded px-2 py-1 hover:bg-neutral-700 disabled:opacity-40"
      onclick={undo}
      disabled={!editor.canUndo}
    >
      Undo
    </button>
    <button
      class="rounded px-2 py-1 hover:bg-neutral-700 disabled:opacity-40"
      onclick={redo}
      disabled={!editor.canRedo}
    >
      Redo
    </button>
    <div class="mx-2 h-5 w-px bg-[var(--fl-panel-border)]"></div>
    <TransformMenu />
    <input
      bind:this={fileInput}
      type="file"
      accept="image/png,image/jpeg,image/webp,image/bmp,image/gif,image/tiff"
      class="hidden"
      onchange={onFileChosen}
    />
  </header>

  <div class="flex min-h-0 flex-1">
    <ToolBar />

    <div class="flex min-w-0 flex-1 flex-col">
      <ToolOptions />
      <main class="min-h-0 flex-1">
        <MainCanvas />
      </main>
      <!-- Status bar (spec §16.8) -->
      <footer
        class="flex items-center justify-between border-t border-[var(--fl-panel-border)] bg-[var(--fl-panel-bg)] px-3 py-1 text-xs text-neutral-400"
      >
        <span>{toolLabels[tool.kind]}</span>
        <span>{editor.width} × {editor.height} · sRGB</span>
        <span>{loadError ?? 'Ready'}</span>
      </footer>
    </div>

    <aside class="flex w-56 flex-col border-l border-[var(--fl-panel-border)] bg-[var(--fl-app-bg)]">
      <ColorsPanel />
      <LayersPanel />
    </aside>
  </div>
</div>
