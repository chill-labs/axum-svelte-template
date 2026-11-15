<script lang="ts">
  import { toasts } from '#lib/state/toast.svelte.ts';

  const toneStyles: Record<string, string> = {
    success: 'border-emerald-500/35 bg-emerald-500/12 text-emerald-800 dark:text-emerald-200',
    error: 'border-rose-500/35 bg-rose-500/12 text-rose-800 dark:text-rose-200',
    warning: 'border-amber-500/35 bg-amber-500/12 text-amber-800 dark:text-amber-200',
    info: 'border-primary/35 bg-primary/12 text-base-content'
  };
</script>

<div
  class="pointer-events-none fixed right-4 bottom-4 z-[9999] flex w-full max-w-sm flex-col gap-2"
  aria-live="polite"
>
  {#each toasts.items as toast (toast.id)}
    <div
      class="pointer-events-auto flex items-start justify-between gap-3 rounded-2xl border px-4 py-3 text-sm font-medium shadow-lg backdrop-blur-md {toneStyles[
        toast.kind
      ] ?? toneStyles.info}"
    >
      <span>{toast.message}</span>
      <button
        type="button"
        onclick={() => toasts.dismiss(toast.id)}
        aria-label="Dismiss notification"
        class="cursor-pointer opacity-60 transition hover:opacity-100"
      >
        ×
      </button>
    </div>
  {/each}
</div>
