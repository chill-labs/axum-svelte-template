<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type { HealthcheckResponse } from '#lib/api/types.generated.ts';
  import Badge from '#lib/components/ui/Badge.svelte';
  import Button from '#lib/components/ui/Button.svelte';
  import Card from '#lib/components/ui/Card.svelte';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';

  const d = $derived(uiPrefs.dict);

  let health = $state<HealthcheckResponse | null>(null);
  let loading = $state(false);

  async function loadHealth() {
    loading = true;
    try {
      const res = await apiRequest<HealthcheckResponse>(API_ENDPOINTS.HEALTH_CHECK_HEALTH);
      if (res.success && res.data) {
        health = res.data;
      }
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void loadHealth();
  });
</script>

<div class="mx-auto max-w-2xl space-y-5">
  <div class="flex items-center justify-between gap-3">
    <h1 class="text-2xl font-bold tracking-tight text-base-content">System Healthcheck</h1>
    <Button variant="outline" size="sm" {loading} onclick={loadHealth}>
      {d.common.refresh}
    </Button>
  </div>

  {#if health}
    <Card class="space-y-3" data-testid="healthcheck-card">
      <div class="flex items-center justify-between">
        <span class="text-sm font-medium text-base-content-secondary">Status</span>
        <Badge tone={health.status === 'ok' ? 'success' : 'danger'}>{health.status}</Badge>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm font-medium text-base-content-secondary">PostgreSQL</span>
        <Badge tone={health.database ? 'success' : 'danger'}>
          {health.database ? 'connected' : 'down'}
        </Badge>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm font-medium text-base-content-secondary">Object Storage (R2)</span>
        <Badge tone={health.storage_enabled ? 'success' : 'neutral'}>
          {health.storage_enabled ? 'enabled' : 'disabled (optional)'}
        </Badge>
      </div>
      <div class="flex items-center justify-between text-xs text-base-content-secondary">
        <span>Environment</span>
        <span class="font-mono text-base-content">{health.environment}</span>
      </div>
      <div class="flex items-center justify-between text-xs text-base-content-secondary">
        <span>Version</span>
        <span class="font-mono text-base-content">{health.version}</span>
      </div>
      <div class="flex items-center justify-between text-xs text-base-content-secondary">
        <span>Timestamp</span>
        <span class="font-mono text-base-content">{health.timestamp}</span>
      </div>
    </Card>
  {:else if loading}
    <Card>
      <p class="py-8 text-center text-xs text-base-content-muted">{d.common.loading}</p>
    </Card>
  {/if}
</div>
