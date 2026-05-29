<script lang="ts">
  import { onMount } from 'svelte';
  import { editor, resetColors, swapColors } from './lib/stores/editor.svelte';
  import {
    newDocument,
    openFile,
    exportPng,
    undo,
    redo,
  } from './lib/core/controller';
  import MainCanvas from './lib/components/canvas/MainCanvas.svelte';
  import ToolBar from './lib/components/toolbar/ToolBar.svelte';
  import ToolOptions from './lib/components/toolbar/ToolOptions.svelte';
  import ColorsPanel from './lib/components/panels/ColorsPanel.svelte';

  let fileInput: HTMLInputElement;
  let loadError = $state<string | null>(null);

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
    if (e.target instanceof HTMLInputElement) {
      return;
    }
    const ctrl = e.ctrlKey || e.metaKey;
    if (ctrl && e.key.toLowerCase() === 'z' && !e.shiftKey) {
      e.preventDefault();
      undo();
    } else if (ctrl && (e.key.toLowerCase() === 'y' || (e.key.toLowerCase() === 'z' && e.shiftKey))) {
      e.preventDefault();
      redo();
    } else if (!ctrl && e.key.toLowerCase() === 'x') {
      swapColors();
    } else if (!ctrl && e.key.toLowerCase() === 'd') {
      resetColors();
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
    <button class="rounded px-2 py-1 hover:bg-neutral-700" onclick={exportPng}>Export PNG</button>
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
        <span>Pencil</span>
        <span>{editor.width} × {editor.height} · sRGB</span>
        <span>{loadError ?? 'Ready'}</span>
      </footer>
    </div>

    <aside class="w-56 border-l border-[var(--fl-panel-border)] bg-[var(--fl-app-bg)]">
      <ColorsPanel />
    </aside>
  </div>
</div>
