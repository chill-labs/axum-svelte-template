<script lang="ts">
  import { browser } from '$app/env';
  import { page } from '$app/state';
  import { ROUTES } from '#lib/config/routes.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';

  const d = $derived(uiPrefs.dict);

  const errorCode = $derived(page.url.searchParams.get('error'));
  const googleLoginHref = $derived.by(() => {
    const returnTo = browser ? localStorage.getItem('return_to') || ROUTES.TODOS : ROUTES.TODOS;
    return `/api/v1/auth/google/login?return_to=${encodeURIComponent(returnTo)}`;
  });
</script>

<div class="space-y-6">
  <div class="space-y-1.5 text-center">
    <h1 class="text-2xl font-bold tracking-tight text-base-content">{d.auth.welcomeTitle}</h1>
    <p class="text-sm text-base-content-secondary">{d.auth.welcomeSubtitle}</p>
  </div>

  {#if errorCode}
    <div
      data-testid="oauth-error-banner"
      class="rounded-2xl border border-rose-500/35 bg-rose-500/10 px-4 py-3 text-xs font-medium text-rose-700 dark:text-rose-300"
    >
      {uiPrefs.err(errorCode)}
    </div>
  {/if}

  <div class="space-y-3">
    <a
      href={googleLoginHref}
      data-testid="google-login-button"
      class="cta-gradient-primary inline-flex h-12 w-full items-center justify-center gap-2.5 rounded-2xl px-4 text-sm font-semibold text-white transition"
    >
      <svg class="h-4.5 w-4.5" viewBox="0 0 24 24" aria-hidden="true">
        <path
          fill="currentColor"
          d="M21.35 11.1h-9.17v2.73h6.51c-.33 3.81-3.5 5.44-6.5 5.44C8.36 19.27 5 16.25 5 12c0-4.1 3.2-7.27 7.2-7.27c3.09 0 4.9 1.97 4.9 1.97L19 4.72S16.56 2 12.1 2C6.42 2 2.03 6.8 2.03 12c0 5.05 4.13 10 10.22 10c5.35 0 9.25-3.67 9.25-9.09c0-1.15-.15-1.81-.15-1.81Z"
        />
      </svg>
      <span>{d.auth.continueWithGoogle}</span>
    </a>

    <div class="relative flex items-center py-2">
      <div class="grow border-t border-base-line"></div>
      <span class="mx-3 text-[11px] font-medium text-base-content-muted uppercase">or</span>
      <div class="grow border-t border-base-line"></div>
    </div>

    <a
      href={ROUTES.LOGIN_PASSWORD}
      data-testid="test-login-link"
      class="inline-flex h-11 w-full items-center justify-center rounded-2xl border border-base-line bg-base-sub px-4 text-sm font-semibold text-base-content transition hover:bg-base-elevated"
    >
      {d.auth.testAccountLoginLink}
    </a>
  </div>
</div>
