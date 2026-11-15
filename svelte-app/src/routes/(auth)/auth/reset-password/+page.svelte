<script lang="ts">
  import { goto } from '$app/navigation';
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type {
    InitiatePasswordReauthRequest,
    ReauthRedirectResponse
  } from '#lib/api/types.generated.ts';
  import Button from '#lib/components/ui/Button.svelte';
  import Input from '#lib/components/ui/Input.svelte';
  import { ROUTES } from '#lib/config/routes.ts';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  const d = $derived(uiPrefs.dict);

  let newPassword = $state('');
  let submitting = $state(false);
  let errorMsg = $state('');

  $effect(() => {
    if (!sessionStore.isAuthenticated) {
      void goto(ROUTES.LOGIN, { replaceState: true });
    }
  });

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    errorMsg = '';
    submitting = true;
    try {
      const res = await apiRequest<ReauthRedirectResponse, InitiatePasswordReauthRequest>(
        API_ENDPOINTS.AUTH_REAUTH_PASSWORD,
        {
          body: { new_password: newPassword }
        }
      );
      if (res.success && res.data?.redirect_url) {
        window.location.href = res.data.redirect_url;
      } else {
        errorMsg = uiPrefs.err(res.message);
        toasts.error(errorMsg);
      }
    } finally {
      submitting = false;
    }
  }
</script>

<div class="space-y-5">
  <div>
    <h1 class="text-xl font-bold text-base-content">{d.auth.reauthTitle}</h1>
    <p class="mt-1 text-sm text-base-content-secondary">{d.auth.reauthSubtitle}</p>
  </div>

  {#if errorMsg}
    <div
      class="rounded-xl border border-rose-500/35 bg-rose-500/10 px-3.5 py-2.5 text-xs font-medium text-rose-700 dark:text-rose-300"
    >
      {errorMsg}
    </div>
  {/if}

  <form onsubmit={handleSubmit} class="space-y-4">
    <Input
      id="reauth-new-password"
      type="password"
      label={d.auth.newPassword}
      bind:value={newPassword}
      minlength={8}
      required
    />

    <Button type="submit" variant="primary" size="lg" loading={submitting} class="w-full">
      {d.auth.verifyWithGoogle}
    </Button>
  </form>

  <div class="text-center">
    <a href={ROUTES.PROFILE} class="text-xs font-semibold text-primary hover:underline">
      {d.common.cancel}
    </a>
  </div>
</div>
