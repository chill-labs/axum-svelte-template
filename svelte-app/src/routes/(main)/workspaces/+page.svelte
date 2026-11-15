<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type { WorkspaceResponse } from '#lib/api/types.generated.ts';
  import Button from '#lib/components/ui/Button.svelte';
  import EmptyState from '#lib/components/ui/EmptyState.svelte';
  import WorkspaceCard from '#lib/components/workspaces/WorkspaceCard.svelte';
  import WorkspaceModal from '#lib/components/workspaces/WorkspaceModal.svelte';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  const d = $derived(uiPrefs.dict);

  let workspaces = $state<WorkspaceResponse[]>([]);
  let loading = $state(false);
  let modalOpen = $state(false);
  let editingWorkspace = $state<WorkspaceResponse | null>(null);

  async function loadWorkspaces() {
    if (!sessionStore.isAuthenticated) return;
    loading = true;
    try {
      const res = await apiRequest<WorkspaceResponse[]>(API_ENDPOINTS.WORKSPACES_LIST_WORKSPACES);
      if (res.success && res.data) {
        workspaces = res.data;
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void sessionStore.user?.id;
    void loadWorkspaces();
  });

  function openCreate() {
    editingWorkspace = null;
    modalOpen = true;
  }

  function openEdit(ws: WorkspaceResponse) {
    editingWorkspace = ws;
    modalOpen = true;
  }

  async function handleSwitch(ws: WorkspaceResponse) {
    const ok = await sessionStore.switchWorkspace(ws.id);
    if (ok) {
      toasts.success(ws.name);
    }
  }

  async function handleDelete(ws: WorkspaceResponse) {
    const res = await apiRequest<boolean>(API_ENDPOINTS.WORKSPACES_DELETE_WORKSPACE, {
      params: { workspace_id: ws.id }
    });
    if (res.success) {
      toasts.success(d.todos.deletedSuccess);
      await sessionStore.refresh();
      await loadWorkspaces();
    } else {
      toasts.error(uiPrefs.err(res.message));
    }
  }

  async function handleSaved(saved: WorkspaceResponse) {
    const wasCreate = editingWorkspace === null;
    await loadWorkspaces();
    if (wasCreate) {
      await sessionStore.switchWorkspace(saved.id);
    } else if (saved.id === sessionStore.activeWorkspaceId) {
      await sessionStore.refresh();
    }
    toasts.success(wasCreate ? d.todos.createdSuccess : d.todos.updatedSuccess);
  }
</script>

<div class="space-y-5">
  <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
    <div>
      <h1 class="text-2xl font-bold tracking-tight text-base-content">{d.workspaces.pageTitle}</h1>
      <p class="mt-0.5 text-sm text-base-content-secondary">{d.workspaces.pageSubtitle}</p>
    </div>

    <Button variant="primary" data-testid="create-workspace-button" onclick={openCreate}>
      + {d.workspaces.createWorkspace}
    </Button>
  </div>

  {#if loading && workspaces.length === 0}
    <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
      {#each [1, 2] as idx (idx)}
        <div class="soft-card h-44 animate-pulse bg-base-sub/60"></div>
      {/each}
    </div>
  {:else if workspaces.length === 0}
    <EmptyState title={d.common.empty}>
      {#snippet action()}
        <Button variant="primary" onclick={openCreate}>+ {d.workspaces.createWorkspace}</Button>
      {/snippet}
    </EmptyState>
  {:else}
    <div class="grid grid-cols-1 gap-4 md:grid-cols-2" data-testid="workspaces-grid">
      {#each workspaces as ws (ws.id)}
        <WorkspaceCard
          workspace={ws}
          isActive={ws.id === sessionStore.activeWorkspaceId}
          canDeleteAny={workspaces.length > 1}
          onSwitch={handleSwitch}
          onEdit={openEdit}
          onDelete={handleDelete}
        />
      {/each}
    </div>
  {/if}
</div>

<WorkspaceModal
  open={modalOpen}
  workspace={editingWorkspace}
  onClose={() => (modalOpen = false)}
  onSaved={handleSaved}
/>
