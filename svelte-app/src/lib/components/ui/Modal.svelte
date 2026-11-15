<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    open: boolean;
    title: string;
    maxWidthClass?: string;
    onClose: () => void;
    children: Snippet;
    footer?: Snippet;
  }

  let {
    open,
    title,
    maxWidthClass = 'max-w-lg',
    onClose,
    children,
    footer
  }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();

  $effect(() => {
    if (!dialogEl) return;
    if (open && !dialogEl.open) {
      dialogEl.showModal();
    } else if (!open && dialogEl.open) {
      dialogEl.close();
    }
  });

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === dialogEl) {
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
<dialog
  bind:this={dialogEl}
  closedby="any"
  oncancel={(e) => {
    e.preventDefault();
    onClose();
  }}
  onclick={handleBackdropClick}
  class="m-auto w-[calc(100%-2rem)] {maxWidthClass} rounded-3xl border border-base-line bg-base-sub p-0 text-base-content shadow-2xl backdrop:bg-slate-950/55 backdrop:backdrop-blur-xs"
>
  <div class="flex items-center justify-between border-b border-base-line px-6 py-4">
    <h2 class="text-base font-bold text-base-content">{title}</h2>
    <button
      type="button"
      onclick={onClose}
      aria-label="Close"
      class="flex h-8 w-8 cursor-pointer items-center justify-center rounded-xl text-base-content-secondary transition hover:bg-base-elevated hover:text-base-content"
    >
      <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
        <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  </div>
  <div class="max-h-[75vh] overflow-y-auto px-6 py-5">
    {@render children()}
  </div>
  {#if footer}
    <div class="flex items-center justify-end gap-2 border-t border-base-line bg-base-main/40 px-6 py-3.5">
      {@render footer()}
    </div>
  {/if}
</dialog>
