<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import type { Snippet } from 'svelte';
  import AppLogo from '#lib/components/ui/AppLogo.svelte';
  import { matchRouteRule, ROUTES } from '#lib/config/routes.ts';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';

  let { children }: { children: Snippet } = $props();

  const d = $derived(uiPrefs.dict);

  $effect(() => {
    const rule = matchRouteRule(page.url.pathname);
    if (rule?.type === 'guest_only' && sessionStore.isAuthenticated) {
      const returnTo = localStorage.getItem('return_to');
      localStorage.removeItem('return_to');
      const next =
        returnTo && returnTo.startsWith('/') && !returnTo.startsWith('/auth/')
          ? returnTo
          : ROUTES.TODOS;
      void goto(next, { replaceState: true });
    }
  });
</script>

<div class="flex min-h-screen flex-col justify-between bg-base-main px-4 py-8">
  <div class="mx-auto flex w-full max-w-md items-center justify-between">
    <a href={ROUTES.TODOS} class="flex items-center gap-2.5">
      <AppLogo sizeClass="h-9 w-9" />
      <span class="text-sm font-bold text-base-content">{d.common.appName}</span>
    </a>

    <div class="flex items-center gap-2">
      <button
        type="button"
        onclick={() => uiPrefs.toggleLocale()}
        class="h-8 cursor-pointer rounded-xl border border-base-line bg-base-sub px-2.5 text-xs font-bold text-base-content-secondary hover:bg-base-elevated"
      >
        {uiPrefs.locale.toUpperCase()}
      </button>
      <button
        type="button"
        onclick={() => uiPrefs.toggleTheme()}
        aria-label={uiPrefs.theme === 'dark' ? d.common.lightMode : d.common.darkMode}
        class="flex h-8 w-8 cursor-pointer items-center justify-center rounded-xl border border-base-line bg-base-sub text-xs text-base-content-secondary hover:bg-base-elevated"
      >
        {uiPrefs.theme === 'dark' ? '☀' : '☾'}
      </button>
    </div>
  </div>

  <main class="mx-auto my-8 w-full max-w-md">
    <div class="glass-card rounded-3xl p-6 sm:p-8">
      {@render children()}
    </div>
  </main>

  <footer class="mx-auto flex w-full max-w-md items-center justify-center gap-4 text-xs text-base-content-muted">
    <a href={ROUTES.TERMS} class="hover:text-base-content">Terms</a>
    <span>•</span>
    <a href={ROUTES.PRIVACY} class="hover:text-base-content">Privacy</a>
    <span>•</span>
    <a href={ROUTES.HEALTHCHECK} class="hover:text-base-content">Status</a>
  </footer>
</div>
