<script lang="ts">
  // Foreground/background swatches with swap (spec §16.6, §4.2).
  import { tool, swapColors } from '../../stores/editor.svelte';
</script>

<div class="border-b border-[var(--fl-panel-border)] bg-[var(--fl-panel-bg)] p-3">
  <h2 class="mb-2 text-xs font-semibold uppercase tracking-wide text-neutral-500">Colors</h2>
  <div class="flex items-center gap-3">
    <label class="flex flex-col items-center gap-1 text-xs text-neutral-400">
      Foreground
      <input
        type="color"
        bind:value={tool.foreground}
        class="h-10 w-10 cursor-pointer rounded border border-[var(--fl-panel-border)] bg-transparent"
        aria-label="Foreground color"
      />
    </label>
    <button
      class="rounded px-2 py-1 text-lg text-neutral-300 hover:bg-neutral-700"
      title="Swap colors (X)"
      onclick={swapColors}
      aria-label="Swap foreground and background colors"
    >
      ⇄
    </button>
    <label class="flex flex-col items-center gap-1 text-xs text-neutral-400">
      Background
      <input
        type="color"
        bind:value={tool.background}
        class="h-10 w-10 cursor-pointer rounded border border-[var(--fl-panel-border)] bg-transparent"
        aria-label="Background color"
      />
    </label>
  </div>
</div>
