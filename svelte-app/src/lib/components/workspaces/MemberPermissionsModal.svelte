<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type {
    MemberRolePreset,
    PermAction,
    PermResource,
    Permission,
    UpdateMemberPermissionsRequest,
    WorkspaceMemberResponse
  } from '#lib/api/types.generated.ts';
  import Button from '#lib/components/ui/Button.svelte';
  import Modal from '#lib/components/ui/Modal.svelte';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  interface Props {
    open: boolean;
    workspaceId: number;
    member: WorkspaceMemberResponse | null;
    onClose: () => void;
    onSaved: (member: WorkspaceMemberResponse) => void;
  }

  let { open, workspaceId, member, onClose, onSaved }: Props = $props();

  const d = $derived(uiPrefs.dict);

  const RESOURCES: PermResource[] = ['workspace', 'members', 'todos'];
  const ACTIONS: PermAction[] = ['view', 'create', 'edit', 'delete'];

  let role = $state<MemberRolePreset>('member');
  let permissions = $state<Permission[]>([]);
  let saving = $state(false);

  function presetPermissions(preset: MemberRolePreset): Permission[] {
    if (preset === 'editor') {
      return [
        { resource: 'workspace', action: 'view' },
        { resource: 'members', action: 'view' },
        { resource: 'todos', action: 'view' },
        { resource: 'todos', action: 'create' },
        { resource: 'todos', action: 'edit' },
        { resource: 'todos', action: 'delete' }
      ];
    }
    if (preset === 'member') {
      return [
        { resource: 'workspace', action: 'view' },
        { resource: 'members', action: 'view' },
        { resource: 'todos', action: 'view' },
        { resource: 'todos', action: 'create' },
        { resource: 'todos', action: 'edit' }
      ];
    }
    if (preset === 'viewer') {
      return [
        { resource: 'workspace', action: 'view' },
        { resource: 'todos', action: 'view' }
      ];
    }
    return [...permissions];
  }

  $effect(() => {
    if (!open || !member) return;
    const rawRole = member.role;
    if (rawRole === 'editor' || rawRole === 'member' || rawRole === 'viewer' || rawRole === 'custom') {
      role = rawRole;
    } else {
      role = 'custom';
    }
    permissions = member.permissions.map((p) => ({ ...p }));
  });

  function hasPerm(resource: PermResource, action: PermAction): boolean {
    return permissions.some((p) => p.resource === resource && p.action === action);
  }

  function togglePerm(resource: PermResource, action: PermAction) {
    role = 'custom';
    if (hasPerm(resource, action)) {
      permissions = permissions.filter((p) => !(p.resource === resource && p.action === action));
    } else {
      permissions = [...permissions, { resource, action }];
    }
  }

  function handlePresetChange(nextRole: MemberRolePreset) {
    role = nextRole;
    if (nextRole !== 'custom') {
      permissions = presetPermissions(nextRole);
    }
  }

  async function handleSave() {
    if (!member) return;
    saving = true;
    try {
      const res = await apiRequest<WorkspaceMemberResponse, UpdateMemberPermissionsRequest>(
        API_ENDPOINTS.WORKSPACES_UPDATE_MEMBER_PERMISSIONS,
        {
          params: { workspace_id: workspaceId, member_user_id: member.user_id },
          body: { role, permissions }
        }
      );
      if (res.success && res.data) {
        onSaved(res.data);
        onClose();
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      saving = false;
    }
  }
</script>

<Modal
  {open}
  title="{d.workspaces.permissionsLabel} — {member?.user_name ?? ''}"
  maxWidthClass="max-w-lg"
  {onClose}
>
  <div class="space-y-4">
    <div class="space-y-1.5">
      <label
        for="member-role-preset"
        class="block text-xs font-semibold tracking-wide text-base-content-secondary"
      >
        {d.workspaces.rolePreset}
      </label>
      <select
        id="member-role-preset"
        data-testid="member-role-preset-select"
        value={role}
        onchange={(e) =>
          handlePresetChange((e.currentTarget as HTMLSelectElement).value as MemberRolePreset)}
        class="h-10 w-full cursor-pointer rounded-xl border border-base-line bg-base-sub px-3 text-sm text-base-content focus:border-primary focus:outline-none"
      >
        <option value="editor">editor</option>
        <option value="member">member</option>
        <option value="viewer">viewer</option>
        <option value="custom">custom</option>
      </select>
    </div>

    <div class="overflow-x-auto rounded-2xl border border-base-line">
      <table class="w-full text-left text-xs">
        <thead class="border-b border-base-line bg-base-main text-base-content-secondary uppercase">
          <tr>
            <th class="px-3 py-2.5 font-semibold">Resource</th>
            {#each ACTIONS as action (action)}
              <th class="px-3 py-2.5 text-center font-semibold">{action}</th>
            {/each}
          </tr>
        </thead>
        <tbody class="divide-y divide-base-line bg-base-sub">
          {#each RESOURCES as resource (resource)}
            <tr>
              <td class="px-3 py-2.5 font-semibold text-base-content capitalize">{resource}</td>
              {#each ACTIONS as action (action)}
                <td class="px-3 py-2.5 text-center">
                  <input
                    type="checkbox"
                    aria-label="{resource} {action}"
                    data-testid="perm-{resource}-{action}"
                    checked={hasPerm(resource, action)}
                    onchange={() => togglePerm(resource, action)}
                    class="h-4 w-4 cursor-pointer accent-primary"
                  />
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>

  {#snippet footer()}
    <Button variant="ghost" onclick={onClose}>{d.common.cancel}</Button>
    <Button
      variant="primary"
      loading={saving}
      data-testid="member-permissions-save"
      onclick={handleSave}
    >
      {saving ? d.common.saving : d.common.save}
    </Button>
  {/snippet}
</Modal>
