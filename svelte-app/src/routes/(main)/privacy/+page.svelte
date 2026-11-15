<script lang="ts">
  import Button from '#lib/components/ui/Button.svelte';
  import Card from '#lib/components/ui/Card.svelte';
  import { ROUTES } from '#lib/config/routes.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';

  const d = $derived(uiPrefs.dict);
</script>

<div class="mx-auto max-w-2xl space-y-5">
  <h1 class="text-2xl font-bold tracking-tight text-base-content">Privacy Policy</h1>
  <Card class="space-y-3 text-sm leading-relaxed text-base-content-secondary">
    <p>
      Authentication sessions use SHA-256 hashed HttpOnly cookies stored in PostgreSQL and an
      in-memory Moka cache. Uploaded files can optionally use client-side XOR obfuscation with R2
      storage.
    </p>
    <p>
      Replace this notice with your organization's data processing and privacy disclosures before
      deploying to users.
    </p>
  </Card>
  <a href={ROUTES.TODOS}>
    <Button variant="outline" size="sm">{d.nav.todos}</Button>
  </a>
</div>
