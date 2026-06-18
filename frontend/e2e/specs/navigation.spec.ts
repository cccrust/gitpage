import { test } from '../fixtures/auth';
import { expect } from '@playwright/test';

test.describe('Navigation', () => {
    test('home page loads', async ({ page }) => {
        await page.goto('/');
        await expect(page.locator('nav').first()).toBeVisible();
    });

    test('SPA fallback works for unknown routes', async ({ page }) => {
        await page.goto('/some/nonexistent/route');
        await expect(page.locator('nav').first()).toBeVisible();
    });

    test('navigation links exist', async ({ page }) => {
        await page.goto('/');
        // The page should have navigation links (top nav or bottom nav)
        await expect(page.locator('nav').first()).toBeVisible();
        // Check that some link exists
        const links = page.locator('nav a');
        await expect(links.first()).toBeVisible();
    });
});
