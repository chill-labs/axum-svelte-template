<script lang="ts">
  import { goto } from '$app/navigation';
  import { ROUTES } from '#lib/config/routes.ts';

  $effect(() => {
    void goto(ROUTES.LOGIN, { replaceState: true });
  });
</script>

<div class="py-8 text-center text-sm text-base-content-secondary">Redirecting...</div>
