import { test } from '../fixtures/auth';
import { expect } from '@playwright/test';

test.describe('Authentication', () => {
    test('register a new user and see dashboard', async ({ page }) => {
        const ts = Date.now();
        const username = `e2e-reg-${ts}`;

        await page.goto('/');
        await page.click('a:has-text("Register")');

        // Inputs don't have name attributes; use nth-child or label text
        const inputs = page.locator('form input');
        await inputs.nth(0).fill(username);
        await inputs.nth(1).fill(`${username}@test.com`);
        await inputs.nth(2).fill('pass123');
        await page.click('button:has-text("Register")');

        await expect(page.locator('text=Repositories')).toBeVisible({ timeout: 10000 });
        await expect(page.locator(`text=${username}`)).toBeVisible();
    });

    test('login and logout flow', async ({ page, authUser }) => {
        const { username } = authUser;

        // Already logged in via fixture; go to dashboard
        await page.goto('/');
        await expect(page.locator('text=Repositories')).toBeVisible();

        // Logout via nav link
        await page.click('a:has-text("Logout")');
        await page.waitForURL(/\/login/);
        await expect(page.locator('text=Login')).toBeVisible();

        // Login again
        await page.fill('form input[type="text"]', username);
        await page.fill('form input[type="password"]', 'pass123');
        await page.click('button:has-text("Login")');
        await expect(page.locator('text=Repositories')).toBeVisible();
    });

    test('reject bad credentials', async ({ page }) => {
        await page.goto('/login');
        await page.fill('form input[type="text"]', 'nonexistent');
        await page.fill('form input[type="password"]', 'wrong');
        await page.click('button:has-text("Login")');
        await expect(page.locator('.msg-err')).toBeVisible({ timeout: 5000 });
    });

    test('redirect to login when accessing protected page', async ({ page }) => {
        await page.goto('/settings');
        await expect(page).toHaveURL(/\/login/);
    });
});
