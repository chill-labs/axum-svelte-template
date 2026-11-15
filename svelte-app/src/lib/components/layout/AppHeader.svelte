<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type { WorkspaceResponse } from '#lib/api/types.generated.ts';
  import NotificationDropdown from '#lib/components/layout/NotificationDropdown.svelte';
  import AppLogo from '#lib/components/ui/AppLogo.svelte';
  import ProtectedImage from '#lib/components/ui/ProtectedImage.svelte';
  import { ROUTES } from '#lib/config/routes.ts';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  interface Props {
    onToggleMobileNav: () => void;
    onWorkspaceChanged?: () => void;
  }

  let { onToggleMobileNav, onWorkspaceChanged }: Props = $props();

  const d = $derived(uiPrefs.dict);
  let workspaces = $state<WorkspaceResponse[]>([]);
  let switchingWorkspace = $state(false);
  let closingHotSupport = $state(false);

  $effect(() => {
    if (!sessionStore.isAuthenticated) {
      workspaces = [];
      return;
    }
    // Track activeWorkspaceId & name so switching/renaming keeps selector synced.
    void sessionStore.activeWorkspaceId;
    void sessionStore.activeWorkspaceName;
    void apiRequest<WorkspaceResponse[]>(API_ENDPOINTS.WORKSPACES_LIST_WORKSPACES).then((res) => {
      if (res.success && res.data) {
        workspaces = res.data;
      }
    });
  });

  async function handleWorkspaceSelect(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    const id = Number(select.value);
    if (!id || id === sessionStore.activeWorkspaceId) return;

    switchingWorkspace = true;
    try {
      const ok = await sessionStore.switchWorkspace(id);
      if (ok) {
        onWorkspaceChanged?.();
      }
    } finally {
      switchingWorkspace = false;
    }
  }

  async function handleToggleLocale() {
    uiPrefs.toggleLocale();
    if (sessionStore.isAuthenticated) {
      await apiRequest(API_ENDPOINTS.PROFILE_UPDATE_LANGUAGE, {
        body: { language: uiPrefs.locale }
      });
    }
  }

  async function handleExitHotSupport() {
    closingHotSupport = true;
    try {
      const res = await apiRequest<boolean>(API_ENDPOINTS.ADMIN_HOT_SUPPORT_CLOSE);
      if (res.success) {
        if (window.parent !== window) {
          window.parent.postMessage({ type: 'hot-support-closed' }, '*');
        } else {
          window.location.href = ROUTES.ADMIN;
        }
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      closingHotSupport = false;
    }
  }
</script>

{#if sessionStore.isHotSupport}
  <div
    data-testid="hot-support-banner"
    class="flex flex-wrap items-center justify-between gap-2 bg-amber-500 px-4 py-2 text-xs font-semibold text-slate-950"
  >
    <div class="flex items-center gap-2">
      <span>{d.auth.hotSupportBanner}</span>
      {#if sessionStore.user}
        <span class="rounded-full bg-slate-950/15 px-2.5 py-0.5 font-mono">
          {sessionStore.user.name} ({sessionStore.user.email})
        </span>
      {/if}
    </div>
    <button
      type="button"
      disabled={closingHotSupport}
      onclick={handleExitHotSupport}
      class="cursor-pointer rounded-lg bg-slate-950 px-3 py-1 text-xs font-semibold text-white transition hover:bg-slate-800 disabled:opacity-60"
    >
      {d.auth.exitHotSupport}
    </button>
  </div>
{/if}

<header
  class="sticky top-0 z-30 border-b border-base-line bg-base-sub/85 px-4 py-2.5 backdrop-blur-md sm:px-6"
>
  <div class="mx-auto flex max-w-7xl items-center justify-between gap-3">
    <div class="flex items-center gap-3">
      <button
        type="button"
        onclick={onToggleMobileNav}
        aria-label="Toggle navigation menu"
        class="flex h-9 w-9 cursor-pointer items-center justify-center rounded-xl border border-base-line bg-base-sub text-base-content lg:hidden"
      >
        <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8">
          <path stroke-linecap="round" stroke-linejoin="round" d="M4 6h16M4 12h16M4 18h16" />
        </svg>
      </button>

      <a href={ROUTES.TODOS} class="flex items-center gap-2.5">
        <AppLogo sizeClass="h-8 w-8" />
        <span class="hidden text-sm font-bold tracking-tight text-base-content sm:inline">
          {d.common.appName}
        </span>
      </a>
    </div>

    <div class="flex items-center gap-2 sm:gap-2.5">
      {#if sessionStore.isAuthenticated && workspaces.length > 0}
        <div class="flex items-center gap-1.5">
          <label for="header-workspace-switcher" class="sr-only">
            {d.nav.activeWorkspace}
          </label>
          <select
            id="header-workspace-switcher"
            data-testid="workspace-switcher"
            value={sessionStore.activeWorkspaceId ?? ''}
            disabled={switchingWorkspace}
            onchange={handleWorkspaceSelect}
            class="h-9 max-w-[175px] cursor-pointer rounded-xl border border-base-line bg-base-elevated px-2.5 text-xs font-semibold text-base-content focus:border-primary focus:outline-none sm:max-w-[220px]"
          >
            {#each workspaces as ws (ws.id)}
              <option value={ws.id}>{ws.name}</option>
            {/each}
          </select>
        </div>
      {/if}

      <button
        type="button"
        data-open-page-guide
        aria-label={d.common.helpGuide}
        title={d.common.helpGuide}
        class="flex h-9 w-9 cursor-pointer items-center justify-center rounded-xl border border-base-line bg-base-sub text-xs font-bold text-base-content-secondary transition hover:bg-base-elevated hover:text-base-content"
      >
        ?
      </button>

      <button
        type="button"
        data-testid="lang-toggle"
        onclick={handleToggleLocale}
        aria-label={d.common.language}
        class="flex h-9 min-w-10 cursor-pointer items-center justify-center rounded-xl border border-base-line bg-base-sub px-2.5 text-xs font-bold text-base-content-secondary transition hover:bg-base-elevated hover:text-base-content"
      >
        {uiPrefs.locale.toUpperCase()}
      </button>

      <button
        type="button"
        data-testid="theme-toggle"
        onclick={() => uiPrefs.toggleTheme()}
        aria-label={uiPrefs.theme === 'dark' ? d.common.lightMode : d.common.darkMode}
        class="flex h-9 w-9 cursor-pointer items-center justify-center rounded-xl border border-base-line bg-base-sub text-base-content-secondary transition hover:bg-base-elevated hover:text-base-content"
      >
        {#if uiPrefs.theme === 'dark'}
          <svg class="h-4.5 w-4.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z"
            />
          </svg>
        {:else}
          <svg class="h-4.5 w-4.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"
            />
          </svg>
        {/if}
      </button>

      <NotificationDropdown />

      {#if sessionStore.user}
        <a
          href={ROUTES.PROFILE}
          class="hidden items-center gap-2 rounded-xl border border-base-line bg-base-sub px-2.5 py-1 transition hover:bg-base-elevated sm:flex"
        >
          <ProtectedImage
            entityType="user"
            entityId={sessionStore.user.id}
            mediaKey={sessionStore.user.avatar_key}
            src={sessionStore.user.avatar_url}
            alt={sessionStore.user.name}
            fallbackText={sessionStore.user.name}
            class="h-7 w-7 rounded-lg object-cover"
          />
          <span class="max-w-[110px] truncate text-xs font-semibold text-base-content">
            {sessionStore.user.name}
          </span>
        </a>

        <button
          type="button"
          data-testid="logout-button"
          onclick={() => sessionStore.logout()}
          class="cursor-pointer rounded-xl border border-base-line bg-base-sub px-3 py-2 text-xs font-semibold text-base-content-secondary transition hover:bg-rose-500/10 hover:text-rose-600"
        >
          {d.nav.signOut}
        </button>
      {:else}
        <a
          href={ROUTES.LOGIN}
          data-testid="header-signin-link"
          class="cta-gradient-primary inline-flex h-9 items-center rounded-xl px-3.5 text-xs font-semibold text-white transition"
        >
          {d.nav.signIn}
        </a>
      {/if}
    </div>
  </div>
</header>
