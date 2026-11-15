<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type {
    AdminTestAccountResponse,
    CreateTestAccountRequest,
    UserRole
  } from '#lib/api/types.generated.ts';
  import Badge from '#lib/components/ui/Badge.svelte';
  import Button from '#lib/components/ui/Button.svelte';
  import Card from '#lib/components/ui/Card.svelte';
  import Input from '#lib/components/ui/Input.svelte';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  const d = $derived(uiPrefs.dict);

  let accounts = $state<AdminTestAccountResponse[]>([]);
  let loading = $state(false);
  let creating = $state(false);

  let email = $state('');
  let name = $state('');
  let password = $state('');
  let role = $state<UserRole>('user');
  let language = $state<'vi' | 'en'>('vi');

  async function loadAccounts() {
    loading = true;
    try {
      const res = await apiRequest<AdminTestAccountResponse[]>(
        API_ENDPOINTS.ADMIN_LIST_TEST_ACCOUNTS
      );
      if (res.success && res.data) {
        accounts = res.data;
      }
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void loadAccounts();
  });

  async function handleCreate(event: SubmitEvent) {
    event.preventDefault();
    creating = true;
    try {
      const res = await apiRequest<AdminTestAccountResponse, CreateTestAccountRequest>(
        API_ENDPOINTS.ADMIN_CREATE_TEST_ACCOUNT,
        {
          body: {
            email: email.trim(),
            name: name.trim(),
            password,
            role,
            language
          }
        }
      );
      if (res.success && res.data) {
        accounts = [res.data, ...accounts];
        email = '';
        name = '';
        password = '';
        toasts.success(d.todos.createdSuccess);
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      creating = false;
    }
  }

  async function handleDelete(id: number) {
    const res = await apiRequest<boolean>(API_ENDPOINTS.ADMIN_DELETE_TEST_ACCOUNT, {
      params: { id }
    });
    if (res.success) {
      accounts = accounts.filter((a) => a.id !== id);
      toasts.success(d.todos.deletedSuccess);
    } else {
      toasts.error(uiPrefs.err(res.message));
    }
  }
</script>

<div class="space-y-5">
  <Card>
    <h3 class="mb-3 text-sm font-bold text-base-content">{d.admin.createTestAccount}</h3>
    <form onsubmit={handleCreate} class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-5">
      <Input
        id="test-acc-email"
        data-testid="test-account-email"
        type="email"
        label={d.auth.email}
        bind:value={email}
        required
      />
      <Input
        id="test-acc-name"
        data-testid="test-account-name"
        type="text"
        label={d.auth.displayName}
        bind:value={name}
        required
      />
      <Input
        id="test-acc-password"
        data-testid="test-account-password"
        type="password"
        label={d.auth.password}
        bind:value={password}
        required
      />
      <div class="space-y-1.5">
        <label
          for="test-acc-role"
          class="block text-xs font-semibold tracking-wide text-base-content-secondary"
        >
          Role
        </label>
        <select
          id="test-acc-role"
          bind:value={role}
          class="h-10 w-full cursor-pointer rounded-xl border border-base-line bg-base-sub px-3 text-sm text-base-content"
        >
          <option value="user">user</option>
          <option value="admin">admin</option>
        </select>
      </div>
      <div class="flex items-end gap-2">
        <select
          id="test-acc-lang"
          aria-label={d.common.language}
          bind:value={language}
          class="h-10 cursor-pointer rounded-xl border border-base-line bg-base-sub px-2.5 text-sm text-base-content"
        >
          <option value="vi">vi</option>
          <option value="en">en</option>
        </select>
        <Button
          type="submit"
          variant="primary"
          loading={creating}
          data-testid="create-test-account-submit"
          class="flex-1"
        >
          {d.common.create}
        </Button>
      </div>
    </form>
  </Card>

  <div class="overflow-x-auto rounded-2xl border border-base-line bg-base-sub">
    <table class="w-full text-left text-sm" data-testid="test-accounts-table">
      <thead class="border-b border-base-line bg-base-main text-xs text-base-content-secondary uppercase">
        <tr>
          <th class="px-4 py-3 font-semibold">{d.auth.displayName}</th>
          <th class="px-4 py-3 font-semibold">{d.auth.email}</th>
          <th class="px-4 py-3 font-semibold">Role</th>
          <th class="px-4 py-3 text-right font-semibold">{d.common.actions}</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-base-line">
        {#if loading}
          <tr>
            <td colspan="4" class="px-4 py-8 text-center text-xs text-base-content-muted">
              {d.common.loading}
            </td>
          </tr>
        {:else if accounts.length === 0}
          <tr>
            <td colspan="4" class="px-4 py-8 text-center text-xs text-base-content-muted">
              {d.common.empty}
            </td>
          </tr>
        {:else}
          {#each accounts as acc (acc.id)}
            <tr data-testid="test-account-row-{acc.id}">
              <td class="px-4 py-3 font-medium text-base-content">{acc.name}</td>
              <td class="px-4 py-3 text-xs text-base-content-secondary">{acc.email}</td>
              <td class="px-4 py-3">
                <Badge tone={acc.role === 'admin' ? 'warning' : 'neutral'}>{acc.role}</Badge>
              </td>
              <td class="px-4 py-3 text-right">
                <Button
                  variant="ghost"
                  size="sm"
                  data-testid="delete-test-account-{acc.id}"
                  class="text-rose-600 hover:bg-rose-500/10 hover:text-rose-600"
                  onclick={() => handleDelete(acc.id)}
                >
                  {d.common.delete}
                </Button>
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>
</div>
