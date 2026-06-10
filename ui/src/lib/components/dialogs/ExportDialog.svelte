<script lang="ts">
  // JPEG export dialog (spec §13.2): lossy quality 1–100. PNG/WebP are
  // lossless and export directly without this dialog.
  interface Props {
    onApply: (quality: number) => void;
    onClose: () => void;
  }
  const { onApply, onClose }: Props = $props();

  let quality = $state(90);
</script>

<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
  role="presentation"
  onclick={onClose}
>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="w-72 rounded-lg border border-[var(--fl-panel-border)] bg-[var(--fl-panel-bg)] p-4 text-sm text-neutral-200"
    role="dialog"
    aria-label="Export JPEG"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <h2 class="mb-3 text-sm font-semibold">Export JPEG</h2>

    <label class="mb-3 flex items-center justify-between gap-2">
      <span class="text-neutral-400">Quality</span>
      <input type="range" min="1" max="100" bind:value={quality} class="flex-1" />
      <span class="w-8 text-right tabular-nums">{quality}</span>
    </label>

    <div class="flex justify-end gap-2">
      <button class="rounded px-3 py-1 hover:bg-neutral-700" onclick={onClose}>Cancel</button>
      <button class="rounded bg-[var(--fl-accent)] px-3 py-1 text-white" onclick={() => onApply(quality)}>
        Export
      </button>
    </div>
  </div>
</div>
