<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type {
    AddWorkspaceMemberRequest,
    MemberRolePreset,
    WorkspaceMemberResponse
  } from '#lib/api/types.generated.ts';
  import Badge from '#lib/components/ui/Badge.svelte';
  import Button from '#lib/components/ui/Button.svelte';
  import Card from '#lib/components/ui/Card.svelte';
  import EmptyState from '#lib/components/ui/EmptyState.svelte';
  import Input from '#lib/components/ui/Input.svelte';
  import ProtectedImage from '#lib/components/ui/ProtectedImage.svelte';
  import MemberPermissionsModal from '#lib/components/workspaces/MemberPermissionsModal.svelte';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  const d = $derived(uiPrefs.dict);

  let members = $state<WorkspaceMemberResponse[]>([]);
  let loading = $state(false);
  let inviteEmail = $state('');
  let inviteRole = $state<MemberRolePreset>('member');
  let inviting = $state(false);

  let editingMember = $state<WorkspaceMemberResponse | null>(null);
  let modalOpen = $state(false);

  const canView = $derived(sessionStore.can('members', 'view'));
  const canCreate = $derived(sessionStore.can('members', 'create'));
  const canEdit = $derived(sessionStore.can('members', 'edit'));
  const canDelete = $derived(sessionStore.can('members', 'delete'));

  async function loadMembers() {
    const wsId = sessionStore.activeWorkspaceId;
    if (!sessionStore.isAuthenticated || !wsId || !canView) {
      members = [];
      return;
    }

    loading = true;
    try {
      const res = await apiRequest<WorkspaceMemberResponse[]>(
        API_ENDPOINTS.WORKSPACES_LIST_MEMBERS,
        { params: { workspace_id: wsId } }
      );
      if (res.success && res.data) {
        members = res.data;
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void sessionStore.activeWorkspaceId;
    void loadMembers();
  });

  async function handleInvite(event: SubmitEvent) {
    event.preventDefault();
    const wsId = sessionStore.activeWorkspaceId;
    const email = inviteEmail.trim();
    if (!wsId || !email) return;

    inviting = true;
    try {
      const res = await apiRequest<WorkspaceMemberResponse, AddWorkspaceMemberRequest>(
        API_ENDPOINTS.WORKSPACES_ADD_MEMBER,
        {
          params: { workspace_id: wsId },
          body: {
            email,
            role: inviteRole,
            permissions: null
          }
        }
      );
      if (res.success && res.data) {
        members = [...members, res.data];
        inviteEmail = '';
        toasts.success(d.todos.createdSuccess);
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      inviting = false;
    }
  }

  async function handleRemove(member: WorkspaceMemberResponse) {
    const wsId = sessionStore.activeWorkspaceId;
    if (!wsId) return;
    const res = await apiRequest<boolean>(API_ENDPOINTS.WORKSPACES_REMOVE_MEMBER, {
      params: { workspace_id: wsId, member_user_id: member.user_id }
    });
    if (res.success) {
      members = members.filter((m) => m.user_id !== member.user_id);
      toasts.success(d.todos.deletedSuccess);
    } else {
      toasts.error(uiPrefs.err(res.message));
    }
  }
</script>

<div class="space-y-5">
  <div>
    <h1 class="text-2xl font-bold tracking-tight text-base-content">{d.workspaces.membersTitle}</h1>
    <p class="mt-0.5 text-sm text-base-content-secondary">
      {d.workspaces.membersSubtitle}
      {#if sessionStore.activeWorkspaceName}
        — <strong class="text-base-content">{sessionStore.activeWorkspaceName}</strong>
      {/if}
    </p>
  </div>

  {#if canCreate}
    <Card>
      <form onsubmit={handleInvite} class="flex flex-col gap-3 sm:flex-row sm:items-end">
        <div class="flex-1">
          <Input
            id="invite-member-email"
            data-testid="invite-email-input"
            type="email"
            label={d.auth.email}
            placeholder={d.workspaces.invitePlaceholder}
            bind:value={inviteEmail}
            required
          />
        </div>
        <div class="space-y-1.5">
          <label
            for="invite-member-role"
            class="block text-xs font-semibold tracking-wide text-base-content-secondary"
          >
            {d.workspaces.rolePreset}
          </label>
          <select
            id="invite-member-role"
            data-testid="invite-role-select"
            bind:value={inviteRole}
            class="h-10 cursor-pointer rounded-xl border border-base-line bg-base-sub px-3 text-sm text-base-content"
          >
            <option value="editor">editor</option>
            <option value="member">member</option>
            <option value="viewer">viewer</option>
          </select>
        </div>
        <Button
          type="submit"
          variant="primary"
          loading={inviting}
          data-testid="invite-member-submit"
        >
          {d.workspaces.inviteButton}
        </Button>
      </form>
    </Card>
  {/if}

  {#if !canView}
    <EmptyState title={d.errors.PERMISSION_DENIED} />
  {:else if loading && members.length === 0}
    <Card>
      <p class="py-8 text-center text-xs text-base-content-muted">{d.common.loading}</p>
    </Card>
  {:else if members.length === 0}
    <EmptyState title={d.common.empty} />
  {:else}
    <div class="space-y-2.5" data-testid="members-list">
      {#each members as member (member.user_id)}
        <Card class="flex flex-col justify-between gap-3 sm:flex-row sm:items-center">
          <div class="flex items-center gap-3">
            <ProtectedImage
              entityType="user"
              entityId={member.user_id}
              mediaKey={member.user_avatar_key}
              src={member.user_avatar_url}
              alt={member.user_name}
              fallbackText={member.user_name}
              class="h-10 w-10 rounded-xl object-cover"
            />
            <div>
              <div class="flex items-center gap-2">
                <span class="text-sm font-bold text-base-content">{member.user_name}</span>
                <Badge tone={member.is_owner ? 'success' : 'neutral'}>
                  {member.is_owner ? d.workspaces.ownerBadge : member.role}
                </Badge>
              </div>
              <p class="text-xs text-base-content-secondary">{member.user_email}</p>
            </div>
          </div>

          <div class="flex items-center justify-end gap-2">
            {#if canEdit && !member.is_owner}
              <Button
                variant="outline"
                size="sm"
                data-testid="edit-member-perms-{member.user_id}"
                onclick={() => {
                  editingMember = member;
                  modalOpen = true;
                }}
              >
                {d.workspaces.permissionsLabel}
              </Button>
            {/if}
            {#if canDelete && !member.is_owner}
              <Button
                variant="ghost"
                size="sm"
                data-testid="remove-member-{member.user_id}"
                class="text-rose-600 hover:bg-rose-500/10 hover:text-rose-600"
                onclick={() => handleRemove(member)}
              >
                {d.workspaces.removeMember}
              </Button>
            {/if}
          </div>
        </Card>
      {/each}
    </div>
  {/if}
</div>

{#if sessionStore.activeWorkspaceId}
  <MemberPermissionsModal
    open={modalOpen}
    workspaceId={sessionStore.activeWorkspaceId}
    member={editingMember}
    onClose={() => (modalOpen = false)}
    onSaved={(updated) => {
      members = members.map((m) => (m.user_id === updated.user_id ? updated : m));
      toasts.success(d.todos.updatedSuccess);
    }}
  />
{/if}
