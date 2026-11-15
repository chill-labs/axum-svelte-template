<script lang="ts">
  import Button from '#lib/components/ui/Button.svelte';
  import Card from '#lib/components/ui/Card.svelte';
  import { ROUTES } from '#lib/config/routes.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';

  const d = $derived(uiPrefs.dict);
</script>

<div class="mx-auto max-w-2xl space-y-5">
  <h1 class="text-2xl font-bold tracking-tight text-base-content">Terms of Service</h1>
  <Card class="space-y-3 text-sm leading-relaxed text-base-content-secondary">
    <p>
      This open-source template demonstrates a multi-workspace Todo platform powered by Rust Axum,
      SeaORM, PostgreSQL, and Svelte 5 Runes.
    </p>
    <p>
      Replace this template notice with your production product's legal terms before launching.
    </p>
  </Card>
  <a href={ROUTES.TODOS}>
    <Button variant="outline" size="sm">{d.nav.todos}</Button>
  </a>
</div>
