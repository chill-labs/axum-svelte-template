<script lang="ts">
  import type { SortOrder, TodoPriority, TodoStatus } from '#lib/api/types.generated.ts';
  import Button from '#lib/components/ui/Button.svelte';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';

  interface Props {
    search: string;
    status: TodoStatus | 'all';
    priority: TodoPriority | 'all';
    sortOrder: SortOrder;
    canCreate?: boolean;
    onSearchChange: (val: string) => void;
    onStatusChange: (val: TodoStatus | 'all') => void;
    onPriorityChange: (val: TodoPriority | 'all') => void;
    onToggleSort: () => void;
    onNewTask: () => void;
  }

  let {
    search,
    status,
    priority,
    sortOrder,
    canCreate = true,
    onSearchChange,
    onStatusChange,
    onPriorityChange,
    onToggleSort,
    onNewTask
  }: Props = $props();

  const d = $derived(uiPrefs.dict);
</script>

<div class="soft-card flex flex-col gap-3 p-4 lg:flex-row lg:items-center lg:justify-between">
  <div class="grid flex-1 grid-cols-1 gap-2.5 sm:grid-cols-4">
    <div class="sm:col-span-2">
      <label for="todo-filter-search" class="sr-only">{d.common.search}</label>
      <input
        id="todo-filter-search"
        data-testid="todo-search-input"
        type="search"
        value={search}
        oninput={(e) => onSearchChange((e.currentTarget as HTMLInputElement).value)}
        placeholder="{d.common.search}..."
        class="h-10 w-full rounded-xl border border-base-line bg-base-main px-3.5 text-sm text-base-content placeholder:text-base-content-muted focus:border-primary focus:outline-none"
      />
    </div>

    <div>
      <label for="todo-filter-status" class="sr-only">Status</label>
      <select
        id="todo-filter-status"
        data-testid="todo-status-filter"
        value={status}
        onchange={(e) =>
          onStatusChange((e.currentTarget as HTMLSelectElement).value as TodoStatus | 'all')}
        class="h-10 w-full cursor-pointer rounded-xl border border-base-line bg-base-main px-3 text-sm text-base-content focus:border-primary focus:outline-none"
      >
        <option value="all">{d.todos.filterAll}</option>
        <option value="todo">{d.todos.statusTodo}</option>
        <option value="in_progress">{d.todos.statusInProgress}</option>
        <option value="done">{d.todos.statusDone}</option>
      </select>
    </div>

    <div>
      <label for="todo-filter-priority" class="sr-only">Priority</label>
      <select
        id="todo-filter-priority"
        data-testid="todo-priority-filter"
        value={priority}
        onchange={(e) =>
          onPriorityChange((e.currentTarget as HTMLSelectElement).value as TodoPriority | 'all')}
        class="h-10 w-full cursor-pointer rounded-xl border border-base-line bg-base-main px-3 text-sm text-base-content focus:border-primary focus:outline-none"
      >
        <option value="all">{d.todos.filterAll}</option>
        <option value="low">{d.todos.priorityLow}</option>
        <option value="medium">{d.todos.priorityMedium}</option>
        <option value="high">{d.todos.priorityHigh}</option>
        <option value="urgent">{d.todos.priorityUrgent}</option>
      </select>
    </div>
  </div>

  <div class="flex items-center gap-2">
    <Button variant="outline" size="md" onclick={onToggleSort} aria-label="Toggle sort order">
      {sortOrder === 'asc' ? '↑ Pos' : '↓ Pos'}
    </Button>

    {#if canCreate}
      <Button
        variant="primary"
        size="md"
        data-testid="new-todo-button"
        onclick={onNewTask}
      >
        + {d.todos.newTask}
      </Button>
    {/if}
  </div>
</div>
