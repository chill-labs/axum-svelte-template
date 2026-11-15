<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type { HotSupportLoginRequest, HotSupportSessionInfo } from '#lib/api/types.generated.ts';
  import AdminHotSupportTab from '#lib/components/admin/AdminHotSupportTab.svelte';
  import AdminNotificationsTab from '#lib/components/admin/AdminNotificationsTab.svelte';
  import AdminTestAccountsTab from '#lib/components/admin/AdminTestAccountsTab.svelte';
  import AdminUsersTab from '#lib/components/admin/AdminUsersTab.svelte';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  const d = $derived(uiPrefs.dict);

  type AdminTab = 'users' | 'hot_support' | 'test_accounts' | 'notifications';

  let activeTab = $state<AdminTab>('users');
  let hotSupportRefresh = $state(0);

  async function handleImpersonate(userId: number) {
    const res = await apiRequest<HotSupportSessionInfo, HotSupportLoginRequest>(
      API_ENDPOINTS.ADMIN_HOT_SUPPORT_LOGIN,
      { body: { user_id: userId } }
    );
    if (res.success) {
      hotSupportRefresh += 1;
      activeTab = 'hot_support';
    } else {
      toasts.error(uiPrefs.err(res.message));
    }
  }

  const tabs = $derived([
    { id: 'users' as AdminTab, label: d.admin.tabUsers },
    { id: 'hot_support' as AdminTab, label: d.admin.tabHotSupport },
    { id: 'test_accounts' as AdminTab, label: d.admin.tabTestAccounts },
    { id: 'notifications' as AdminTab, label: d.admin.tabNotifications }
  ]);
</script>

<div class="space-y-5">
  <div>
    <h1 class="text-2xl font-bold tracking-tight text-base-content">{d.admin.pageTitle}</h1>
    <p class="mt-0.5 text-sm text-base-content-secondary">{d.admin.pageSubtitle}</p>
  </div>

  <div class="flex flex-wrap gap-2 border-b border-base-line pb-3">
    {#each tabs as tab (tab.id)}
      <button
        type="button"
        data-testid="admin-tab-{tab.id}"
        onclick={() => (activeTab = tab.id)}
        class="cursor-pointer rounded-xl px-3.5 py-2 text-xs font-semibold transition {activeTab ===
        tab.id
          ? 'cta-gradient-primary text-white'
          : 'border border-base-line bg-base-sub text-base-content-secondary hover:bg-base-elevated'}"
      >
        {tab.label}
      </button>
    {/each}
  </div>

  {#if activeTab === 'users'}
    <AdminUsersTab onImpersonate={handleImpersonate} />
  {:else if activeTab === 'hot_support'}
    <AdminHotSupportTab refreshSignal={hotSupportRefresh} />
  {:else if activeTab === 'test_accounts'}
    <AdminTestAccountsTab />
  {:else if activeTab === 'notifications'}
    <AdminNotificationsTab />
  {/if}
</div>
