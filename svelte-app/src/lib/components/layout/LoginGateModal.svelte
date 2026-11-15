<script lang="ts">
  import { browser } from '$app/env';
  import Modal from '#lib/components/ui/Modal.svelte';
  import { ROUTES } from '#lib/config/routes.ts';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';

  const d = $derived(uiPrefs.dict);

  const googleLoginHref = $derived.by(() => {
    const returnTo = browser ? window.location.pathname + window.location.search : ROUTES.TODOS;
    return `/api/v1/auth/google/login?return_to=${encodeURIComponent(returnTo)}`;
  });
</script>

<Modal
  open={sessionStore.loginGateOpen}
  title={d.auth.loginGateTitle}
  maxWidthClass="max-w-md"
  onClose={() => sessionStore.closeLoginGate()}
>
  <div class="space-y-4">
    <p class="text-sm leading-relaxed text-base-content-secondary">
      {d.auth.loginGateBody}
    </p>

    <div class="flex flex-col gap-2.5 pt-1">
      <a
        href={googleLoginHref}
        class="cta-gradient-primary inline-flex h-11 items-center justify-center gap-2.5 rounded-2xl px-4 text-sm font-semibold text-white transition"
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" aria-hidden="true">
          <path
            fill="currentColor"
            d="M21.35 11.1h-9.17v2.73h6.51c-.33 3.81-3.5 5.44-6.5 5.44C8.36 19.27 5 16.25 5 12c0-4.1 3.2-7.27 7.2-7.27c3.09 0 4.9 1.97 4.9 1.97L19 4.72S16.56 2 12.1 2C6.42 2 2.03 6.8 2.03 12c0 5.05 4.13 10 10.22 10c5.35 0 9.25-3.67 9.25-9.09c0-1.15-.15-1.81-.15-1.81Z"
          />
        </svg>
        <span>{d.auth.continueWithGoogle}</span>
      </a>

      <a
        href={ROUTES.LOGIN_PASSWORD}
        onclick={() => sessionStore.closeLoginGate()}
        class="inline-flex h-10 items-center justify-center rounded-xl border border-base-line bg-base-elevated px-4 text-sm font-medium text-base-content transition hover:bg-base-elevated-hover"
      >
        {d.auth.testAccountLoginLink}
      </a>
    </div>
  </div>
</Modal>
