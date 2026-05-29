<script lang="ts">
  // Tool options bar (spec §16.3, §9.2). Controls shown depend on the active
  // tool. Bindings write straight into the tool store.
  import { tool } from '../../stores/editor.svelte';

  // Hardness only matters for soft/flat tips.
  const showHardness = $derived(tool.shape !== 'hard_round');
</script>

<div
  class="flex items-center gap-6 border-b border-[var(--fl-panel-border)] bg-[var(--fl-panel-bg)] px-4 py-2 text-sm"
>
  {#if tool.kind === 'pencil' || tool.kind === 'eraser'}
    <label class="flex items-center gap-2">
      <span class="text-neutral-400">Size</span>
      <input type="range" min="1" max="500" bind:value={tool.size} class="w-32" />
      <span class="w-10 tabular-nums text-neutral-300">{tool.size}</span>
    </label>

    <label class="flex items-center gap-2">
      <span class="text-neutral-400">Opacity</span>
      <input type="range" min="1" max="100" bind:value={tool.opacity} class="w-28" />
      <span class="w-10 tabular-nums text-neutral-300">{tool.opacity}%</span>
    </label>

    <label class="flex items-center gap-2">
      <span class="text-neutral-400">Shape</span>
      <select bind:value={tool.shape} class="rounded bg-neutral-800 px-2 py-1">
        <option value="hard_round">Hard Round</option>
        <option value="soft_round">Soft Round</option>
        <option value="flat">Flat</option>
      </select>
    </label>

    {#if showHardness}
      <label class="flex items-center gap-2">
        <span class="text-neutral-400">Hardness</span>
        <input type="range" min="0" max="100" bind:value={tool.hardness} class="w-28" />
        <span class="w-10 tabular-nums text-neutral-300">{tool.hardness}%</span>
      </label>
    {/if}

    {#if tool.kind === 'eraser'}
      <label class="flex items-center gap-2">
        <span class="text-neutral-400">Mode</span>
        <select bind:value={tool.eraserMode} class="rounded bg-neutral-800 px-2 py-1">
          <option value="to_transparent">To Transparent</option>
          <option value="to_background">To Background</option>
        </select>
      </label>
    {/if}
  {:else if tool.kind === 'fill'}
    <label class="flex items-center gap-2">
      <span class="text-neutral-400">Tolerance</span>
      <input type="range" min="0" max="255" bind:value={tool.tolerance} class="w-32" />
      <span class="w-10 tabular-nums text-neutral-300">{tool.tolerance}</span>
    </label>

    <label class="flex items-center gap-2">
      <span class="text-neutral-400">Opacity</span>
      <input type="range" min="1" max="100" bind:value={tool.opacity} class="w-28" />
      <span class="w-10 tabular-nums text-neutral-300">{tool.opacity}%</span>
    </label>

    <label class="flex items-center gap-2">
      <input type="checkbox" bind:checked={tool.contiguous} />
      <span class="text-neutral-400">Contiguous</span>
    </label>

    <label class="flex items-center gap-2">
      <span class="text-neutral-400">Sample</span>
      <select bind:value={tool.fillSample} class="rounded bg-neutral-800 px-2 py-1">
        <option value="current_layer">Current Layer</option>
        <option value="all_layers">All Layers</option>
      </select>
    </label>
  {:else if tool.kind === 'eyedropper'}
    <label class="flex items-center gap-2">
      <span class="text-neutral-400">Sample</span>
      <select bind:value={tool.eyedropperSample} class="rounded bg-neutral-800 px-2 py-1">
        <option value="current_layer">Current Layer</option>
        <option value="all_layers">Composite</option>
      </select>
    </label>

    <label class="flex items-center gap-2">
      <span class="text-neutral-400">Size</span>
      <select bind:value={tool.sampleSize} class="rounded bg-neutral-800 px-2 py-1">
        <option value={1}>1×1</option>
        <option value={3}>3×3</option>
        <option value={5}>5×5</option>
        <option value={11}>11×11</option>
        <option value={31}>31×31</option>
      </select>
    </label>
  {:else if tool.kind === 'move'}
    <span class="text-neutral-500">Drag to translate the active layer.</span>
  {/if}
</div>
