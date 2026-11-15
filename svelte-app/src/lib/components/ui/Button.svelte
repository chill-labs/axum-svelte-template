<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Variant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'outline';
  type Size = 'sm' | 'md' | 'lg';

  interface Props extends HTMLButtonAttributes {
    variant?: Variant;
    size?: Size;
    loading?: boolean;
    children: Snippet;
  }

  let {
    variant = 'primary',
    size = 'md',
    loading = false,
    disabled = false,
    class: className = '',
    type = 'button',
    children,
    ...rest
  }: Props = $props();

  const variantClasses: Record<Variant, string> = {
    primary: 'cta-gradient-primary text-white',
    secondary: 'bg-base-elevated text-base-content hover:bg-base-elevated-hover border border-base-line',
    ghost: 'bg-transparent text-base-content-secondary hover:bg-ghost-hover hover:text-base-content',
    danger: 'bg-error text-white hover:brightness-110 shadow-sm',
    outline: 'bg-base-sub text-base-content border border-base-line hover:bg-base-elevated'
  };

  const sizeClasses: Record<Size, string> = {
    sm: 'h-8 px-3 text-xs rounded-xl gap-1.5',
    md: 'h-10 px-4 text-sm rounded-xl gap-2',
    lg: 'h-11 px-5 text-sm rounded-2xl gap-2.5'
  };
</script>

<button
  {type}
  disabled={disabled || loading}
  class="inline-flex cursor-pointer items-center justify-center font-medium transition disabled:cursor-not-allowed disabled:opacity-55 {variantClasses[
    variant
  ]} {sizeClasses[size]} {className}"
  {...rest}
>
  {#if loading}
    <span
      class="h-3.5 w-3.5 animate-spin rounded-full border-2 border-current border-t-transparent"
      aria-hidden="true"
    ></span>
  {/if}
  {@render children()}
</button>
