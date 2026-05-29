<script lang="ts">
  // Vertical tool strip (spec §16.2). Phase-1 demo ships the Pencil only;
  // the active tool is highlighted with the accent color.
  const activeTool = 'pencil';
</script>

<div class="flex w-12 flex-col items-center gap-1 border-r border-[var(--fl-panel-border)] bg-[var(--fl-panel-bg)] py-2">
  <button
    class="flex h-9 w-9 items-center justify-center rounded text-lg"
    class:bg-[var(--fl-accent)]={activeTool === 'pencil'}
    class:text-white={activeTool === 'pencil'}
    title="Pencil (B)"
    aria-label="Pencil"
  >
    ✏️
  </button>
</div>
