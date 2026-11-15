<script lang="ts">
  import type { TodoPriority, TodoResponse, TodoStatus } from '#lib/api/types.generated.ts';
  import Badge from '#lib/components/ui/Badge.svelte';
  import Button from '#lib/components/ui/Button.svelte';
  import ProtectedImage from '#lib/components/ui/ProtectedImage.svelte';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';

  interface Props {
    todo: TodoResponse;
    canEdit: boolean;
    canDelete: boolean;
    canMoveUp?: boolean;
    canMoveDown?: boolean;
    onToggleComplete: (todo: TodoResponse) => void;
    onEdit: (todo: TodoResponse) => void;
    onDelete: (todo: TodoResponse) => void;
    onMoveUp?: (todo: TodoResponse) => void;
    onMoveDown?: (todo: TodoResponse) => void;
  }

  let {
    todo,
    canEdit,
    canDelete,
    canMoveUp = false,
    canMoveDown = false,
    onToggleComplete,
    onEdit,
    onDelete,
    onMoveUp,
    onMoveDown
  }: Props = $props();

  const d = $derived(uiPrefs.dict);

  const priorityTone: Record<TodoPriority, 'neutral' | 'info' | 'warning' | 'danger'> = {
    low: 'neutral',
    medium: 'info',
    high: 'warning',
    urgent: 'danger'
  };

  const priorityLabel = $derived.by(() => {
    switch (todo.priority) {
      case 'low':
        return d.todos.priorityLow;
      case 'medium':
        return d.todos.priorityMedium;
      case 'high':
        return d.todos.priorityHigh;
      case 'urgent':
        return d.todos.priorityUrgent;
    }
  });

  const statusTone: Record<TodoStatus, 'neutral' | 'primary' | 'success'> = {
    todo: 'neutral',
    in_progress: 'primary',
    done: 'success'
  };

  const statusLabel = $derived.by(() => {
    switch (todo.status) {
      case 'todo':
        return d.todos.statusTodo;
      case 'in_progress':
        return d.todos.statusInProgress;
      case 'done':
        return d.todos.statusDone;
    }
  });
</script>

<article
  data-testid="todo-item-{todo.id}"
  class="soft-card flex flex-col gap-3 p-4 transition hover:border-primary/35 sm:flex-row sm:items-start sm:justify-between"
>
  <div class="flex min-w-0 flex-1 items-start gap-3">
    <button
      type="button"
      disabled={!canEdit}
      data-testid="todo-toggle-{todo.id}"
      aria-label={todo.is_completed ? 'Mark todo incomplete' : 'Mark todo complete'}
      onclick={() => onToggleComplete(todo)}
      class="mt-0.5 flex h-5 w-5 shrink-0 cursor-pointer items-center justify-center rounded-md border transition disabled:cursor-not-allowed disabled:opacity-50 {todo.is_completed
        ? 'border-emerald-500 bg-emerald-500 text-white'
        : 'border-base-line bg-base-main hover:border-primary'}"
    >
      {#if todo.is_completed}
        <svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
          <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
        </svg>
      {/if}
    </button>

    <div class="min-w-0 flex-1 space-y-1.5">
      <div class="flex flex-wrap items-center gap-2">
        <h3
          class="text-sm font-semibold {todo.is_completed
            ? 'text-base-content-muted line-through'
            : 'text-base-content'}"
        >
          {todo.title}
        </h3>
        <Badge tone={priorityTone[todo.priority]}>{priorityLabel}</Badge>
        <Badge tone={statusTone[todo.status]}>{statusLabel}</Badge>
      </div>

      {#if todo.description}
        <p class="text-xs leading-relaxed text-base-content-secondary">
          {todo.description}
        </p>
      {/if}

      <div class="flex flex-wrap items-center gap-3 text-[11px] text-base-content-muted">
        {#if todo.due_date}
          <span class="rounded-lg bg-base-elevated px-2 py-0.5 font-medium text-base-content-secondary">
            {d.todos.dueDate}: {todo.due_date}
          </span>
        {/if}
        {#if todo.assigned_to_name}
          <span class="rounded-lg bg-base-elevated px-2 py-0.5 font-medium text-base-content-secondary">
            {d.todos.assignee}: {todo.assigned_to_name}
          </span>
        {/if}
      </div>

      {#if todo.attachment_key}
        <div class="pt-1">
          <ProtectedImage
            entityType="todo"
            entityId={todo.id}
            mediaKey={todo.attachment_key}
            alt={todo.title}
            fallbackText={todo.title}
            class="h-16 w-24 rounded-xl border border-base-line object-cover"
          />
        </div>
      {/if}
    </div>
  </div>

  <div class="flex shrink-0 items-center justify-end gap-1.5">
    {#if canEdit && onMoveUp}
      <button
        type="button"
        disabled={!canMoveUp}
        onclick={() => onMoveUp(todo)}
        aria-label="Move up"
        class="flex h-8 w-8 cursor-pointer items-center justify-center rounded-xl border border-base-line bg-base-main text-xs text-base-content-secondary transition hover:bg-base-elevated disabled:cursor-not-allowed disabled:opacity-40"
      >
        ↑
      </button>
    {/if}
    {#if canEdit && onMoveDown}
      <button
        type="button"
        disabled={!canMoveDown}
        onclick={() => onMoveDown(todo)}
        aria-label="Move down"
        class="flex h-8 w-8 cursor-pointer items-center justify-center rounded-xl border border-base-line bg-base-main text-xs text-base-content-secondary transition hover:bg-base-elevated disabled:cursor-not-allowed disabled:opacity-40"
      >
        ↓
      </button>
    {/if}
    {#if canEdit}
      <Button
        variant="ghost"
        size="sm"
        data-testid="todo-edit-{todo.id}"
        onclick={() => onEdit(todo)}
      >
        {d.common.edit}
      </Button>
    {/if}
    {#if canDelete}
      <Button
        variant="ghost"
        size="sm"
        data-testid="todo-delete-{todo.id}"
        class="text-rose-600 hover:bg-rose-500/10 hover:text-rose-600"
        onclick={() => onDelete(todo)}
      >
        {d.common.delete}
      </Button>
    {/if}
  </div>
</article>
