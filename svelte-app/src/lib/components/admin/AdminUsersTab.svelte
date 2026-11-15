<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type {
    AdminUpdateUserRoleRequest,
    AdminUserResponse,
    Pagination,
    UserRole
  } from '#lib/api/types.generated.ts';
  import Badge from '#lib/components/ui/Badge.svelte';
  import Button from '#lib/components/ui/Button.svelte';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  interface Props {
    onImpersonate: (userId: number) => void;
  }

  let { onImpersonate }: Props = $props();

  const d = $derived(uiPrefs.dict);

  let users = $state<AdminUserResponse[]>([]);
  let pagination = $state<Pagination>({ page: 1, page_size: 20, total_items: 0, total_pages: 1 });
  let search = $state('');
  let roleFilter = $state<UserRole | 'all'>('all');
  let loading = $state(false);
  let updatingId = $state<number | null>(null);

  async function loadUsers(pageNum = 1) {
    loading = true;
    try {
      const res = await apiRequest<AdminUserResponse[]>(API_ENDPOINTS.ADMIN_LIST_ADMIN_USERS, {
        query: {
          page: pageNum,
          page_size: 20,
          search: search.trim() || undefined,
          role: roleFilter === 'all' ? undefined : roleFilter
        }
      });
      if (res.success && res.data) {
        users = res.data;
        if (res.pagination) pagination = res.pagination;
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void roleFilter;
    void loadUsers(1);
  });

  async function toggleRole(user: AdminUserResponse) {
    const nextRole: UserRole = user.role === 'admin' ? 'user' : 'admin';
    updatingId = user.id;
    try {
      const res = await apiRequest<AdminUserResponse, AdminUpdateUserRoleRequest>(
        API_ENDPOINTS.ADMIN_UPDATE_USER_ROLE,
        {
          params: { id: user.id },
          body: { role: nextRole }
        }
      );
      if (res.success && res.data) {
        users = users.map((u) => (u.id === user.id ? res.data! : u));
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      updatingId = null;
    }
  }
</script>

<div class="space-y-4">
  <div class="flex flex-col gap-2.5 sm:flex-row sm:items-center sm:justify-between">
    <div class="flex flex-1 gap-2">
      <label for="admin-users-search" class="sr-only">{d.common.search}</label>
      <input
        id="admin-users-search"
        data-testid="admin-users-search"
        type="search"
        bind:value={search}
        onkeydown={(e) => {
          if (e.key === 'Enter') void loadUsers(1);
        }}
        placeholder="{d.common.search}..."
        class="h-10 w-full max-w-sm rounded-xl border border-base-line bg-base-sub px-3.5 text-sm text-base-content focus:border-primary focus:outline-none"
      />
      <Button variant="secondary" onclick={() => loadUsers(1)}>{d.common.search}</Button>
    </div>

    <div>
      <label for="admin-users-role" class="sr-only">Role filter</label>
      <select
        id="admin-users-role"
        bind:value={roleFilter}
        class="h-10 cursor-pointer rounded-xl border border-base-line bg-base-sub px-3 text-sm text-base-content focus:border-primary focus:outline-none"
      >
        <option value="all">All roles</option>
        <option value="user">user</option>
        <option value="admin">admin</option>
      </select>
    </div>
  </div>

  <div class="overflow-x-auto rounded-2xl border border-base-line bg-base-sub">
    <table class="w-full text-left text-sm" data-testid="admin-users-table">
      <thead class="border-b border-base-line bg-base-main text-xs text-base-content-secondary uppercase">
        <tr>
          <th class="px-4 py-3 font-semibold">User</th>
          <th class="px-4 py-3 font-semibold">Role</th>
          <th class="px-4 py-3 font-semibold">Workspaces</th>
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
        {:else if users.length === 0}
          <tr>
            <td colspan="4" class="px-4 py-8 text-center text-xs text-base-content-muted">
              {d.common.empty}
            </td>
          </tr>
        {:else}
          {#each users as user (user.id)}
            <tr class="hover:bg-base-main/40" data-testid="admin-user-row-{user.id}">
              <td class="px-4 py-3">
                <div class="font-semibold text-base-content">{user.name}</div>
                <div class="text-xs text-base-content-secondary">{user.email}</div>
              </td>
              <td class="px-4 py-3">
                <div class="flex items-center gap-1.5">
                  <Badge tone={user.role === 'admin' ? 'warning' : 'neutral'}>{user.role}</Badge>
                  {#if user.is_test_account}
                    <Badge tone="info">test</Badge>
                  {/if}
                </div>
              </td>
              <td class="px-4 py-3 text-xs text-base-content-secondary">
                {user.workspace_count}
              </td>
              <td class="px-4 py-3">
                <div class="flex items-center justify-end gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    loading={updatingId === user.id}
                    onclick={() => toggleRole(user)}
                  >
                    {user.role === 'admin' ? '-> user' : '-> admin'}
                  </Button>
                  <Button
                    variant="primary"
                    size="sm"
                    data-testid="impersonate-user-{user.id}"
                    onclick={() => onImpersonate(user.id)}
                  >
                    {d.admin.impersonate}
                  </Button>
                </div>
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>

  {#if pagination.total_pages > 1}
    <div class="flex items-center justify-end gap-2 text-xs text-base-content-secondary">
      <Button
        variant="outline"
        size="sm"
        disabled={pagination.page <= 1}
        onclick={() => loadUsers(pagination.page - 1)}
      >
        Prev
      </Button>
      <span>{pagination.page} / {pagination.total_pages}</span>
      <Button
        variant="outline"
        size="sm"
        disabled={pagination.page >= pagination.total_pages}
        onclick={() => loadUsers(pagination.page + 1)}
      >
        Next
      </Button>
    </div>
  {/if}
</div>
