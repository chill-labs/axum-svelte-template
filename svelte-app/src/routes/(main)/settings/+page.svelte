<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import Button from '#lib/components/ui/Button.svelte';
  import Card from '#lib/components/ui/Card.svelte';
  import { sessionStore } from '#lib/state/session.svelte.ts';
  import { uiPrefs, type Locale, type ThemeMode } from '#lib/state/theme.svelte.ts';

  const d = $derived(uiPrefs.dict);

  function selectTheme(theme: ThemeMode) {
    uiPrefs.setTheme(theme);
  }

  async function selectLanguage(language: Locale) {
    uiPrefs.setLocale(language);
    if (sessionStore.isAuthenticated) {
      await apiRequest(API_ENDPOINTS.PROFILE_UPDATE_LANGUAGE, {
        body: { language }
      });
    }
  }
</script>

<div class="mx-auto max-w-2xl space-y-5">
  <div>
    <h1 class="text-2xl font-bold tracking-tight text-base-content">{d.settings.pageTitle}</h1>
    <p class="mt-0.5 text-sm text-base-content-secondary">{d.settings.pageSubtitle}</p>
  </div>

  <Card class="space-y-3">
    <h2 class="text-base font-bold text-base-content">{d.settings.appearanceSection}</h2>
    <div class="flex flex-wrap gap-2">
      <Button
        variant={uiPrefs.theme === 'light' ? 'primary' : 'outline'}
        onclick={() => selectTheme('light')}
      >
        ☀ {d.common.lightMode}
      </Button>
      <Button
        variant={uiPrefs.theme === 'dark' ? 'primary' : 'outline'}
        onclick={() => selectTheme('dark')}
      >
        ☾ {d.common.darkMode}
      </Button>
    </div>
  </Card>

  <Card class="space-y-3">
    <h2 class="text-base font-bold text-base-content">{d.settings.languageSection}</h2>
    <div class="flex flex-wrap gap-2">
      <Button
        variant={uiPrefs.locale === 'vi' ? 'primary' : 'outline'}
        onclick={() => selectLanguage('vi')}
      >
        Tiếng Việt
      </Button>
      <Button
        variant={uiPrefs.locale === 'en' ? 'primary' : 'outline'}
        onclick={() => selectLanguage('en')}
      >
        English
      </Button>
    </div>
  </Card>

  <Card class="flex items-center justify-between gap-4">
    <div>
      <h2 class="text-base font-bold text-base-content">{d.settings.sessionSection}</h2>
      <p class="text-xs text-base-content-secondary">
        {sessionStore.user?.email ?? ''} • {sessionStore.activeWorkspaceName ?? ''}
      </p>
    </div>
    <Button variant="danger" size="sm" onclick={() => sessionStore.logout()}>
      {d.nav.signOut}
    </Button>
  </Card>
</div>
