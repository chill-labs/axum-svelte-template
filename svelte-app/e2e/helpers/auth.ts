import { request, type APIRequestContext, type Page } from '@playwright/test';

export interface SeedUserOptions {
  email: string;
  name?: string;
  role?: 'user' | 'admin';
  language?: 'vi' | 'en';
  onboarding_completed?: boolean;
  is_test_account?: boolean;
  password?: string;
}

export interface SeededUser {
  id: number;
  email: string;
  name: string;
  role: 'user' | 'admin';
  active_workspace_id?: number | null;
}

/**
 * Seeds a user + PostgreSQL session in a standalone API context (does NOT set cookies on `page`),
 * ideal for creating test users before logging in through the UI.
 */
export async function seedUserOnly(baseURL: string, opts: SeedUserOptions): Promise<SeededUser> {
  const ctx: APIRequestContext = await request.newContext({ baseURL });
  try {
    const res = await ctx.post('/api/test/session', { data: opts });
    if (!res.ok()) {
      throw new Error(`Failed /api/test/session (${res.status()}): ${await res.text()}`);
    }
    const json = (await res.json()) as { success: boolean; data: SeededUser };
    return json.data;
  } finally {
    await ctx.dispose();
  }
}

/**
 * Seeds a user directly into the Playwright `page` context so `auth-session` cookie is active immediately.
 */
export async function seedBrowserSession(page: Page, opts: SeedUserOptions): Promise<SeededUser> {
  const res = await page.request.post('/api/test/session', { data: opts });
  if (!res.ok()) {
    throw new Error(`Failed browser /api/test/session (${res.status()}): ${await res.text()}`);
  }
  const json = (await res.json()) as { success: boolean; data: SeededUser };
  return json.data;
}

export function attachConsoleGuards(page: Page): () => void {
  const errors: string[] = [];
  const onPageError = (err: Error) => errors.push(`pageerror: ${err.message}`);
  const onConsole = (msg: { type: () => string; text: () => string }) => {
    if (msg.type() === 'error') {
      errors.push(`console.error: ${msg.text()}`);
    }
  };
  page.on('pageerror', onPageError);
  page.on('console', onConsole);
  return () => {
    page.off('pageerror', onPageError);
    page.off('console', onConsole);
    if (errors.length > 0) {
      throw new Error(`Browser errors detected:\n${errors.join('\n')}`);
    }
  };
}
