<script lang="ts">
  import { goto } from '$app/navigation';
  import Button from '#lib/components/ui/Button.svelte';
  import { ROUTES } from '#lib/config/routes.ts';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';

  const d = $derived(uiPrefs.dict);

  $effect(() => {
    if (!sessionStore.isAuthenticated) {
      void goto(ROUTES.LOGIN, { replaceState: true });
    }
  });
</script>

<div class="space-y-4 text-center">
  <div class="mx-auto flex h-12 w-12 items-center justify-center rounded-2xl bg-emerald-500/15 text-emerald-600">
    ✓
  </div>
  <h1 class="text-xl font-bold text-base-content">{d.auth.reauthSuccessTitle}</h1>
  <p class="text-sm text-base-content-secondary">{d.auth.reauthSuccessBody}</p>
  <div class="flex justify-center gap-2 pt-2">
    <a href={ROUTES.PROFILE}>
      <Button variant="primary">{d.nav.profile}</Button>
    </a>
    <a href={ROUTES.TODOS}>
      <Button variant="outline">{d.nav.todos}</Button>
    </a>
  </div>
</div>
