import { test } from '../fixtures/auth';
import { expect } from '@playwright/test';

test.describe('Repository Management', () => {
    test('create and delete a repository', async ({ page }) => {
        const repoName = `e2e-repo-${Date.now()}`;

        await page.goto('/');
        await page.click('a:has-text("New")');

        await page.fill('input[type="text"]', repoName);
        await page.fill('textarea', 'E2E test repo');
        await page.click('button:has-text("Create")');

        // Check redirected to repo page
        await page.waitForURL(/\/repo\/\d+/);
        await expect(page.locator(`text=${repoName}`)).toBeVisible();

        // Go to repo settings to delete
        await page.click('a:has-text("Settings")');
        await page.waitForSelector('button:has-text("Delete")');
        await page.click('button:has-text("Delete")');
        // Wait for redirect back to dashboard
        await page.waitForURL('/');
        await expect(page.locator('text=Repositories')).toBeVisible();
    });

    test('view public repositories', async ({ page }) => {
        // As a logged-in user, go to dashboard and see the repo list
        await page.goto('/');
        await expect(page.locator('text=My Repos')).toBeVisible();
    });
});
