import { expect, test } from '@playwright/test';
import { attachConsoleGuards } from '../helpers/auth';

test.describe('Guest Experience, Healthcheck & Preferences', () => {
  test('redirects root / to /todos, opens LoginGateModal, persists dark mode and language', async ({
    page
  }) => {
    const verifyClean = attachConsoleGuards(page);

    await page.goto('/');
    await expect(page).toHaveURL(/\/todos$/);
    await expect(page.getByTestId('guest-demo-banner')).toBeVisible();

    // Guest clicking "+ New Task" opens LoginGateModal with Google + Test Account options
    await page.getByTestId('new-todo-button').click();
    const dialog = page.locator('dialog[open]');
    await expect(dialog).toBeVisible();
    await expect(dialog.locator('a[href*="/api/v1/auth/google/login"]')).toBeVisible();
    await expect(dialog.locator('a[href="/auth/login-password"]')).toBeVisible();
    await dialog.getByRole('button', { name: 'Close' }).click();
    await expect(dialog).toBeHidden();

    // Theme toggle persists across reload (SSR shell injects class="dark")
    await page.getByTestId('theme-toggle').click();
    await expect(page.locator('html')).toHaveClass(/dark/);
    await page.reload();
    await expect(page.locator('html')).toHaveClass(/dark/);

    // Switch back to light so default state is verified too
    await page.getByTestId('theme-toggle').click();
    await expect(page.locator('html')).not.toHaveClass(/dark/);

    // Language toggle persists across reload
    const langBtn = page.getByTestId('lang-toggle');
    const initialLang = (await langBtn.innerText()).trim();
    await langBtn.click();
    const expectedNext = initialLang === 'VI' ? 'EN' : 'VI';
    await expect(langBtn).toHaveText(expectedNext);
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('lang', expectedNext.toLowerCase());

    // Page guide modal opens cleanly on /todos
    await page.locator('[data-open-page-guide]').click();
    const guideDialog = page.locator('#app-page-guide-dialog[open]');
    await expect(guideDialog).toBeVisible();
    await page.locator('#app-page-guide-dialog button').first().click();

    // Healthcheck reports connected Postgres and optional disabled R2 storage
    await page.goto('/healthcheck');
    const healthCard = page.getByTestId('healthcheck-card');
    await expect(healthCard).toBeVisible();
    await expect(healthCard).toContainText('ok');
    await expect(healthCard).toContainText('connected');
    await expect(healthCard).toContainText('disabled (optional)');

    verifyClean();
  });
});
