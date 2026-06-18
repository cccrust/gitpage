import { test as base, type Page } from '@playwright/test';

// Request helper
async function api(method: string, path: string, token?: string, body?: unknown) {
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    if (token) headers['Authorization'] = `Bearer ${token}`;
    const res = await fetch(`http://localhost:8080${path}`, {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
    });
    return res.json();
}

export const test = base.extend<{
    authUser: { page: Page; username: string; token: string };
}>({
    authUser: async ({ browser }, use) => {
        const context = await browser.newContext();
        const page = await context.newPage();
        const ts = Date.now();
        const username = `e2e-${ts}`;

        // Register via API
        const res = await api('POST', '/api/auth/register', undefined, {
            username, email: `${username}@test.com`, password: 'pass123',
        });
        const token = res.token;

        // Set JWT in localStorage so the SPA thinks we're logged in
        await page.goto('/');
        await page.evaluate((t) => {
            localStorage.setItem('token', t);
        }, token);
        // Reload to pick up the token
        await page.goto('/');
        await page.waitForSelector('text=Repositories', { timeout: 10000 });

        await use({ page, username, token });
        await context.close();
    },
});
