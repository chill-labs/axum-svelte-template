<script lang="ts">
  import { browser } from '$app/env';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import type { Snippet } from 'svelte';
  import AppHeader from '#lib/components/layout/AppHeader.svelte';
  import AppSidebar from '#lib/components/layout/AppSidebar.svelte';
  import { matchRouteRule, ROUTES } from '#lib/config/routes.ts';
  import { sessionStore } from '#lib/state/session.svelte.ts';

  let { children }: { children: Snippet } = $props();

  let mobileNavOpen = $state(false);

  $effect(() => {
    if (!browser) return;
    const pathname = page.url.pathname;
    const rule = matchRouteRule(pathname);

    if (rule?.type === 'protected' && !sessionStore.isAuthenticated) {
      localStorage.setItem('return_to', pathname + page.url.search);
      void goto(ROUTES.LOGIN, { replaceState: true });
      return;
    }

    if (rule?.adminOnly && !sessionStore.isAdmin) {
      void goto(ROUTES.TODOS, { replaceState: true });
      return;
    }

    if (
      sessionStore.isAuthenticated &&
      sessionStore.user &&
      !sessionStore.user.onboarding_completed &&
      pathname !== ROUTES.ONBOARDING
    ) {
      void goto(ROUTES.ONBOARDING, { replaceState: true });
    }
  });
</script>

<div class="flex min-h-screen flex-col bg-base-main text-base-content">
  <AppHeader
    onToggleMobileNav={() => (mobileNavOpen = !mobileNavOpen)}
  />

  <div class="mx-auto flex w-full max-w-7xl flex-1">
    <AppSidebar mobileOpen={mobileNavOpen} onCloseMobile={() => (mobileNavOpen = false)} />

    <main class="min-w-0 flex-1 px-4 py-6 sm:px-6 lg:px-8">
      {@render children()}
    </main>
  </div>
</div>
