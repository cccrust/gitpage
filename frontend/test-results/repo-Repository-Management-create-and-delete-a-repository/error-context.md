# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: repo.spec.ts >> Repository Management >> create and delete a repository
- Location: e2e/specs/repo.spec.ts:5:5

# Error details

```
Test timeout of 60000ms exceeded.
```

```
Error: page.click: Test timeout of 60000ms exceeded.
Call log:
  - waiting for locator('a:has-text("New")')
    - locator resolved to <a class="" href="/new" data-discover="true">New</a>
  - attempting click action
    2 × waiting for element to be visible, enabled and stable
      - element is not visible
    - retrying click action
    - waiting 20ms
    2 × waiting for element to be visible, enabled and stable
      - element is not visible
    - retrying click action
      - waiting 100ms
    113 × waiting for element to be visible, enabled and stable
        - element is not visible
      - retrying click action
        - waiting 500ms

```

# Page snapshot

```yaml
- generic [ref=e2]:
  - navigation [ref=e3]:
    - generic [ref=e4]:
      - link "gitpage" [ref=e5] [cursor=pointer]:
        - /url: /
      - link "Login" [ref=e6] [cursor=pointer]:
        - /url: /login
      - link "Register" [ref=e7] [cursor=pointer]:
        - /url: /register
  - generic [ref=e11]:
    - heading "gitpage" [level=1] [ref=e12]
    - paragraph [ref=e13]: Self-hosted Git platform
    - paragraph [ref=e14]:
      - link "Login" [ref=e15] [cursor=pointer]:
        - /url: /login
      - text: or
      - link "Register" [ref=e16] [cursor=pointer]:
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
> 9  |         await page.click('a:has-text("New")');
     |                    ^ Error: page.click: Test timeout of 60000ms exceeded.
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
  31 |         await expect(page.locator('text=My Repos')).toBeVisible();
  32 |     });
  33 | });
  34 | 
```