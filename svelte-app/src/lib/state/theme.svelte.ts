// =============================================================================
// REACTIVE UI PREFERENCE STORE (THEME + LANGUAGE)
// =============================================================================

import { browser } from '$app/env';
import { getDictionary, normalizeLocale, translateError, type Dictionary, type Locale } from '#lib/i18n/index.ts';

export type { Locale };
export type ThemeMode = 'light' | 'dark';

function detectInitialLocale(): Locale {
  if (browser && window.__APP_BOOTSTRAP__?.lang) {
    return normalizeLocale(window.__APP_BOOTSTRAP__.lang);
  }
  if (browser && document.documentElement.lang) {
    return normalizeLocale(document.documentElement.lang);
  }
  return 'vi';
}

function detectInitialTheme(): ThemeMode {
  if (browser) {
    if (window.__APP_BOOTSTRAP__?.theme === 'dark') return 'dark';
    if (window.__APP_BOOTSTRAP__?.theme === 'light') return 'light';
    if (document.documentElement.classList.contains('dark')) return 'dark';
  }
  return 'light';
}

class UiPreferencesStore {
  locale = $state<Locale>(detectInitialLocale());
  theme = $state<ThemeMode>(detectInitialTheme());

  dict = $derived<Dictionary>(getDictionary(this.locale));

  setLocale(next: Locale) {
    this.locale = next;
    if (browser) {
      document.documentElement.lang = next;
      document.cookie = `lang=${next}; Path=/; Max-Age=31536000; SameSite=Lax`;
    }
  }

  toggleLocale() {
    this.setLocale(this.locale === 'vi' ? 'en' : 'vi');
  }

  setTheme(next: ThemeMode) {
    this.theme = next;
    if (browser) {
      document.documentElement.classList.toggle('dark', next === 'dark');
      document.cookie = `theme=${next}; Path=/; Max-Age=31536000; SameSite=Lax`;
    }
  }

  toggleTheme() {
    this.setTheme(this.theme === 'dark' ? 'light' : 'dark');
  }

  err(code?: string | null): string {
    return translateError(this.locale, code);
  }
}

export const uiPrefs = new UiPreferencesStore();
