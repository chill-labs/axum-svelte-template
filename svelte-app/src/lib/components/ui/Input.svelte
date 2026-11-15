<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props extends Omit<HTMLInputAttributes, 'value'> {
    label?: string;
    error?: string;
    value?: string;
  }

  let {
    label,
    error,
    value = $bindable(''),
    id,
    class: className = '',
    ...rest
  }: Props = $props();

  const generatedId = `input-${Math.random().toString(36).slice(2, 8)}`;
  const inputId = $derived(id ?? generatedId);
</script>

<div class="space-y-1.5">
  {#if label}
    <label for={inputId} class="block text-xs font-semibold tracking-wide text-base-content-secondary">
      {label}
    </label>
  {/if}
  <input
    id={inputId}
    bind:value
    class="w-full rounded-xl border bg-base-sub px-3.5 py-2.5 text-sm text-base-content placeholder:text-base-content-muted focus:outline-none focus:ring-2 {error
      ? 'border-error focus:ring-error/30'
      : 'border-base-line focus:border-primary focus:ring-primary/25'} {className}"
    {...rest}
  />
  {#if error}
    <p class="text-xs font-medium text-error">{error}</p>
  {/if}
</div>
