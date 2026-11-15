<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type {
    AdminGlobalNotificationResponse,
    CreateGlobalNotificationRequest,
    GlobalNotificationAudience,
    UpdateGlobalNotificationRequest
  } from '#lib/api/types.generated.ts';
  import Badge from '#lib/components/ui/Badge.svelte';
  import Button from '#lib/components/ui/Button.svelte';
  import Card from '#lib/components/ui/Card.svelte';
  import Input from '#lib/components/ui/Input.svelte';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  const d = $derived(uiPrefs.dict);

  let announcements = $state<AdminGlobalNotificationResponse[]>([]);
  let loading = $state(false);
  let publishing = $state(false);

  let title = $state('');
  let htmlContent = $state('<p>System notice</p>');
  let targetAudience = $state<GlobalNotificationAudience>('all');

  async function loadAnnouncements() {
    loading = true;
    try {
      const res = await apiRequest<AdminGlobalNotificationResponse[]>(
        API_ENDPOINTS.ADMIN_LIST_GLOBAL_NOTIFICATIONS
      );
      if (res.success && res.data) {
        announcements = res.data;
      }
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void loadAnnouncements();
  });

  async function handlePublish(event: SubmitEvent) {
    event.preventDefault();
    publishing = true;
    try {
      const res = await apiRequest<AdminGlobalNotificationResponse, CreateGlobalNotificationRequest>(
        API_ENDPOINTS.ADMIN_CREATE_GLOBAL_NOTIFICATION,
        {
          body: {
            title: title.trim(),
            html_content: htmlContent.trim(),
            target_audience: targetAudience,
            is_active: true
          }
        }
      );
      if (res.success && res.data) {
        announcements = [res.data, ...announcements];
        title = '';
        htmlContent = '<p></p>';
        toasts.success(d.todos.createdSuccess);
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      publishing = false;
    }
  }

  async function toggleActive(item: AdminGlobalNotificationResponse) {
    const res = await apiRequest<AdminGlobalNotificationResponse, UpdateGlobalNotificationRequest>(
      API_ENDPOINTS.ADMIN_UPDATE_GLOBAL_NOTIFICATION,
      {
        params: { id: item.id },
        body: { is_active: !item.is_active }
      }
    );
    if (res.success && res.data) {
      announcements = announcements.map((a) => (a.id === item.id ? res.data! : a));
    } else {
      toasts.error(uiPrefs.err(res.message));
    }
  }

  async function handleDelete(id: number) {
    const res = await apiRequest<boolean>(API_ENDPOINTS.ADMIN_DELETE_GLOBAL_NOTIFICATION, {
      params: { id }
    });
    if (res.success) {
      announcements = announcements.filter((a) => a.id !== id);
    } else {
      toasts.error(uiPrefs.err(res.message));
    }
  }
</script>

<div class="space-y-5">
  <Card>
    <h3 class="mb-3 text-sm font-bold text-base-content">{d.admin.publishAnnouncement}</h3>
    <form onsubmit={handlePublish} class="space-y-3">
      <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <div class="sm:col-span-2">
          <Input
            id="notice-title"
            data-testid="announcement-title-input"
            label="Title"
            bind:value={title}
            required
          />
        </div>
        <div class="space-y-1.5">
          <label
            for="notice-audience"
            class="block text-xs font-semibold tracking-wide text-base-content-secondary"
          >
            {d.admin.targetAudience}
          </label>
          <select
            id="notice-audience"
            data-testid="announcement-audience-select"
            bind:value={targetAudience}
            class="h-10 w-full cursor-pointer rounded-xl border border-base-line bg-base-sub px-3 text-sm text-base-content"
          >
            <option value="all">all</option>
            <option value="authenticated">authenticated</option>
            <option value="admins">admins</option>
          </select>
        </div>
      </div>

      <div class="space-y-1.5">
        <label
          for="notice-html"
          class="block text-xs font-semibold tracking-wide text-base-content-secondary"
        >
          {d.admin.htmlContent}
        </label>
        <textarea
          id="notice-html"
          data-testid="announcement-html-input"
          rows="3"
          bind:value={htmlContent}
          required
          class="w-full rounded-xl border border-base-line bg-base-sub px-3.5 py-2 font-mono text-xs text-base-content focus:border-primary focus:outline-none"
        ></textarea>
      </div>

      <div class="flex justify-end">
        <Button
          type="submit"
          variant="primary"
          loading={publishing}
          data-testid="publish-announcement-submit"
        >
          {d.admin.publishAnnouncement}
        </Button>
      </div>
    </form>
  </Card>

  <div class="space-y-3">
    {#if loading}
      <Card>
        <p class="py-6 text-center text-xs text-base-content-muted">{d.common.loading}</p>
      </Card>
    {:else if announcements.length === 0}
      <Card>
        <p class="py-6 text-center text-xs text-base-content-muted">{d.common.empty}</p>
      </Card>
    {:else}
      {#each announcements as item (item.id)}
        <Card class="space-y-2.5">
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div class="flex items-center gap-2">
              <h4 class="font-semibold text-base-content">{item.title}</h4>
              <Badge tone={item.is_active ? 'success' : 'neutral'}>
                {item.is_active ? d.admin.active : d.admin.inactive}
              </Badge>
              <Badge tone="info">{item.target_audience}</Badge>
            </div>
            <div class="flex items-center gap-2">
              <Button variant="outline" size="sm" onclick={() => toggleActive(item)}>
                {item.is_active ? d.admin.inactive : d.admin.active}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                class="text-rose-600 hover:bg-rose-500/10 hover:text-rose-600"
                onclick={() => handleDelete(item.id)}
              >
                {d.common.delete}
              </Button>
            </div>
          </div>
          <div class="rounded-xl border border-base-line bg-base-main/60 p-3 text-xs text-base-content-secondary">
            {@html item.html_content}
          </div>
        </Card>
      {/each}
    {/if}
  </div>
</div>
