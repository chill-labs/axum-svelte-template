<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type { HotSupportLoginRequest, HotSupportSessionInfo } from '#lib/api/types.generated.ts';
  import Button from '#lib/components/ui/Button.svelte';
  import Card from '#lib/components/ui/Card.svelte';
  import EmptyState from '#lib/components/ui/EmptyState.svelte';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  interface Props {
    refreshSignal?: number;
  }

  let { refreshSignal = 0 }: Props = $props();

  const d = $derived(uiPrefs.dict);

  let sessionInfo = $state<HotSupportSessionInfo>({ active: false });
  let userIdInput = $state('');
  let loading = $state(false);
  let acting = $state(false);

  async function loadSession() {
    loading = true;
    try {
      const res = await apiRequest<HotSupportSessionInfo>(
        API_ENDPOINTS.ADMIN_HOT_SUPPORT_SESSION
      );
      if (res.success && res.data) {
        sessionInfo = res.data;
      }
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void refreshSignal;
    void loadSession();
  });

  $effect(() => {
    function handleFrameMessage(event: MessageEvent) {
      if (event.data?.type === 'hot-support-closed') {
        sessionInfo = { active: false };
      }
    }
    window.addEventListener('message', handleFrameMessage);
    return () => window.removeEventListener('message', handleFrameMessage);
  });

  async function startById(event: SubmitEvent) {
    event.preventDefault();
    const userId = Number(userIdInput);
    if (!userId) return;

    acting = true;
    try {
      const res = await apiRequest<HotSupportSessionInfo, HotSupportLoginRequest>(
        API_ENDPOINTS.ADMIN_HOT_SUPPORT_LOGIN,
        { body: { user_id: userId } }
      );
      if (res.success && res.data) {
        sessionInfo = res.data;
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      acting = false;
    }
  }

  async function closeHotSupport() {
    acting = true;
    try {
      const res = await apiRequest<boolean>(API_ENDPOINTS.ADMIN_HOT_SUPPORT_CLOSE);
      if (res.success) {
        sessionInfo = { active: false };
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      acting = false;
    }
  }
</script>

<div class="space-y-4">
  <Card>
    <div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
      <form onsubmit={startById} class="flex flex-wrap items-center gap-2">
        <label for="hs-user-id" class="text-xs font-semibold text-base-content-secondary">
          User ID
        </label>
        <input
          id="hs-user-id"
          type="number"
          min="1"
          bind:value={userIdInput}
          placeholder="e.g. 2"
          class="h-9 w-28 rounded-xl border border-base-line bg-base-main px-3 text-sm text-base-content focus:border-primary focus:outline-none"
        />
        <Button type="submit" size="sm" loading={acting}>{d.admin.impersonate}</Button>
      </form>

      {#if sessionInfo.active}
        <div class="flex flex-wrap items-center gap-3">
          <div class="text-xs text-base-content-secondary">
            <strong class="text-base-content">{sessionInfo.target_user_name}</strong>
            ({sessionInfo.target_user_email})
          </div>
          <Button
            variant="danger"
            size="sm"
            loading={acting}
            data-testid="admin-close-hot-support"
            onclick={closeHotSupport}
          >
            {d.admin.closeHotSupport}
          </Button>
        </div>
      {/if}
    </div>
  </Card>

  {#if loading}
    <Card>
      <p class="py-8 text-center text-xs text-base-content-muted">{d.common.loading}</p>
    </Card>
  {:else if sessionInfo.active}
    <iframe
      data-testid="hot-support-iframe"
      src={sessionInfo.launch_url || '/todos?_hs=1'}
      title="Hot Support Workspace Preview"
      class="h-[640px] w-full rounded-2xl border-2 border-amber-500/50 bg-base-main shadow-lg"
    ></iframe>
  {:else}
    <EmptyState title={d.admin.tabHotSupport} description={d.common.empty} />
  {/if}
</div>
