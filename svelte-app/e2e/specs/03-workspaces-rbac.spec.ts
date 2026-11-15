import { expect, test } from '@playwright/test';
import { attachConsoleGuards, seedBrowserSession, seedUserOnly } from '../helpers/auth';

test.describe('Multi-Workspace Switching & Collaborator RBAC', () => {
  test('creates workspace, invites viewer, upgrades to editor, and deletes secondary workspace', async ({
    page,
    baseURL
  }) => {
    const verifyClean = attachConsoleGuards(page);
    const stamp = `${Date.now()}-${Math.floor(Math.random() * 10000)}`;
    const ownerEmail = `ws-owner-${stamp}@example.com`;
    const collabEmail = `ws-collab-${stamp}@example.com`;
    const collabPassword = 'CollabPassword123!';

    const collab = await seedUserOnly(baseURL!, {
      email: collabEmail,
      name: 'Collaborator One',
      language: 'en',
      password: collabPassword
    });

    await seedBrowserSession(page, {
      email: ownerEmail,
      name: 'Workspace Owner',
      language: 'en',
      password: collabPassword
    });

    // Sole workspace delete button is guarded/disabled
    await page.goto('/workspaces');
    await expect(page.getByTestId('workspaces-grid').locator('article')).toHaveCount(1);
    await expect(page.locator('[data-testid^="workspace-delete-"]').first()).toBeDisabled();

    // Create secondary workspace "Team Alpha"
    await page.getByTestId('create-workspace-button').click();
    await page.getByTestId('workspace-form-name').fill('Team Alpha');
    await page.getByTestId('workspace-form-description').fill('Shared RBAC test workspace');
    await page.getByTestId('workspace-form-submit').click();

    const teamCard = page.locator('article', { hasText: 'Team Alpha' });
    await expect(teamCard).toBeVisible();
    await expect(teamCard).toContainText('Active');

    // Invite collaborator as read-only viewer
    await page.goto('/workspaces/members');
    await page.getByTestId('invite-email-input').fill(collabEmail);
    await page.getByTestId('invite-role-select').selectOption('viewer');
    await page.getByTestId('invite-member-submit').click();
    await expect(page.getByTestId('members-list')).toContainText(collabEmail);

    // Log out owner and sign in as collaborator
    await page.getByTestId('logout-button').click();
    await expect(page).toHaveURL(/\/auth\/login$/);
    await page.goto('/auth/login-password');
    await page.getByTestId('login-email-input').fill(collabEmail);
    await page.getByTestId('login-password-input').fill(collabPassword);
    await page.getByTestId('login-password-submit').click();
    await expect(page).toHaveURL(/\/todos$/);

    // Switch collaborator active workspace to Team Alpha
    const switcher = page.getByTestId('workspace-switcher');
    const teamOptionValue = await switcher
      .locator('option', { hasText: 'Team Alpha' })
      .getAttribute('value');
    await switcher.selectOption(teamOptionValue!);

    // Viewer sees read-only banner and no New Task / Quick Add controls
    await expect(page.getByTestId('readonly-permission-banner')).toBeVisible();
    await expect(page.getByTestId('new-todo-button')).toHaveCount(0);

    // Sign back in as owner and upgrade collaborator preset from viewer -> editor
    await page.getByTestId('logout-button').click();
    await seedBrowserSession(page, {
      email: ownerEmail,
      name: 'Workspace Owner',
      language: 'en'
    });
    await page.goto('/workspaces');
    const ownerSwitcher = page.getByTestId('workspace-switcher');
    const ownerTeamVal = await ownerSwitcher
      .locator('option', { hasText: 'Team Alpha' })
      .getAttribute('value');
    await ownerSwitcher.selectOption(ownerTeamVal!);

    await page.goto('/workspaces/members');
    await page.getByTestId(`edit-member-perms-${collab.id}`).click();
    await page.getByTestId('member-role-preset-select').selectOption('editor');
    await page.getByTestId('member-permissions-save').click();

    // Sign in as collaborator again -> now can create Todo in Team Alpha
    await page.getByTestId('logout-button').click();
    await seedBrowserSession(page, {
      email: collabEmail,
      name: 'Collaborator One',
      language: 'en'
    });
    await page.goto('/todos');
    await expect(page.getByTestId('readonly-permission-banner')).toHaveCount(0);
    await expect(page.getByTestId('new-todo-button')).toBeVisible();
    await page.getByTestId('quick-add-todo-input').fill('Collaborator Editor Task');
    await page.getByTestId('quick-add-todo-submit').click();
    await expect(page.getByTestId('todo-list')).toContainText('Collaborator Editor Task');

    // Clean up secondary workspace as owner
    await page.getByTestId('logout-button').click();
    await seedBrowserSession(page, {
      email: ownerEmail,
      name: 'Workspace Owner',
      language: 'en'
    });
    await page.goto('/workspaces');
    const teamCardOwner = page.locator('article', { hasText: 'Team Alpha' });
    const teamCardTestId = await teamCardOwner.getAttribute('data-testid');
    const teamWsId = teamCardTestId!.replace('workspace-card-', '');
    await page.getByTestId(`workspace-delete-${teamWsId}`).click();
    await expect(page.getByTestId('workspaces-grid').locator('article')).toHaveCount(1);

    verifyClean();
  });
});
