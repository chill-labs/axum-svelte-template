<script lang="ts">
  import type { Snippet } from 'svelte';

  type Tone = 'neutral' | 'primary' | 'success' | 'warning' | 'danger' | 'info';

  interface Props {
    tone?: Tone;
    class?: string;
    children: Snippet;
  }

  let { tone = 'neutral', class: className = '', children }: Props = $props();

  const tones: Record<Tone, string> = {
    neutral: 'bg-base-elevated text-base-content-secondary border-base-line',
    primary: 'bg-primary/12 text-primary border-primary/25',
    success: 'bg-emerald-500/12 text-emerald-700 dark:text-emerald-300 border-emerald-500/25',
    warning: 'bg-amber-500/15 text-amber-800 dark:text-amber-300 border-amber-500/25',
    danger: 'bg-rose-500/12 text-rose-700 dark:text-rose-300 border-rose-500/25',
    info: 'bg-sky-500/12 text-sky-700 dark:text-sky-300 border-sky-500/25'
  };
</script>

<span
  class="inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-xs font-semibold {tones[
    tone
  ]} {className}"
>
  {@render children()}
</span>
