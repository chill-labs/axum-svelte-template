<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type { UpdateProfileRequest, UserResponse } from '#lib/api/types.generated.ts';
  import Badge from '#lib/components/ui/Badge.svelte';
  import Button from '#lib/components/ui/Button.svelte';
  import Card from '#lib/components/ui/Card.svelte';
  import ImageUploader from '#lib/components/ui/ImageUploader.svelte';
  import Input from '#lib/components/ui/Input.svelte';
  import ProtectedImage from '#lib/components/ui/ProtectedImage.svelte';
  import { ROUTES } from '#lib/config/routes.ts';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { uiPrefs, type Locale } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';

  const d = $derived(uiPrefs.dict);

  let name = $state(sessionStore.user?.name ?? '');
  let language = $state<Locale>(uiPrefs.locale);
  let saving = $state(false);

  $effect(() => {
    if (sessionStore.user) {
      name = sessionStore.user.name;
      language = sessionStore.user.language?.startsWith('en') ? 'en' : 'vi';
    }
  });

  async function handleSaveProfile(event: SubmitEvent) {
    event.preventDefault();
    if (!sessionStore.data) return;

    saving = true;
    try {
      const res = await apiRequest<UserResponse, UpdateProfileRequest>(
        API_ENDPOINTS.PROFILE_UPDATE_PROFILE,
        {
          body: {
            name: name.trim(),
            language
          }
        }
      );
      if (res.success && res.data && sessionStore.data) {
        sessionStore.setSession({
          ...sessionStore.data,
          user: res.data
        });
        uiPrefs.setLocale(language);
        toasts.success(d.todos.updatedSuccess);
      } else {
        toasts.error(uiPrefs.err(res.message));
      }
    } finally {
      saving = false;
    }
  }
</script>

<div class="mx-auto max-w-2xl space-y-5">
  <div>
    <h1 class="text-2xl font-bold tracking-tight text-base-content">{d.profile.pageTitle}</h1>
    <p class="mt-0.5 text-sm text-base-content-secondary">{d.profile.pageSubtitle}</p>
  </div>

  {#if sessionStore.user}
    {@const user = sessionStore.user}
    <Card class="space-y-5">
      <div class="flex flex-wrap items-center justify-between gap-4 border-b border-base-line pb-4">
        <div class="flex items-center gap-3.5">
          <ProtectedImage
            entityType="user"
            entityId={user.id}
            mediaKey={user.avatar_key}
            src={user.avatar_url}
            alt={user.name}
            fallbackText={user.name}
            class="h-14 w-14 rounded-2xl object-cover"
          />
          <div>
            <div class="flex items-center gap-2">
              <h2 class="text-base font-bold text-base-content">{user.name}</h2>
              <Badge tone={user.role === 'admin' ? 'warning' : 'neutral'}>{user.role}</Badge>
            </div>
            <p class="text-xs text-base-content-secondary">{user.email}</p>
          </div>
        </div>

        <ImageUploader
          purpose="user_avatar"
          entityId={user.id}
          label={d.profile.avatarLabel}
          onUploaded={() => void sessionStore.refresh()}
        />
      </div>

      <form onsubmit={handleSaveProfile} class="space-y-4">
        <Input
          id="profile-name"
          data-testid="profile-name-input"
          label={d.auth.displayName}
          bind:value={name}
          required
        />

        <div class="space-y-1.5">
          <label
            for="profile-language"
            class="block text-xs font-semibold tracking-wide text-base-content-secondary"
          >
            {d.common.language}
          </label>
          <select
            id="profile-language"
            bind:value={language}
            class="h-10 w-full cursor-pointer rounded-xl border border-base-line bg-base-sub px-3 text-sm text-base-content focus:border-primary focus:outline-none"
          >
            <option value="vi">Tiếng Việt</option>
            <option value="en">English</option>
          </select>
        </div>

        <div class="flex justify-end">
          <Button
            type="submit"
            variant="primary"
            loading={saving}
            data-testid="profile-save-submit"
          >
            {saving ? d.common.saving : d.common.save}
          </Button>
        </div>
      </form>
    </Card>

    <Card class="space-y-3">
      <h3 class="text-base font-bold text-base-content">{d.profile.securitySection}</h3>
      <p class="text-xs leading-relaxed text-base-content-secondary">
        {d.profile.localPasswordNote}
      </p>
      <div>
        <a href={ROUTES.RESET_PASSWORD}>
          <Button variant="outline" size="sm">{d.auth.reauthTitle}</Button>
        </a>
      </div>
    </Card>
  {/if}
</div>
