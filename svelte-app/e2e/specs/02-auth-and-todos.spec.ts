import { expect, test } from '@playwright/test';
import { attachConsoleGuards, seedUserOnly } from '../helpers/auth';

test.describe('Test Account Login & Todo CRUD + Double-Option Clear', () => {
  test('logs in via UI, receives trigger-seeded starter todos, creates/updates/clears/deletes todo', async ({
    page,
    baseURL
  }) => {
    const verifyClean = attachConsoleGuards(page);
    const stamp = `${Date.now()}-${Math.floor(Math.random() * 10000)}`;
    const email = `e2e-todo-${stamp}@example.com`;
    const password = 'TodoPassword123!';

    await seedUserOnly(baseURL!, {
      email,
      name: 'E2E Todo Tester',
      language: 'en',
      password
    });

    // /auth/register redirects to /auth/login
    await page.goto('/auth/register');
    await expect(page).toHaveURL(/\/auth\/login$/);

    // Navigate to password login and sign in
    await page.getByTestId('test-login-link').click();
    await expect(page).toHaveURL(/\/auth\/login-password$/);
    await page.getByTestId('login-email-input').fill(email);
    await page.getByTestId('login-password-input').fill(password);
    await page.getByTestId('login-password-submit').click();

    await expect(page).toHaveURL(/\/todos$/);
    await expect(page.getByTestId('todo-stats-bar')).toContainText('5');
    await expect(page.getByTestId('todo-list').locator('article')).toHaveCount(5);

    // Create a new urgent Todo with description and due date via modal
    await page.getByTestId('new-todo-button').click();
    await page.getByTestId('todo-form-title').fill('Verify Double Option Null Clear');
    await page.getByTestId('todo-form-description').fill('Should be cleared cleanly later');
    await page.getByTestId('todo-form-priority').selectOption('urgent');
    await page.getByTestId('todo-form-due-date').fill('2026-12-31');
    await page.getByTestId('todo-form-submit').click();

    const createdCard = page.locator('article', { hasText: 'Verify Double Option Null Clear' });
    await expect(createdCard).toBeVisible();
    await expect(createdCard).toContainText('2026-12-31');
    await expect(createdCard).toContainText('Should be cleared cleanly later');

    // Filter by urgent priority
    await page.getByTestId('todo-priority-filter').selectOption('urgent');
    await expect(createdCard).toBeVisible();
    await page.getByTestId('todo-priority-filter').selectOption('all');

    // Toggle completion status
    const createdTestId = await createdCard.getAttribute('data-testid');
    const todoId = createdTestId!.replace('todo-item-', '');
    await page.getByTestId(`todo-toggle-${todoId}`).click();
    await expect(createdCard).toContainText('Done');

    // Edit Todo and clear description + due_date -> persisted as NULL
    await page.getByTestId(`todo-edit-${todoId}`).click();
    await page.getByTestId('todo-form-description').fill('');
    await page.getByTestId('todo-clear-due-date').click();
    await page.getByTestId('todo-form-submit').click();

    await page.reload();
    const reloadedCard = page.getByTestId(`todo-item-${todoId}`);
    await expect(reloadedCard).toBeVisible();
    await expect(reloadedCard).not.toContainText('2026-12-31');
    await expect(reloadedCard).not.toContainText('Should be cleared cleanly later');

    // Delete created Todo
    await page.getByTestId(`todo-delete-${todoId}`).click();
    await expect(page.getByTestId(`todo-item-${todoId}`)).toHaveCount(0);

    verifyClean();
  });
});
