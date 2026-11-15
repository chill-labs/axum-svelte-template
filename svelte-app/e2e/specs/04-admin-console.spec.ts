import { expect, test } from '@playwright/test';
import { attachConsoleGuards, seedBrowserSession, seedUserOnly } from '../helpers/auth';

test.describe('Admin Console (Users, Test Accounts, Global Announcements, Hot Support)', () => {
  test('exercises all 4 admin tabs including isolated Hot Support iframe', async ({
    page,
    baseURL
  }) => {
    const verifyClean = attachConsoleGuards(page);
    const stamp = `${Date.now()}-${Math.floor(Math.random() * 10000)}`;

    const targetUser = await seedUserOnly(baseURL!, {
      email: `hs-target-${stamp}@example.com`,
      name: `HotSupport Target ${stamp}`,
      language: 'en'
    });

    await seedBrowserSession(page, {
      email: `e2e-admin-${stamp}@example.com`,
      name: 'E2E System Admin',
      role: 'admin',
      language: 'en'
    });

    await page.goto('/admin');

    // Tab 1: Search users and verify directory row
    await page.getByTestId('admin-users-search').fill(targetUser.email);
    await page.getByTestId('admin-users-search').press('Enter');
    await expect(page.getByTestId(`admin-user-row-${targetUser.id}`)).toBeVisible();

    // Tab 3: Create and delete a test account
    await page.getByTestId('admin-tab-test_accounts').click();
    const createdAccEmail = `created-test-${stamp}@example.com`;
    await page.getByTestId('test-account-email').fill(createdAccEmail);
    await page.getByTestId('test-account-name').fill('Created Via Admin');
    await page.getByTestId('test-account-password').fill('AdminCreated123!');
    await page.getByTestId('create-test-account-submit').click();

    const createdRow = page.locator('tr', { hasText: createdAccEmail });
    await expect(createdRow).toBeVisible();
    await createdRow.locator('button', { hasText: 'Delete' }).click();
    await expect(page.locator('tr', { hasText: createdAccEmail })).toHaveCount(0);

    // Tab 4: Publish active HTML announcement and inspect via NotificationDropdown
    await page.getByTestId('admin-tab-notifications').click();
    const noticeTitle = `E2E Notice ${stamp}`;
    await page.getByTestId('announcement-title-input').fill(noticeTitle);
    await page.getByTestId('announcement-audience-select').selectOption('all');
    await page
      .getByTestId('announcement-html-input')
      .fill(`<p><strong>Release ${stamp}</strong> ready.</p>`);
    await page.getByTestId('publish-announcement-submit').click();
    await expect(page.getByRole('heading', { name: noticeTitle })).toBeVisible();

    await page.getByTestId('notification-bell').click();
    await expect(page.getByTestId('notification-panel')).toContainText(noticeTitle);
    await expect(page.getByTestId('notification-panel')).toContainText(`Release ${stamp}`);
    await page.getByTestId('notification-bell').click();

    // Tab 1 -> Tab 2: Start Hot Support impersonation in isolated iframe
    await page.getByTestId('admin-tab-users').click();
    await page.getByTestId('admin-users-search').fill(targetUser.email);
    await page.getByTestId('admin-users-search').press('Enter');
    await page.getByTestId(`impersonate-user-${targetUser.id}`).click();

    await expect(page).toHaveURL(/\/admin$/);
    const iframe = page.getByTestId('hot-support-iframe');
    await expect(iframe).toBeVisible();

    const frame = page.frameLocator('[data-testid="hot-support-iframe"]');
    await expect(frame.getByTestId('hot-support-banner')).toContainText(targetUser.email);
    await expect(frame.getByTestId('todo-list').locator('article')).toHaveCount(5);

    // Close Hot Support session cleanly
    await page.getByTestId('admin-close-hot-support').click();
    await expect(page.getByTestId('hot-support-iframe')).toHaveCount(0);

    verifyClean();
  });
});
