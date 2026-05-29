<script lang="ts">
  // Pencil options bar (spec §16.3, §9.2): brush size and opacity.
  import { tool } from '../../stores/editor.svelte';
</script>

<div
  class="flex items-center gap-6 border-b border-[var(--fl-panel-border)] bg-[var(--fl-panel-bg)] px-4 py-2 text-sm"
>
  <label class="flex items-center gap-2">
    <span class="text-neutral-400">Size</span>
    <input type="range" min="1" max="500" bind:value={tool.size} class="w-32" />
    <span class="w-10 tabular-nums text-neutral-300">{tool.size}</span>
  </label>

  <label class="flex items-center gap-2">
    <span class="text-neutral-400">Opacity</span>
    <input type="range" min="1" max="100" bind:value={tool.opacity} class="w-32" />
    <span class="w-10 tabular-nums text-neutral-300">{tool.opacity}%</span>
  </label>
</div>
