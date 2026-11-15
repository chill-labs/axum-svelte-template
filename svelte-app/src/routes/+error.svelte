<script lang="ts">
  import { page } from '$app/state';
  import Button from '#lib/components/ui/Button.svelte';
  import Card from '#lib/components/ui/Card.svelte';
  import { ROUTES } from '#lib/config/routes.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';

  const d = $derived(uiPrefs.dict);
</script>

<div class="flex min-h-screen items-center justify-center bg-base-main px-4">
  <Card class="max-w-md space-y-4 text-center">
    <p class="font-mono text-4xl font-extrabold text-primary">{page.status}</p>
    <h1 class="text-lg font-bold text-base-content">
      {page.error?.message || d.errors.UNKNOWN_ERROR}
    </h1>
    <div class="pt-2">
      <a href={ROUTES.TODOS}>
        <Button variant="primary">{d.nav.todos}</Button>
      </a>
    </div>
  </Card>
</div>
