<script lang="ts">
  import { browser } from '$app/env';
  import { goto } from '$app/navigation';
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type { LoginPasswordRequest, SessionBootstrapData } from '#lib/api/types.generated.ts';
  import Button from '#lib/components/ui/Button.svelte';
  import Input from '#lib/components/ui/Input.svelte';
  import { ROUTES } from '#lib/config/routes.ts';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  const d = $derived(uiPrefs.dict);

  let email = $state('');
  let password = $state('');
  let submitting = $state(false);
  let errorMsg = $state('');

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    errorMsg = '';
    submitting = true;
    try {
      const res = await apiRequest<SessionBootstrapData, LoginPasswordRequest>(
        API_ENDPOINTS.AUTH_LOGIN_PASSWORD,
        {
          body: { email: email.trim(), password }
        }
      );
      if (res.success && res.data) {
        sessionStore.setSession(res.data);
        uiPrefs.setLocale(res.data.user.language?.startsWith('en') ? 'en' : 'vi');
        const returnTo = browser ? localStorage.getItem('return_to') : null;
        if (browser) localStorage.removeItem('return_to');
        const nextPath = !res.data.user.onboarding_completed
          ? ROUTES.ONBOARDING
          : returnTo && returnTo.startsWith('/') && !returnTo.startsWith('/auth/')
            ? returnTo
            : ROUTES.TODOS;
        await goto(nextPath);
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
    <h1 class="text-xl font-bold text-base-content">{d.auth.testAccountTitle}</h1>
    <p class="mt-1 text-sm text-base-content-secondary">{d.auth.testAccountSubtitle}</p>
  </div>

  {#if errorMsg}
    <div
      data-testid="login-error-banner"
      class="rounded-xl border border-rose-500/35 bg-rose-500/10 px-3.5 py-2.5 text-xs font-medium text-rose-700 dark:text-rose-300"
    >
      {errorMsg}
    </div>
  {/if}

  <form onsubmit={handleSubmit} class="space-y-4">
    <Input
      id="login-email"
      data-testid="login-email-input"
      type="email"
      label={d.auth.email}
      bind:value={email}
      autocomplete="email"
      required
    />

    <Input
      id="login-password"
      data-testid="login-password-input"
      type="password"
      label={d.auth.password}
      bind:value={password}
      autocomplete="current-password"
      required
    />

    <Button
      type="submit"
      variant="primary"
      size="lg"
      loading={submitting}
      data-testid="login-password-submit"
      class="w-full"
    >
      {submitting ? d.common.loading : d.auth.signInButton}
    </Button>
  </form>

  <div class="text-center">
    <a href={ROUTES.LOGIN} class="text-xs font-semibold text-primary hover:underline">
      {d.auth.backToGoogleLogin}
    </a>
  </div>
</div>
