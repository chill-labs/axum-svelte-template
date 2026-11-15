<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type {
    CreateTodoRequest,
    Pagination,
    SortOrder,
    TodoListPayload,
    TodoPriority,
    TodoResponse,
    TodoStatus,
    TodoSummaryStats,
    UpdateTodoRequest,
    WorkspaceMemberResponse
  } from '#lib/api/types.generated.ts';
  import TodoFilterBar from '#lib/components/todos/TodoFilterBar.svelte';
  import TodoFormModal from '#lib/components/todos/TodoFormModal.svelte';
  import TodoItemCard from '#lib/components/todos/TodoItemCard.svelte';
  import TodoStatsBar from '#lib/components/todos/TodoStatsBar.svelte';
  import Button from '#lib/components/ui/Button.svelte';
  import EmptyState from '#lib/components/ui/EmptyState.svelte';
  import { ROUTES } from '#lib/config/routes.ts';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { sseClient } from '#lib/state/sse.svelte.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  const d = $derived(uiPrefs.dict);

  const GUEST_DEMO_TODOS: TodoResponse[] = [
    {
      id: -1,
      workspace_id: 0,
      created_by: 0,
      created_by_name: 'Template Bot',
      title: 'Explore multi-workspace RBAC & REST + TS codegen',
      description: 'Run `cargo xtask codegen` to regenerate strict TypeScript API contracts.',
      status: 'in_progress',
      priority: 'urgent',
      due_date: null,
      is_completed: false,
      completed_at: null,
      position: 1,
      attachment_key: null,
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z'
    },
    {
      id: -2,
      workspace_id: 0,
      created_by: 0,
      created_by_name: 'Template Bot',
      title: 'Test Light/Dark theme persistence & bilingual VI/EN switch',
      description: 'Cookies are injected SSR-side by Axum shell.rs to eliminate first-paint flash.',
      status: 'todo',
      priority: 'high',
      due_date: null,
      is_completed: false,
      completed_at: null,
      position: 2,
      attachment_key: null,
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z'
    },
    {
      id: -3,
      workspace_id: 0,
      created_by: 0,
      created_by_name: 'Template Bot',
      title: 'Sign in with Google or a Test Account to unlock persistence',
      description: null,
      status: 'done',
      priority: 'medium',
      due_date: null,
      is_completed: true,
      completed_at: '2026-01-01T00:00:00Z',
      position: 3,
      attachment_key: null,
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z'
    }
  ];

  const GUEST_DEMO_STATS: TodoSummaryStats = {
    total: 3,
    todo_count: 1,
    in_progress_count: 1,
    done_count: 1,
    urgent_count: 1
  };

  let items = $state<TodoResponse[]>([]);
  let stats = $state<TodoSummaryStats>({
    total: 0,
    todo_count: 0,
    in_progress_count: 0,
    done_count: 0,
    urgent_count: 0
  });
  let pagination = $state<Pagination>({ page: 1, page_size: 20, total_items: 0, total_pages: 1 });
  let members = $state<WorkspaceMemberResponse[]>([]);

  let loading = $state(false);
  let search = $state('');
  let statusFilter = $state<TodoStatus | 'all'>('all');
  let priorityFilter = $state<TodoPriority | 'all'>('all');
  let sortOrder = $state<SortOrder>('asc');
  let quickTitle = $state('');
  let quickAdding = $state(false);

  let modalOpen = $state(false);
  let editingTodo = $state<TodoResponse | null>(null);

  const canView = $derived(sessionStore.isAuthenticated ? sessionStore.can('todos', 'view') : true);
  const canCreate = $derived(
    sessionStore.isAuthenticated ? sessionStore.can('todos', 'create') : true
  );
  const canEdit = $derived(sessionStore.isAuthenticated ? sessionStore.can('todos', 'edit') : true);
  const canDelete = $derived(
    sessionStore.isAuthenticated ? sessionStore.can('todos', 'delete') : true
  );
  const isReadOnlyMember = $derived(
    sessionStore.isAuthenticated && !sessionStore.can('todos', 'create') && !sessionStore.can('todos', 'edit')
  );

  async function loadTodos(pageNum = pagination.page) {
    const wsId = sessionStore.activeWorkspaceId;
    if (!sessionStore.isAuthenticated || !wsId) return;

    loading = true;
    try {
      const res = await apiRequest<TodoListPayload>(API_ENDPOINTS.TODOS_LIST_TODOS, {
        params: { workspace_id: wsId },
        query: {
          page: pageNum,
          page_size: 20,
          search: search.trim() || undefined,
          status: statusFilter === 'all' ? undefined : statusFilter,
          priority: priorityFilter === 'all' ? undefined : priorityFilter,
          sort_order: sortOrder
        }
      });
      if (res.success && res.data) {
        items = res.data.items;
        stats = res.data.stats;
        if (res.pagination) pagination = res.pagination;
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      loading = false;
    }
  }

  async function loadMembers() {
    const wsId = sessionStore.activeWorkspaceId;
    if (!sessionStore.isAuthenticated || !wsId || !sessionStore.can('members', 'view')) {
      members = [];
      return;
    }
    const res = await apiRequest<WorkspaceMemberResponse[]>(API_ENDPOINTS.WORKSPACES_LIST_MEMBERS, {
      params: { workspace_id: wsId }
    });
    if (res.success && res.data) {
      members = res.data;
    }
  }

  $effect(() => {
    const wsId = sessionStore.activeWorkspaceId;
    void search;
    void statusFilter;
    void priorityFilter;
    void sortOrder;
    if (!sessionStore.isAuthenticated || !wsId) {
      items = [];
      return;
    }
    void loadTodos(1);
    void loadMembers();
  });

  $effect(() => {
    const unsub = sseClient.onTodoChanged((payload) => {
      if (payload.workspace_id === sessionStore.activeWorkspaceId) {
        void loadTodos(pagination.page);
      }
    });
    return unsub;
  });

  function openNewModal() {
    if (!sessionStore.requireAuth()) return;
    editingTodo = null;
    modalOpen = true;
  }

  function openEditModal(todo: TodoResponse) {
    if (!sessionStore.requireAuth()) return;
    editingTodo = todo;
    modalOpen = true;
  }

  async function handleQuickAdd(event: SubmitEvent) {
    event.preventDefault();
    if (!sessionStore.requireAuth()) return;
    const wsId = sessionStore.activeWorkspaceId;
    const trimmed = quickTitle.trim();
    if (!wsId || !trimmed) return;

    quickAdding = true;
    try {
      const res = await apiRequest<TodoResponse, CreateTodoRequest>(
        API_ENDPOINTS.TODOS_CREATE_TODO,
        {
          params: { workspace_id: wsId },
          body: {
            title: trimmed,
            status: 'todo',
            priority: 'medium'
          }
        }
      );
      if (res.success) {
        quickTitle = '';
        toasts.success(d.todos.createdSuccess);
        await loadTodos(1);
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      quickAdding = false;
    }
  }

  async function handleToggleComplete(todo: TodoResponse) {
    if (!sessionStore.requireAuth()) return;
    const wsId = sessionStore.activeWorkspaceId;
    if (!wsId) return;

    const nextCompleted = !todo.is_completed;
    const res = await apiRequest<TodoResponse, UpdateTodoRequest>(
      API_ENDPOINTS.TODOS_UPDATE_TODO,
      {
        params: { workspace_id: wsId, todo_id: todo.id },
        body: {
          is_completed: nextCompleted,
          status: nextCompleted ? 'done' : 'todo'
        }
      }
    );
    if (res.success) {
      await loadTodos(pagination.page);
    } else {
      toasts.error(uiPrefs.err(res.message));
    }
  }

  async function handleDelete(todo: TodoResponse) {
    if (!sessionStore.requireAuth()) return;
    const wsId = sessionStore.activeWorkspaceId;
    if (!wsId) return;

    const res = await apiRequest<boolean>(API_ENDPOINTS.TODOS_DELETE_TODO, {
      params: { workspace_id: wsId, todo_id: todo.id }
    });
    if (res.success) {
      toasts.success(d.todos.deletedSuccess);
      await loadTodos(pagination.page);
    } else {
      toasts.error(uiPrefs.err(res.message));
    }
  }

  async function moveTodo(todo: TodoResponse, direction: -1 | 1) {
    const wsId = sessionStore.activeWorkspaceId;
    if (!wsId) return;
    const idx = items.findIndex((i) => i.id === todo.id);
    const targetIdx = idx + direction;
    if (idx < 0 || targetIdx < 0 || targetIdx >= items.length) return;

    const nextItems = [...items];
    const [moved] = nextItems.splice(idx, 1);
    nextItems.splice(targetIdx, 0, moved);
    items = nextItems;

    const res = await apiRequest<boolean>(API_ENDPOINTS.TODOS_REORDER_TODOS, {
      params: { workspace_id: wsId },
      body: { ordered_ids: nextItems.map((i) => i.id) }
    });
    if (!res.success) {
      toasts.error(uiPrefs.err(res.message));
      await loadTodos(pagination.page);
    }
  }

  const displayedItems = $derived.by(() => {
    if (!sessionStore.isAuthenticated) {
      return GUEST_DEMO_TODOS.filter((item) => {
        if (statusFilter !== 'all' && item.status !== statusFilter) return false;
        if (priorityFilter !== 'all' && item.priority !== priorityFilter) return false;
        if (search.trim()) {
          const q = search.trim().toLowerCase();
          return (
            item.title.toLowerCase().includes(q) ||
            (item.description?.toLowerCase().includes(q) ?? false)
          );
        }
        return true;
      });
    }
    return items;
  });

  const displayedStats = $derived(sessionStore.isAuthenticated ? stats : GUEST_DEMO_STATS);
</script>

<div class="space-y-5">
  <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
    <div>
      <h1 class="text-2xl font-bold tracking-tight text-base-content">{d.todos.pageTitle}</h1>
      <p class="mt-0.5 text-sm text-base-content-secondary">{d.todos.pageSubtitle}</p>
    </div>
  </div>

  {#if !sessionStore.isAuthenticated}
    <div
      data-testid="guest-demo-banner"
      class="flex flex-col justify-between gap-3 rounded-2xl border border-primary/25 bg-primary/8 px-4 py-3 sm:flex-row sm:items-center"
    >
      <p class="text-xs font-medium text-base-content">{d.todos.guestBanner}</p>
      <div class="flex shrink-0 items-center gap-2">
        <a href={ROUTES.LOGIN}>
          <Button variant="primary" size="sm">{d.nav.signIn}</Button>
        </a>
      </div>
    </div>
  {/if}

  {#if isReadOnlyMember}
    <div
      data-testid="readonly-permission-banner"
      class="rounded-2xl border border-amber-500/35 bg-amber-500/10 px-4 py-2.5 text-xs font-semibold text-amber-800 dark:text-amber-300"
    >
      {d.todos.readOnlyPermission}
    </div>
  {/if}

  <TodoStatsBar stats={displayedStats} />

  {#if canCreate}
    <form onsubmit={handleQuickAdd} class="soft-card flex flex-col gap-2.5 p-3 sm:flex-row sm:items-center">
      <label for="quick-add-todo" class="sr-only">{d.todos.addPlaceholder}</label>
      <input
        id="quick-add-todo"
        data-testid="quick-add-todo-input"
        type="text"
        bind:value={quickTitle}
        placeholder={d.todos.addPlaceholder}
        class="h-10 flex-1 rounded-xl border border-base-line bg-base-main px-3.5 text-sm text-base-content placeholder:text-base-content-muted focus:border-primary focus:outline-none"
      />
      <Button
        type="submit"
        variant="primary"
        loading={quickAdding}
        data-testid="quick-add-todo-submit"
      >
        + {d.common.create}
      </Button>
    </form>
  {/if}

  <TodoFilterBar
    {search}
    status={statusFilter}
    priority={priorityFilter}
    {sortOrder}
    {canCreate}
    onSearchChange={(val) => (search = val)}
    onStatusChange={(val) => (statusFilter = val)}
    onPriorityChange={(val) => (priorityFilter = val)}
    onToggleSort={() => (sortOrder = sortOrder === 'asc' ? 'desc' : 'asc')}
    onNewTask={openNewModal}
  />

  {#if !canView}
    <EmptyState title={d.errors.PERMISSION_DENIED} />
  {:else if loading && items.length === 0}
    <div class="space-y-3">
      {#each [1, 2, 3] as idx (idx)}
        <div class="soft-card h-20 animate-pulse bg-base-sub/60"></div>
      {/each}
    </div>
  {:else if displayedItems.length === 0}
    <EmptyState title={d.todos.noTodos}>
      {#snippet action()}
        {#if canCreate}
          <Button variant="primary" onclick={openNewModal}>+ {d.todos.newTask}</Button>
        {/if}
      {/snippet}
    </EmptyState>
  {:else}
    <div class="space-y-2.5" data-testid="todo-list">
      {#each displayedItems as todo, index (todo.id)}
        <TodoItemCard
          {todo}
          {canEdit}
          {canDelete}
          canMoveUp={sessionStore.isAuthenticated && index > 0}
          canMoveDown={sessionStore.isAuthenticated && index < displayedItems.length - 1}
          onToggleComplete={handleToggleComplete}
          onEdit={openEditModal}
          onDelete={handleDelete}
          onMoveUp={(t) => moveTodo(t, -1)}
          onMoveDown={(t) => moveTodo(t, 1)}
        />
      {/each}
    </div>

    {#if sessionStore.isAuthenticated && pagination.total_pages > 1}
      <div class="flex items-center justify-end gap-2 pt-2 text-xs text-base-content-secondary">
        <Button
          variant="outline"
          size="sm"
          disabled={pagination.page <= 1}
          onclick={() => loadTodos(pagination.page - 1)}
        >
          Prev
        </Button>
        <span>{pagination.page} / {pagination.total_pages}</span>
        <Button
          variant="outline"
          size="sm"
          disabled={pagination.page >= pagination.total_pages}
          onclick={() => loadTodos(pagination.page + 1)}
        >
          Next
        </Button>
      </div>
    {/if}
  {/if}
</div>

{#if sessionStore.isAuthenticated && sessionStore.activeWorkspaceId}
  <TodoFormModal
    open={modalOpen}
    workspaceId={sessionStore.activeWorkspaceId}
    todo={editingTodo}
    {members}
    onClose={() => (modalOpen = false)}
    onSaved={() => void loadTodos(pagination.page)}
  />
{/if}
