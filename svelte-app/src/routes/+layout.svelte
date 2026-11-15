<script lang="ts">
  import '../app.css';
  import { browser } from '$app/env';
  import type { Snippet } from 'svelte';
  import LoginGateModal from '#lib/components/layout/LoginGateModal.svelte';
  import ToastContainer from '#lib/components/ui/ToastContainer.svelte';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { sseClient } from '#lib/state/sse.svelte.ts';

  let { children }: { children: Snippet } = $props();

  $effect(() => {
    if (!browser) return;
    // Track user ID & hot-support flag so SSE reconnects cleanly on login/logout.
    void sessionStore.user?.id;
    void sessionStore.isHotSupport;
    sseClient.disconnect();
    sseClient.connect();
    return () => {
      sseClient.disconnect();
    };
  });
</script>

{@render children()}
<LoginGateModal />
<ToastContainer />
