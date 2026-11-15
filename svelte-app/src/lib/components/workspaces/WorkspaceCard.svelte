<script lang="ts">
  import type { WorkspaceResponse } from '#lib/api/types.generated.ts';
  import Badge from '#lib/components/ui/Badge.svelte';
  import Button from '#lib/components/ui/Button.svelte';
  import ProtectedImage from '#lib/components/ui/ProtectedImage.svelte';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';

  interface Props {
    workspace: WorkspaceResponse;
    isActive: boolean;
    canDeleteAny: boolean;
    onSwitch: (ws: WorkspaceResponse) => void;
    onEdit: (ws: WorkspaceResponse) => void;
    onDelete: (ws: WorkspaceResponse) => void;
  }

  let { workspace, isActive, canDeleteAny, onSwitch, onEdit, onDelete }: Props = $props();

  const d = $derived(uiPrefs.dict);

  const canEdit = $derived(
    workspace.is_owner ||
      workspace.my_permissions.some((p) => p.resource === 'workspace' && p.action === 'edit')
  );
  const canDelete = $derived(
    workspace.is_owner ||
      workspace.my_permissions.some((p) => p.resource === 'workspace' && p.action === 'delete')
  );
</script>

<article
  data-testid="workspace-card-{workspace.id}"
  class="soft-card flex flex-col justify-between gap-4 p-5 transition {isActive
    ? 'ring-2 ring-primary/50'
    : 'hover:border-primary/35'}"
>
  <div class="space-y-3">
    <div class="flex items-start justify-between gap-3">
      <div class="flex min-w-0 items-center gap-3">
        {#if workspace.logo_key}
          <ProtectedImage
            entityType="workspace"
            entityId={workspace.id}
            mediaKey={workspace.logo_key}
            alt={workspace.name}
            fallbackText={workspace.name}
            class="h-11 w-11 shrink-0 rounded-2xl object-cover"
          />
        {:else}
          <div
            class="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl text-sm font-bold text-white shadow-sm"
            style:background-color={workspace.color || '#3b82f6'}
          >
            {workspace.name.slice(0, 2).toUpperCase()}
          </div>
        {/if}

        <div class="min-w-0">
          <h3 class="truncate text-base font-bold text-base-content">{workspace.name}</h3>
          <p class="truncate font-mono text-xs text-base-content-muted">/{workspace.slug}</p>
        </div>
      </div>

      <div class="flex shrink-0 flex-wrap gap-1.5">
        {#if isActive}
          <Badge tone="primary">{d.workspaces.activeBadge}</Badge>
        {/if}
        <Badge tone={workspace.is_owner ? 'success' : 'neutral'}>
          {workspace.is_owner ? d.workspaces.ownerBadge : workspace.my_role}
        </Badge>
      </div>
    </div>

    {#if workspace.description}
      <p class="text-xs leading-relaxed text-base-content-secondary">{workspace.description}</p>
    {/if}

    <div class="flex items-center gap-4 text-xs text-base-content-secondary">
      <span>
        <strong class="text-base-content">{workspace.completed_todo_count}/{workspace.todo_count}</strong>
        {d.workspaces.tasksCount}
      </span>
      <span>
        <strong class="text-base-content">{workspace.member_count}</strong>
        {d.workspaces.membersCount}
      </span>
    </div>
  </div>

  <div class="flex flex-wrap items-center justify-between gap-2 border-t border-base-line pt-3">
    {#if !isActive}
      <Button
        variant="outline"
        size="sm"
        data-testid="workspace-switch-{workspace.id}"
        onclick={() => onSwitch(workspace)}
      >
        {d.workspaces.switchHere}
      </Button>
    {:else}
      <span class="text-xs font-semibold text-primary">✓ {d.workspaces.activeBadge}</span>
    {/if}

    <div class="flex items-center gap-1.5">
      {#if canEdit}
        <Button
          variant="ghost"
          size="sm"
          data-testid="workspace-edit-{workspace.id}"
          onclick={() => onEdit(workspace)}
        >
          {d.common.edit}
        </Button>
      {/if}
      {#if canDelete}
        <Button
          variant="ghost"
          size="sm"
          disabled={!canDeleteAny}
          title={!canDeleteAny ? d.workspaces.cannotDeleteLast : undefined}
          data-testid="workspace-delete-{workspace.id}"
          class="text-rose-600 hover:bg-rose-500/10 hover:text-rose-600"
          onclick={() => onDelete(workspace)}
        >
          {d.common.delete}
        </Button>
      {/if}
    </div>
  </div>
</article>
