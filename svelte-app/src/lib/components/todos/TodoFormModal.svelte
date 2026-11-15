<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type {
    CreateTodoRequest,
    TodoPriority,
    TodoResponse,
    TodoStatus,
    UpdateTodoRequest,
    WorkspaceMemberResponse
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
    workspaceId: number;
    todo: TodoResponse | null;
    members: WorkspaceMemberResponse[];
    onClose: () => void;
    onSaved: () => void;
  }

  let { open, workspaceId, todo, members, onClose, onSaved }: Props = $props();

  const d = $derived(uiPrefs.dict);

  let title = $state('');
  let description = $state('');
  let status = $state<TodoStatus>('todo');
  let priority = $state<TodoPriority>('medium');
  let dueDate = $state('');
  let assignedTo = $state('');
  let saving = $state(false);
  let fieldError = $state('');
  let attachmentKey = $state<string | null>(null);

  $effect(() => {
    if (!open) return;
    fieldError = '';
    if (todo) {
      title = todo.title;
      description = todo.description ?? '';
      status = todo.status;
      priority = todo.priority;
      dueDate = todo.due_date ?? '';
      assignedTo = todo.assigned_to ? String(todo.assigned_to) : '';
      attachmentKey = todo.attachment_key ?? null;
    } else {
      title = '';
      description = '';
      status = 'todo';
      priority = 'medium';
      dueDate = '';
      assignedTo = '';
      attachmentKey = null;
    }
  });

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    fieldError = '';
    const trimmedTitle = title.trim();
    if (!trimmedTitle) {
      fieldError = uiPrefs.err('FIELD_REQUIRED');
      return;
    }

    saving = true;
    try {
      if (todo) {
        const body: UpdateTodoRequest = {
          title: trimmedTitle,
          description: description.trim() ? description.trim() : null,
          status,
          priority,
          due_date: dueDate ? dueDate : null,
          assigned_to: assignedTo ? Number(assignedTo) : null,
          is_completed: status === 'done'
        };
        const res = await apiRequest<TodoResponse, UpdateTodoRequest>(
          API_ENDPOINTS.TODOS_UPDATE_TODO,
          {
            params: { workspace_id: workspaceId, todo_id: todo.id },
            body
          }
        );
        if (res.success) {
          toasts.success(d.todos.updatedSuccess);
          onSaved();
          onClose();
        } else {
          fieldError = uiPrefs.err(res.errors?.title ?? res.message);
          toasts.error(uiPrefs.err(res.message));
        }
      } else {
        const body: CreateTodoRequest = {
          title: trimmedTitle,
          description: description.trim() ? description.trim() : null,
          status,
          priority,
          due_date: dueDate ? dueDate : null,
          assigned_to: assignedTo ? Number(assignedTo) : null
        };
        const res = await apiRequest<TodoResponse, CreateTodoRequest>(
          API_ENDPOINTS.TODOS_CREATE_TODO,
          {
            params: { workspace_id: workspaceId },
            body
          }
        );
        if (res.success) {
          toasts.success(d.todos.createdSuccess);
          onSaved();
          onClose();
        } else {
          fieldError = uiPrefs.err(res.errors?.title ?? res.message);
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
  title={todo ? d.todos.editTask : d.todos.newTask}
  maxWidthClass="max-w-lg"
  {onClose}
>
  <form id="todo-modal-form" onsubmit={handleSubmit} class="space-y-4">
    <Input
      id="todo-form-title"
      data-testid="todo-form-title"
      label={d.todos.addPlaceholder}
      bind:value={title}
      error={fieldError}
      required
    />

    <div class="space-y-1.5">
      <label
        for="todo-form-description"
        class="block text-xs font-semibold tracking-wide text-base-content-secondary"
      >
        {d.todos.descriptionPlaceholder}
      </label>
      <textarea
        id="todo-form-description"
        data-testid="todo-form-description"
        rows="3"
        bind:value={description}
        placeholder={d.todos.descriptionPlaceholder}
        class="w-full rounded-xl border border-base-line bg-base-sub px-3.5 py-2.5 text-sm text-base-content placeholder:text-base-content-muted focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/25"
      ></textarea>
    </div>

    <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
      <div class="space-y-1.5">
        <label
          for="todo-form-status"
          class="block text-xs font-semibold tracking-wide text-base-content-secondary"
        >
          Status
        </label>
        <select
          id="todo-form-status"
          data-testid="todo-form-status"
          bind:value={status}
          class="h-10 w-full cursor-pointer rounded-xl border border-base-line bg-base-sub px-3 text-sm text-base-content focus:border-primary focus:outline-none"
        >
          <option value="todo">{d.todos.statusTodo}</option>
          <option value="in_progress">{d.todos.statusInProgress}</option>
          <option value="done">{d.todos.statusDone}</option>
        </select>
      </div>

      <div class="space-y-1.5">
        <label
          for="todo-form-priority"
          class="block text-xs font-semibold tracking-wide text-base-content-secondary"
        >
          Priority
        </label>
        <select
          id="todo-form-priority"
          data-testid="todo-form-priority"
          bind:value={priority}
          class="h-10 w-full cursor-pointer rounded-xl border border-base-line bg-base-sub px-3 text-sm text-base-content focus:border-primary focus:outline-none"
        >
          <option value="low">{d.todos.priorityLow}</option>
          <option value="medium">{d.todos.priorityMedium}</option>
          <option value="high">{d.todos.priorityHigh}</option>
          <option value="urgent">{d.todos.priorityUrgent}</option>
        </select>
      </div>
    </div>

    <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <label
            for="todo-form-due-date"
            class="block text-xs font-semibold tracking-wide text-base-content-secondary"
          >
            {d.todos.dueDate}
          </label>
          {#if dueDate}
            <button
              type="button"
              data-testid="todo-clear-due-date"
              onclick={() => (dueDate = '')}
              class="cursor-pointer text-[11px] font-semibold text-primary hover:underline"
            >
              {d.todos.clearDueDate}
            </button>
          {/if}
        </div>
        <input
          id="todo-form-due-date"
          data-testid="todo-form-due-date"
          type="date"
          bind:value={dueDate}
          class="h-10 w-full rounded-xl border border-base-line bg-base-sub px-3 text-sm text-base-content focus:border-primary focus:outline-none"
        />
      </div>

      <div class="space-y-1.5">
        <label
          for="todo-form-assignee"
          class="block text-xs font-semibold tracking-wide text-base-content-secondary"
        >
          {d.todos.assignee}
        </label>
        <select
          id="todo-form-assignee"
          data-testid="todo-form-assignee"
          bind:value={assignedTo}
          class="h-10 w-full cursor-pointer rounded-xl border border-base-line bg-base-sub px-3 text-sm text-base-content focus:border-primary focus:outline-none"
        >
          <option value="">{d.todos.unassigned}</option>
          {#each members as member (member.user_id)}
            <option value={String(member.user_id)}>{member.user_name}</option>
          {/each}
        </select>
      </div>
    </div>

    {#if todo}
      <div class="space-y-2 rounded-2xl border border-base-line bg-base-main/60 p-3.5">
        <div class="flex items-center justify-between gap-2">
          <span class="text-xs font-semibold text-base-content-secondary">
            {d.todos.attachment}
          </span>
          <ImageUploader
            purpose="todo_attachment"
            entityId={todo.id}
            label={d.todos.uploadAttachment}
            onUploaded={(media) => {
              attachmentKey = media.storage_key;
              onSaved();
            }}
          />
        </div>
        {#if attachmentKey}
          <ProtectedImage
            entityType="todo"
            entityId={todo.id}
            mediaKey={attachmentKey}
            alt={title}
            fallbackText={title}
            class="h-20 w-32 rounded-xl border border-base-line object-cover"
          />
        {:else}
          <p class="text-[11px] text-base-content-muted">{d.common.optionalStorageDisabled}</p>
        {/if}
      </div>
    {/if}
  </form>

  {#snippet footer()}
    <Button variant="ghost" onclick={onClose}>{d.common.cancel}</Button>
    <Button
      type="submit"
      form="todo-modal-form"
      variant="primary"
      loading={saving}
      data-testid="todo-form-submit"
    >
      {saving ? d.common.saving : d.common.save}
    </Button>
  {/snippet}
</Modal>
