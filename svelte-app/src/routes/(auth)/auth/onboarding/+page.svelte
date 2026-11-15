<script lang="ts">
  import { goto } from '$app/navigation';
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type { CompleteOnboardingRequest, SessionBootstrapData } from '#lib/api/types.generated.ts';
  import Button from '#lib/components/ui/Button.svelte';
  import Input from '#lib/components/ui/Input.svelte';
  import { ROUTES } from '#lib/config/routes.ts';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { uiPrefs, type Locale } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  const d = $derived(uiPrefs.dict);

  let name = $state(sessionStore.user?.name ?? '');
  let language = $state<Locale>(uiPrefs.locale);
  let firstWorkspaceName = $state('');
  let saving = $state(false);

  $effect(() => {
    if (!sessionStore.isAuthenticated) {
      void goto(ROUTES.LOGIN, { replaceState: true });
      return;
    }
    if (sessionStore.user?.onboarding_completed) {
      void goto(ROUTES.TODOS, { replaceState: true });
    }
  });

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    try {
      const res = await apiRequest<SessionBootstrapData, CompleteOnboardingRequest>(
        API_ENDPOINTS.AUTH_COMPLETE_ONBOARDING,
        {
          body: {
            name: name.trim(),
            language,
            first_workspace_name: firstWorkspaceName.trim() || null
          }
        }
      );
      if (res.success && res.data) {
        sessionStore.setSession(res.data);
        uiPrefs.setLocale(language);
        await goto(ROUTES.TODOS, { replaceState: true });
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      saving = false;
    }
  }
</script>

<div class="space-y-5">
  <div>
    <h1 class="text-xl font-bold text-base-content">{d.auth.onboardingTitle}</h1>
    <p class="mt-1 text-sm text-base-content-secondary">{d.auth.onboardingSubtitle}</p>
  </div>

  <form onsubmit={handleSubmit} class="space-y-4">
    <Input
      id="onboard-name"
      data-testid="onboarding-name-input"
      label={d.auth.displayName}
      bind:value={name}
      required
    />

    <div class="space-y-1.5">
      <label
        for="onboard-language"
        class="block text-xs font-semibold tracking-wide text-base-content-secondary"
      >
        {d.common.language}
      </label>
      <select
        id="onboard-language"
        bind:value={language}
        class="h-10 w-full cursor-pointer rounded-xl border border-base-line bg-base-sub px-3 text-sm text-base-content focus:border-primary focus:outline-none"
      >
        <option value="vi">Tiếng Việt</option>
        <option value="en">English</option>
      </select>
    </div>

    <Input
      id="onboard-workspace"
      data-testid="onboarding-workspace-input"
      label={d.auth.firstWorkspaceName}
      bind:value={firstWorkspaceName}
      placeholder="Personal Workspace"
    />

    <Button
      type="submit"
      variant="primary"
      size="lg"
      loading={saving}
      data-testid="onboarding-submit"
      class="w-full"
    >
      {d.auth.completeOnboarding}
    </Button>
  </form>
</div>
