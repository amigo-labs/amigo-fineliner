<script lang="ts">
  // Shared effect dialog (spec §11): renders the parameter controls for one
  // effect, shows a debounced live preview on the canvas, and commits on Apply.
  import { onMount, untrack } from 'svelte';
  import type { EffectCommand } from '../../core/wasm';
  import { applyEffect, previewEffect, clearEffectPreview } from '../../core/controller';
  import type { EffectDef } from '../menus/effects';
  import Modal from './Modal.svelte';

  interface Props {
    def: EffectDef;
    onClose: () => void;
  }
  const { def, onClose }: Props = $props();

  // `base` carries the effect type plus any non-UI fields (e.g. radial centre);
  // `params` holds the values the fields edit and is spread over `base`. `def`
  // is fixed for the dialog's lifetime (a new effect opens a fresh dialog), so
  // these initial-value reads are intentional (untrack silences the lint).
  const base = untrack(() => def.make());
  const params = $state<Record<string, number | string>>(
    untrack(() =>
      Object.fromEntries(
        def.fields.map(
          (f): [string, number | string] => [
            f.key,
            (base as unknown as Record<string, number | string>)[f.key],
          ],
        ),
      ),
    ),
  );

  // A generic parameter bag over a discriminated union: the catalog guarantees
  // keys/types match, so the double assertion is safe.
  function command(): EffectCommand {
    return { ...base, ...params } as unknown as EffectCommand;
  }

  let timer: ReturnType<typeof setTimeout> | undefined;

  // Re-preview (debounced) whenever a parameter changes; the first run previews
  // the defaults as the dialog opens.
  $effect(() => {
    const cmd = command();
    clearTimeout(timer);
    timer = setTimeout(() => previewEffect(cmd), 120);
  });

  onMount(() => () => clearTimeout(timer));

  function apply(): void {
    clearTimeout(timer);
    applyEffect(command());
    onClose();
  }

  function close(): void {
    clearTimeout(timer);
    clearEffectPreview();
    onClose();
  }
</script>

<Modal title={def.title} applyLabel="Apply" onApply={apply} onClose={close}>
  {#if def.fields.length === 0}
    <p class="mb-3 text-neutral-400">Applies immediately — preview shown on the canvas.</p>
  {/if}
  {#each def.fields as field (field.key)}
    <label class="mb-2 flex items-center justify-between gap-3">
      <span class="text-neutral-400">{field.label}</span>
      {#if field.kind === 'range'}
        <span class="flex items-center gap-2">
          <input
            type="range"
            min={field.min}
            max={field.max}
            step={field.step}
            value={params[field.key] as number}
            oninput={(e) => (params[field.key] = e.currentTarget.valueAsNumber)}
          />
          <span class="w-12 text-right tabular-nums">{params[field.key]}</span>
        </span>
      {:else}
        <select
          value={params[field.key] as string}
          onchange={(e) => (params[field.key] = e.currentTarget.value)}
          class="rounded border border-[var(--fl-panel-border)] bg-neutral-800 px-2 py-1"
        >
          {#each field.options as opt (opt.value)}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      {/if}
    </label>
  {/each}
</Modal>
