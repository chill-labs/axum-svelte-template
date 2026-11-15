<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type {
    CreateWorkspaceRequest,
    UpdateWorkspaceRequest,
    WorkspaceResponse
  } from '#lib/api/types.generated.ts';
  import Button from '#lib/components/ui/Button.svelte';
  import ImageUploader from '#lib/components/ui/ImageUploader.svelte';
  import Input from '#lib/components/ui/Input.svelte';
  import Modal from '#lib/components/ui/Modal.svelte';
  import ProtectedImage from '#lib/components/ui/ProtectedImage.svelte';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  interface Props {
    open: boolean;
    workspace: WorkspaceResponse | null;
    onClose: () => void;
    onSaved: (ws: WorkspaceResponse) => void;
  }

  let { open, workspace, onClose, onSaved }: Props = $props();

  const d = $derived(uiPrefs.dict);

  const COLOR_PRESETS = ['#3b82f6', '#8b5cf6', '#10b981', '#f59e0b', '#ef4444', '#ec4899'];

  let name = $state('');
  let slug = $state('');
  let description = $state('');
  let color = $state('#3b82f6');
  let logoKey = $state<string | null>(null);
  let saving = $state(false);
  let nameError = $state('');
  let slugError = $state('');

  $effect(() => {
    if (!open) return;
    nameError = '';
    slugError = '';
    if (workspace) {
      name = workspace.name;
      slug = workspace.slug;
      description = workspace.description ?? '';
      color = workspace.color || '#3b82f6';
      logoKey = workspace.logo_key ?? null;
    } else {
      name = '';
      slug = '';
      description = '';
      color = '#3b82f6';
      logoKey = null;
    }
  });

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    nameError = '';
    slugError = '';

    const trimmedName = name.trim();
    if (!trimmedName) {
      nameError = uiPrefs.err('FIELD_REQUIRED');
      return;
    }

    saving = true;
    try {
      if (workspace) {
        const body: UpdateWorkspaceRequest = {
          name: trimmedName,
          slug: slug.trim() || undefined,
          description: description.trim() ? description.trim() : null,
          color: color.trim() || '#3b82f6'
        };
        const res = await apiRequest<WorkspaceResponse, UpdateWorkspaceRequest>(
          API_ENDPOINTS.WORKSPACES_UPDATE_WORKSPACE,
          {
            params: { workspace_id: workspace.id },
            body
          }
        );
        if (res.success && res.data) {
          onSaved(res.data);
          onClose();
        } else {
          nameError = res.errors?.name ? uiPrefs.err(res.errors.name) : '';
          slugError = res.errors?.slug ? uiPrefs.err(res.errors.slug) : '';
          toasts.error(uiPrefs.err(res.message));
        }
      } else {
        const body: CreateWorkspaceRequest = {
          name: trimmedName,
          slug: slug.trim() ? slug.trim() : null,
          description: description.trim() ? description.trim() : null,
          color: color.trim() || '#3b82f6'
        };
        const res = await apiRequest<WorkspaceResponse, CreateWorkspaceRequest>(
          API_ENDPOINTS.WORKSPACES_CREATE_WORKSPACE,
          { body }
        );
        if (res.success && res.data) {
          onSaved(res.data);
          onClose();
        } else {
          nameError = res.errors?.name ? uiPrefs.err(res.errors.name) : '';
          slugError = res.errors?.slug ? uiPrefs.err(res.errors.slug) : '';
          toasts.error(uiPrefs.err(res.message));
        }
      }
    } finally {
      saving = false;
    }
  }
</script>

<Modal
  {open}
  title={workspace ? d.workspaces.editWorkspace : d.workspaces.createWorkspace}
  maxWidthClass="max-w-lg"
  {onClose}
>
  <form id="workspace-modal-form" onsubmit={handleSubmit} class="space-y-4">
    <Input
      id="ws-form-name"
      data-testid="workspace-form-name"
      label={d.workspaces.workspaceName}
      bind:value={name}
      error={nameError}
      required
    />

    <Input
      id="ws-form-slug"
      data-testid="workspace-form-slug"
      label={d.workspaces.workspaceSlug}
      bind:value={slug}
      error={slugError}
      placeholder="team-roadmap"
    />

    <div class="space-y-1.5">
      <label
        for="ws-form-description"
        class="block text-xs font-semibold tracking-wide text-base-content-secondary"
      >
        {d.workspaces.workspaceDescription}
      </label>
      <textarea
        id="ws-form-description"
        data-testid="workspace-form-description"
        rows="2"
        bind:value={description}
        class="w-full rounded-xl border border-base-line bg-base-sub px-3.5 py-2 text-sm text-base-content focus:border-primary focus:outline-none"
      ></textarea>
    </div>

    <div class="space-y-2">
      <label
        for="ws-form-color"
        class="block text-xs font-semibold tracking-wide text-base-content-secondary"
      >
        {d.workspaces.workspaceColor}
      </label>
      <div class="flex flex-wrap items-center gap-2">
        {#each COLOR_PRESETS as preset (preset)}
          <button
            type="button"
            onclick={() => (color = preset)}
            aria-label="Select color {preset}"
            class="h-7 w-7 cursor-pointer rounded-full border-2 transition {color.toLowerCase() ===
            preset
              ? 'scale-110 border-base-content'
              : 'border-transparent'}"
            style:background-color={preset}
          ></button>
        {/each}
        <input
          id="ws-form-color"
          type="text"
          bind:value={color}
          class="h-8 w-28 rounded-xl border border-base-line bg-base-sub px-2.5 font-mono text-xs text-base-content focus:border-primary focus:outline-none"
        />
      </div>
    </div>

    {#if workspace}
      <div class="flex items-center justify-between gap-3 rounded-2xl border border-base-line bg-base-main/60 p-3.5">
        <div class="flex items-center gap-3">
          <ProtectedImage
            entityType="workspace"
            entityId={workspace.id}
            mediaKey={logoKey}
            alt={name}
            fallbackText={name}
            class="h-10 w-10 rounded-xl object-cover"
          />
          <span class="text-xs text-base-content-secondary">{d.common.optionalStorageDisabled}</span>
        </div>
        <ImageUploader
          purpose="workspace_logo"
          entityId={workspace.id}
          onUploaded={(media) => {
            logoKey = media.storage_key;
          }}
        />
      </div>
    {/if}
  </form>

  {#snippet footer()}
    <Button variant="ghost" onclick={onClose}>{d.common.cancel}</Button>
    <Button
      type="submit"
      form="workspace-modal-form"
      variant="primary"
      loading={saving}
      data-testid="workspace-form-submit"
    >
      {saving ? d.common.saving : d.common.save}
    </Button>
  {/snippet}
</Modal>
