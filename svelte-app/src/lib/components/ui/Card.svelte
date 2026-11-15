<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    class?: string;
    children: Snippet;
  }

  let { class: className = '', children, ...rest }: Props = $props();
</script>

<div class="rounded-2xl border border-base-line bg-base-sub p-5 shadow-soft {className}" {...rest}>
  {@render children()}
</div>
