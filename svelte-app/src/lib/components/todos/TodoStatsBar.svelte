<script lang="ts">
  import type { TodoSummaryStats } from '#lib/api/types.generated.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';

  interface Props {
    stats: TodoSummaryStats;
  }

  let { stats }: Props = $props();

  const d = $derived(uiPrefs.dict);
</script>

<div class="grid grid-cols-2 gap-3 sm:grid-cols-5" data-testid="todo-stats-bar">
  <div class="soft-card p-3.5">
    <p class="text-xs font-medium text-base-content-secondary">{d.todos.statsTotal}</p>
    <p class="mt-1 text-2xl font-bold text-base-content">{stats.total}</p>
  </div>
  <div class="soft-card p-3.5">
    <p class="text-xs font-medium text-base-content-secondary">{d.todos.statsTodo}</p>
    <p class="mt-1 text-2xl font-bold text-slate-600 dark:text-slate-300">{stats.todo_count}</p>
  </div>
  <div class="soft-card p-3.5">
    <p class="text-xs font-medium text-base-content-secondary">{d.todos.statsInProgress}</p>
    <p class="mt-1 text-2xl font-bold text-sky-600 dark:text-sky-400">
      {stats.in_progress_count}
    </p>
  </div>
  <div class="soft-card p-3.5">
    <p class="text-xs font-medium text-base-content-secondary">{d.todos.statsDone}</p>
    <p class="mt-1 text-2xl font-bold text-emerald-600 dark:text-emerald-400">{stats.done_count}</p>
  </div>
  <div class="soft-card col-span-2 p-3.5 sm:col-span-1">
    <p class="text-xs font-medium text-base-content-secondary">{d.todos.statsUrgent}</p>
    <p class="mt-1 text-2xl font-bold text-rose-600 dark:text-rose-400">{stats.urgent_count}</p>
  </div>
</div>
