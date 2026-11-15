<script lang="ts">
  import { goto } from '$app/navigation';
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type {
    GlobalAnnouncementResponse,
    NotificationFeedResponse,
    NotificationResponse
  } from '#lib/api/types.generated.ts';
  import Badge from '#lib/components/ui/Badge.svelte';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { sseClient } from '#lib/state/sse.svelte.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  const d = $derived(uiPrefs.dict);

  let open = $state(false);
  let loading = $state(false);
  let feed = $state<NotificationFeedResponse>({
    items: [],
    global_items: [],
    unread_count: 0
  });
  let wrapperEl: HTMLDivElement | undefined = $state();

  async function loadFeed() {
    loading = true;
    try {
      const res = await apiRequest<NotificationFeedResponse>(
        API_ENDPOINTS.NOTIFICATIONS_GET_NOTIFICATIONS
      );
      if (res.success && res.data) {
        feed = res.data;
        if (sessionStore.data) {
          sessionStore.setSession({
            ...sessionStore.data,
            unread_notifications: res.data.unread_count
          });
        }
      }
    } finally {
      loading = false;
    }
  }

  function toggleDropdown() {
    open = !open;
    if (open) {
      void loadFeed();
    }
  }

  async function markOneRead(item: NotificationResponse) {
    if (!item.is_read) {
      const res = await apiRequest<boolean>(API_ENDPOINTS.NOTIFICATIONS_MARK_ONE_READ, {
        params: { id: item.id }
      });
      if (res.success) {
        feed = {
          ...feed,
          items: feed.items.map((n) => (n.id === item.id ? { ...n, is_read: true } : n)),
          unread_count: Math.max(0, feed.unread_count - 1)
        };
        sessionStore.decrementUnread();
      }
    }
    if (item.link_url) {
      open = false;
      await goto(item.link_url);
    }
  }

  async function markGlobalRead(item: GlobalAnnouncementResponse) {
    if (item.is_read || !sessionStore.isAuthenticated) return;
    const res = await apiRequest<boolean>(API_ENDPOINTS.NOTIFICATIONS_MARK_GLOBAL_READ, {
      params: { id: item.id }
    });
    if (res.success) {
      feed = {
        ...feed,
        global_items: feed.global_items.map((g) =>
          g.id === item.id ? { ...g, is_read: true } : g
        ),
        unread_count: Math.max(0, feed.unread_count - 1)
      };
      sessionStore.decrementUnread();
    }
  }

  async function markAllRead() {
    const res = await apiRequest<boolean>(API_ENDPOINTS.NOTIFICATIONS_MARK_ALL_READ);
    if (res.success) {
      feed = {
        items: feed.items.map((n) => ({ ...n, is_read: true })),
        global_items: feed.global_items.map((g) => ({ ...g, is_read: true })),
        unread_count: 0
      };
      sessionStore.clearUnread();
    } else {
      toasts.error(uiPrefs.err(res.message));
    }
  }

  $effect(() => {
    const unsub = sseClient.onNotification(() => {
      if (open) {
        void loadFeed();
      }
    });
    return unsub;
  });

  $effect(() => {
    if (!open) return;
    function handleOutsideClick(event: MouseEvent) {
      if (wrapperEl && !wrapperEl.contains(event.target as Node)) {
        open = false;
      }
    }
    window.addEventListener('click', handleOutsideClick);
    return () => window.removeEventListener('click', handleOutsideClick);
  });

  const displayCount = $derived(
    sessionStore.isAuthenticated ? sessionStore.unreadCount : feed.global_items.length
  );
</script>

<div bind:this={wrapperEl} class="relative">
  <button
    type="button"
    onclick={toggleDropdown}
    aria-label={d.nav.notifications}
    data-testid="notification-bell"
    class="relative flex h-9 w-9 cursor-pointer items-center justify-center rounded-xl border border-base-line bg-base-sub text-base-content-secondary transition hover:bg-base-elevated hover:text-base-content"
  >
    <svg class="h-4.5 w-4.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8">
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"
      />
    </svg>
    {#if displayCount > 0}
      <span
        data-testid="notification-badge"
        class="absolute -top-1 -right-1 flex h-4.5 min-w-4.5 items-center justify-center rounded-full bg-rose-500 px-1 text-[10px] font-bold text-white"
      >
        {displayCount > 99 ? '99+' : displayCount}
      </span>
    {/if}
  </button>

  {#if open}
    <div
      data-testid="notification-panel"
      class="absolute right-0 z-50 mt-2 w-80 overflow-hidden rounded-2xl border border-base-line bg-base-sub shadow-2xl sm:w-96"
    >
      <div class="flex items-center justify-between border-b border-base-line px-4 py-3">
        <span class="text-sm font-bold text-base-content">{d.nav.notifications}</span>
        {#if sessionStore.isAuthenticated && displayCount > 0}
          <button
            type="button"
            onclick={markAllRead}
            class="cursor-pointer text-xs font-semibold text-primary hover:underline"
          >
            {d.nav.markAllRead}
          </button>
        {/if}
      </div>

      <div class="max-h-96 divide-y divide-base-line overflow-y-auto">
        {#if loading}
          <div class="px-4 py-8 text-center text-xs text-base-content-muted">
            {d.common.loading}
          </div>
        {:else if feed.global_items.length === 0 && feed.items.length === 0}
          <div class="px-4 py-8 text-center text-xs text-base-content-muted">
            {d.nav.noNotifications}
          </div>
        {:else}
          {#each feed.global_items as notice (notice.id)}
            <div class="space-y-1.5 bg-primary/5 px-4 py-3">
              <div class="flex items-center justify-between gap-2">
                <Badge tone={notice.is_read ? 'neutral' : 'primary'}>
                  {notice.title}
                </Badge>
                {#if sessionStore.isAuthenticated && !notice.is_read}
                  <button
                    type="button"
                    onclick={() => markGlobalRead(notice)}
                    class="cursor-pointer text-[11px] font-semibold text-primary hover:underline"
                  >
                    ✓
                  </button>
                {/if}
              </div>
              <div class="prose prose-xs max-w-none text-xs text-base-content-secondary">
                {@html notice.html_content}
              </div>
            </div>
          {/each}

          {#each feed.items as item (item.id)}
            <button
              type="button"
              onclick={() => markOneRead(item)}
              class="block w-full cursor-pointer px-4 py-3 text-left transition hover:bg-base-elevated/60 {item.is_read
                ? 'opacity-70'
                : 'bg-base-sub'}"
            >
              <div class="flex items-start justify-between gap-2">
                <span class="text-xs font-semibold text-base-content">{item.title}</span>
                {#if !item.is_read}
                  <span class="mt-1 h-2 w-2 shrink-0 rounded-full bg-primary"></span>
                {/if}
              </div>
              {#if item.body}
                <p class="mt-0.5 text-xs text-base-content-secondary">{item.body}</p>
              {/if}
            </button>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>
