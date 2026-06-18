# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: repo.spec.ts >> Repository Management >> view public repositories
- Location: e2e/specs/repo.spec.ts:28:5

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: locator('text=My Repos')
Expected: visible
Timeout: 5000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 5000ms
  - waiting for locator('text=My Repos')

```

```yaml
- navigation:
  - link "gitpage":
    - /url: /
  - link "Login":
    - /url: /login
  - link "Register":
    - /url: /register
- heading "gitpage" [level=1]
- paragraph: Self-hosted Git platform
- paragraph:
  - link "Login":
    - /url: /login
  - text: or
  - link "Register":
    - /url: /register
```

# Test source

```ts
  1  | import { test } from '../fixtures/auth';
  2  | import { expect } from '@playwright/test';
  3  | 
  4  | test.describe('Repository Management', () => {
  5  |     test('create and delete a repository', async ({ page }) => {
  6  |         const repoName = `e2e-repo-${Date.now()}`;
  7  | 
  8  |         await page.goto('/');
  9  |         await page.click('a:has-text("New")');
  10 | 
  11 |         await page.fill('input[type="text"]', repoName);
  12 |         await page.fill('textarea', 'E2E test repo');
  13 |         await page.click('button:has-text("Create")');
  14 | 
  15 |         // Check redirected to repo page
  16 |         await page.waitForURL(/\/repo\/\d+/);
  17 |         await expect(page.locator(`text=${repoName}`)).toBeVisible();
  18 | 
  19 |         // Go to repo settings to delete
  20 |         await page.click('a:has-text("Settings")');
  21 |         await page.waitForSelector('button:has-text("Delete")');
  22 |         await page.click('button:has-text("Delete")');
  23 |         // Wait for redirect back to dashboard
  24 |         await page.waitForURL('/');
  25 |         await expect(page.locator('text=Repositories')).toBeVisible();
  26 |     });
  27 | 
  28 |     test('view public repositories', async ({ page }) => {
  29 |         // As a logged-in user, go to dashboard and see the repo list
  30 |         await page.goto('/');
> 31 |         await expect(page.locator('text=My Repos')).toBeVisible();
     |                                                     ^ Error: expect(locator).toBeVisible() failed
  32 |     });
  33 | });
  34 | 
```