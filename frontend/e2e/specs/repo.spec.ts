import { test } from '../fixtures/auth';
import { expect } from '@playwright/test';

test.describe('Repository Management', () => {
    test('create and delete a repository', async ({ page, authUser }) => {
        const repoName = `e2e-repo-${Date.now()}`;
        const { username } = authUser;

        await page.goto('/');
        await page.click('a:has-text("+ New")');

        await page.fill('input[placeholder="Repository name"]', repoName);
        await page.fill('input[placeholder="Description (optional)"]', 'E2E test repo');
        await page.click('button:has-text("Create")');

        // Check redirected to repo page
        await page.waitForURL(/\/repo\/\d+/, { timeout: 15000 });
        await expect(page.locator(`h1:has-text("${repoName}")`)).toBeVisible();

        // Go to repo settings to delete
        await page.locator('.actions a:has-text("Settings")').click();
        await page.waitForURL(/\/repo\/\d+\/settings/, { timeout: 10000 });
        await expect(page.locator('h2:has-text("Repository Settings")')).toBeVisible({ timeout: 10000 });
        // Type repo name to confirm deletion
        const confirmInput = page.locator('.danger-zone input[type="text"]');
        await confirmInput.fill(repoName);
        await page.click('button:has-text("刪除此倉庫")');
        // Wait for redirect back to dashboard
        await page.waitForURL('/', { timeout: 15000 });
        await expect(page.locator('h1:has-text("Repositories")')).toBeVisible();
    });

    test('view public repositories', async ({ page, authUser }) => {
        // As a logged-in user, go to dashboard and see the repo list
        await page.goto('/');
        await expect(page.locator('text=My Repos')).toBeVisible();
    });
});
