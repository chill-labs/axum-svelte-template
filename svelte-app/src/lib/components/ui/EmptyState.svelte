<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    description?: string;
    action?: Snippet;
  }

  let { title, description, action }: Props = $props();
</script>

<div
  class="flex flex-col items-center justify-center rounded-2xl border border-dashed border-base-line bg-base-sub/60 px-6 py-12 text-center"
>
  <div class="mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-base-elevated text-primary">
    <svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8">
      <path stroke-linecap="round" stroke-linejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
    </svg>
  </div>
  <h3 class="text-base font-semibold text-base-content">{title}</h3>
  {#if description}
    <p class="mt-1 max-w-md text-sm text-base-content-secondary">{description}</p>
  {/if}
  {#if action}
    <div class="mt-4">
      {@render action()}
    </div>
  {/if}
</div>
