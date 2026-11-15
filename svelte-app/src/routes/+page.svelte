<script lang="ts">
  import { goto } from '$app/navigation';
  import { ROUTES } from '#lib/config/routes.ts';

  $effect(() => {
    void goto(ROUTES.TODOS, { replaceState: true });
  });
</script>

<div class="flex min-h-screen items-center justify-center bg-base-main">
  <div class="h-8 w-8 animate-spin rounded-full border-3 border-primary border-t-transparent"></div>
</div>
