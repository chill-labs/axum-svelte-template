<script lang="ts">
  import { page } from '$app/state';
  import { ROUTES } from '#lib/config/routes.ts';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';

  interface Props {
    mobileOpen: boolean;
    onCloseMobile: () => void;
  }

  let { mobileOpen, onCloseMobile }: Props = $props();

  const d = $derived(uiPrefs.dict);

  interface NavItem {
    href: string;
    label: string;
    protectedRoute?: boolean;
    adminOnly?: boolean;
  }

  const navItems = $derived<NavItem[]>([
    { href: ROUTES.TODOS, label: d.nav.todos },
    { href: ROUTES.WORKSPACES, label: d.nav.workspaces, protectedRoute: true },
    { href: ROUTES.WORKSPACE_MEMBERS, label: d.nav.members, protectedRoute: true },
    { href: ROUTES.PROFILE, label: d.nav.profile, protectedRoute: true },
    { href: ROUTES.SETTINGS, label: d.nav.settings, protectedRoute: true },
    { href: ROUTES.ADMIN, label: d.nav.admin, protectedRoute: true, adminOnly: true }
  ]);

  const visibleItems = $derived(
    navItems.filter((item) => {
      if (item.adminOnly) return sessionStore.isAdmin;
      if (item.protectedRoute) return sessionStore.isAuthenticated;
      return true;
    })
  );

  function isActive(href: string): boolean {
    const pathname = page.url.pathname;
    if (href === ROUTES.WORKSPACES) {
      return pathname === ROUTES.WORKSPACES;
    }
    return pathname === href || pathname.startsWith(`${href}/`);
  }
</script>

{#if mobileOpen}
  <button
    type="button"
    onclick={onCloseMobile}
    aria-label="Close navigation menu"
    class="fixed inset-0 z-30 bg-slate-950/45 backdrop-blur-xs lg:hidden"
  ></button>
{/if}

<aside
  class="fixed top-14 left-0 z-40 h-[calc(100vh-3.5rem)] w-64 shrink-0 border-r border-base-line bg-base-sub p-4 transition-transform duration-200 lg:sticky lg:top-[3.75rem] lg:z-10 lg:block lg:h-[calc(100vh-3.75rem)] lg:translate-x-0 {mobileOpen
    ? 'translate-x-0'
    : '-translate-x-full'}"
>
  <nav aria-label="Primary navigation" class="space-y-1.5">
    {#each visibleItems as item (item.href)}
      {@const active = isActive(item.href)}
      <a
        href={item.href}
        onclick={onCloseMobile}
        class="flex items-center justify-between rounded-xl px-3.5 py-2.5 text-sm font-medium transition {active
          ? 'bg-primary/12 font-semibold text-primary'
          : 'text-base-content-secondary hover:bg-base-elevated hover:text-base-content'}"
      >
        <span>{item.label}</span>
        {#if active}
          <span class="h-1.5 w-1.5 rounded-full bg-primary"></span>
        {/if}
      </a>
    {/each}
  </nav>

  {#if sessionStore.activeWorkspaceName}
    <div class="mt-6 rounded-2xl border border-base-line bg-base-main/70 p-3.5">
      <p class="text-[11px] font-semibold tracking-wider text-base-content-muted uppercase">
        {d.nav.activeWorkspace}
      </p>
      <p class="mt-1 truncate text-sm font-bold text-base-content">
        {sessionStore.activeWorkspaceName}
      </p>
      {#if sessionStore.activeWorkspaceRole}
        <p class="mt-0.5 text-xs text-base-content-secondary capitalize">
          {sessionStore.activeWorkspaceRole}
        </p>
      {/if}
    </div>
  {/if}
</aside>
