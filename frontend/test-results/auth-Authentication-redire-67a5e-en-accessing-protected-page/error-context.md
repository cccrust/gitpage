# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: auth.spec.ts >> Authentication >> redirect to login when accessing protected page
- Location: e2e/specs/auth.spec.ts:50:5

# Error details

```
Error: expect(page).toHaveURL(expected) failed

Expected pattern: /\/login/
Received string:  "http://localhost:8080/settings"
Timeout: 5000ms

Call log:
  - Expect "toHaveURL" with timeout 5000ms
    14 × unexpected value "http://localhost:8080/settings"

```

```yaml
- navigation:
  - link "gitpage":
    - /url: /
  - link "Login":
    - /url: /login
  - link "Register":
    - /url: /register
- text: 請先登入
```

# Test source

```ts
  1  | import { test } from '../fixtures/auth';
  2  | import { expect } from '@playwright/test';
  3  | 
  4  | test.describe('Authentication', () => {
  5  |     test('register a new user and see dashboard', async ({ page }) => {
  6  |         const ts = Date.now();
  7  |         const username = `e2e-reg-${ts}`;
  8  | 
  9  |         await page.goto('/');
  10 |         await page.click('a:has-text("Register")');
  11 | 
  12 |         // Inputs don't have name attributes; use nth-child or label text
  13 |         const inputs = page.locator('form input');
  14 |         await inputs.nth(0).fill(username);
  15 |         await inputs.nth(1).fill(`${username}@test.com`);
  16 |         await inputs.nth(2).fill('pass123');
  17 |         await page.click('button:has-text("Register")');
  18 | 
  19 |         await expect(page.locator('text=Repositories')).toBeVisible({ timeout: 10000 });
  20 |         await expect(page.locator(`text=${username}`)).toBeVisible();
  21 |     });
  22 | 
  23 |     test('login and logout flow', async ({ page, authUser }) => {
  24 |         const { username } = authUser;
  25 | 
  26 |         // Already logged in via fixture; go to dashboard
  27 |         await page.goto('/');
  28 |         await expect(page.locator('text=Repositories')).toBeVisible();
  29 | 
  30 |         // Logout via nav link
  31 |         await page.click('a:has-text("Logout")');
  32 |         await page.waitForURL(/\/login/);
  33 |         await expect(page.locator('text=Login')).toBeVisible();
  34 | 
  35 |         // Login again
  36 |         await page.fill('form input[type="text"]', username);
  37 |         await page.fill('form input[type="password"]', 'pass123');
  38 |         await page.click('button:has-text("Login")');
  39 |         await expect(page.locator('text=Repositories')).toBeVisible();
  40 |     });
  41 | 
  42 |     test('reject bad credentials', async ({ page }) => {
  43 |         await page.goto('/login');
  44 |         await page.fill('form input[type="text"]', 'nonexistent');
  45 |         await page.fill('form input[type="password"]', 'wrong');
  46 |         await page.click('button:has-text("Login")');
  47 |         await expect(page.locator('.msg-err')).toBeVisible({ timeout: 5000 });
  48 |     });
  49 | 
  50 |     test('redirect to login when accessing protected page', async ({ page }) => {
  51 |         await page.goto('/settings');
> 52 |         await expect(page).toHaveURL(/\/login/);
     |                            ^ Error: expect(page).toHaveURL(expected) failed
  53 |     });
  54 | });
  55 | 
```