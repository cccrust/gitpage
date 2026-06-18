# Frontend Source Code

## Stack

- **React 19** with **TypeScript** and **Vite** for build tooling
- **React Router v6** for client-side routing (BrowserRouter, Routes, Route, Navigate)
- **No state management library** — all component state is managed through React's built-in `useState` and `useEffect` hooks. Props are passed only one level deep where needed; there is no global store, no context providers for data, and no Redux/Zustand/Recoil. Shared state (like the JWT token) lives in `localStorage` and is read imperatively via `api.ts` helpers.
- **No CSS framework** — styles are a single `index.css` with dark-theme custom properties. No Tailwind, no CSS modules, no styled-components.
- **UI language** — all user-facing strings are in Chinese (Traditional). Error messages, button labels, empty states, and breadcrumbs all use Chinese. Only API-level terms like "token" or "branch" remain in English.
- **HTTP client** — bare `fetch` wrapped in a generic `request<T>()` function in `api.ts`. No axios, no react-query, no SWR.

## Module Map

| File | Purpose |
|------|---------|
| `api.ts` | HTTP client with JWT injection, response parsing, error extraction, and all typed API functions (~50 endpoints) |
| `App.tsx` | Root component — defines the `BrowserRouter`, wraps all routes in `<Layout>`, declares ~30 `<Route>` elements, and catches unmatched paths with a `*` → `/` redirect |
| `main.tsx` | Vite entry point — calls `createRoot`, renders `<App>` inside `<StrictMode>`, imports `index.css` globally |
| `index.css` | Global stylesheet — dark theme, layout grid, form controls, button classes, file list, pagination, modal overlays, markdown body, code blocks, commit list, deploy log entries, SSH key list |

## Components

| Component | File | Purpose |
|-----------|------|---------|
| Layout | `components/Layout.tsx` | Top navigation bar + bottom navigation bar wrapper. Checks `isLoggedIn()` to conditionally render Login/Register vs Repos/Orgs/Settings/Logout links |
| Spinner | `components/Spinner.tsx` | Minimal loading indicator — renders a `<div className="loading">` with configurable text (default `"Loading..."`) |
| MarkdownView | `components/MarkdownView.tsx` | Renders pre-rendered HTML into the DOM via `dangerouslySetInnerHTML`, then applies KaTeX for math and Mermaid for diagrams via global `window` references and polling |
| Pagination | `components/Pagination.tsx` | Offset-based prev/next pagination — receives `page`, `totalPages`, and `onPageChange` callback. Hides itself when `totalPages <= 1` |

## Pages

| Route(s) | Page Component | Purpose |
|----------|----------------|---------|
| `/` | `Dashboard` | Landing page — shows login/register callout for anonymous users, or the authenticated user's repo list ("My Repos" / "Starred" tabs) with a global search bar that queries `/api/repos/search` |
| `/login` | `LoginPage` | Username + password form, calls `login()` API, stores JWT via `setToken()`, redirects to `/` |
| `/register` | `RegisterPage` | Username + email + password form, calls `register()` API, stores JWT, redirects to `/` |
| `/new` | `NewRepoPage` | Create repository form — name, description, private flag, optional org owner selector (fetches `listMyOrgs`) |
| `/repo/:id` | `RepoPage` | Repository home — breadcrumb, star/fork buttons, file tree, rendered README, recent commits, clone URL. Links to Commits/Files/Issues/PRs/Pages/App/SSH/Settings |
| `/repo/:id/*` | `FileViewPage` | Catch-all for repo paths — parses `*` wildcard into `tree\|blob`/`branch`/`path`, renders directory listing or file content (markdown rendered via MarkdownView, plain text as `<pre>`) |
| `/repo/:id/files` | `FileExplorerPage` | Staging area file manager — directory tree navigation, file upload, New File / New Folder buttons, delete, Save Version (commit dialog with change list) |
| `/repo/:id/files/edit` | `FileEditorPage` | Text file editor — loads file content from staging via `getRawFile`, saves via `writeFile`. Also handles new file creation when path ends in `/files/new` |
| `/repo/:id/files/new` | `FileEditorPage` | Same component as edit — prompts for filename on first save |
| `/repo/:id/commits/:branch` | `CommitsPage` | Commit history for a branch — lists SHA, message, author, timestamp |
| `/repo/:id/pages` | `PagesSettingsPage` | Gitpage Pages configuration — branch, source directory, custom domain, enable toggle, deploy/redeploy button |
| `/repo/:id/app` | `AppSettingsPage` | Gitpage App hosting configuration — branch, source dir, build/start commands, env vars (JSON), enable toggle, status display, redeploy |
| `/repo/:id/deploys` | `DeployLogsPage` | List of deploy log entries — status icon, timestamp, links to detail view |
| `/repo/:id/deploys/:deployId` | `DeployLogDetailPage` | Single deploy log detail — status, timestamps, full log output in `<pre>` |
| `/repo/:id/settings` | `RepoSettingsPage` | Repository settings — rename, description, private toggle, danger zone (delete with name confirmation). Links to collaborators/secrets/branch-protection |
| `/repo/:id/ssh` | `RepoSSHKeysPage` | SSH key management for repo shell access — add/delete public keys, connection instructions |
| `/repo/:id/collaborators` | `RepoSettingsCollaboratorsPage` | Collaborator management — add/remove users with permission levels |
| `/repo/:id/secrets` | `RepoSettingsSecretsPage` | Repo-level secrets (env vars for app deploy) — add/delete key-value pairs |
| `/repo/:id/branch-protection` | `RepoSettingsBranchProtectionPage` | Branch protection rules — add/delete patterns with PR requirement settings |
| `/u/:username` | `UserProfilePage` | User profile page — avatar, bio, join date, list of public repositories |
| `/settings` | `UserSettingsPage` | User settings — bio, avatar URL, password change. Links to Access Tokens page |
| `/settings/tokens` | `SettingsTokensPage` | Personal access tokens — create (shows raw token once), list, delete |
| `/orgs` | `OrgList` | Organization list for the logged-in user — shows display name, description, role badge |
| `/orgs/new` | `OrgCreate` | Create organization form — name, display name, description |
| `/org/:name` | `OrgDetail` | Organization home — display name, description, repo list, links to members/settings |
| `/org/:name/members` | `OrgMembers` | Organization member management — list members with roles, add/remove members |
| `/org/:name/settings` | `OrgSettings` | Organization settings — edit display name and description |
| `/docker-status` | `DockerStatusPage` | Docker runtime container info — mode (docker vs process), container name, SSH port, SSH password, connection command |

### Unrouted Pages

The following page components exist in the source tree but are **not registered in App.tsx routes**. They are accessible only if navigated to programmatically or via direct URL entry:

| File | Expected Route | Purpose |
|------|----------------|---------|
| `IssueList` | `/repo/:id/issues` | Lists issues with open/closed/all tab filter |
| `IssueDetail` | `/repo/:id/issues/:issueNumber` | Single issue view with title, body, labels, state toggle, comments |
| `IssueNew` | `/repo/:id/issues/new` | Create issue form — title + description |
| `PRList` | `/repo/:id/pulls` | Lists pull requests with open/closed/merged/all tab filter |
| `PRDetail` | `/repo/:id/pulls/:prNumber` | Single PR view with merge button, close, diff file list, comments |
| `PRNew` | `/repo/:id/pulls/new` | Create PR form — base branch, head branch, title, description |
| `SettingsBranches` | standalone | Standalone branch protection management (duplicate of the routed version with richer UI) |
| `SettingsCollaborators` | standalone | Standalone collaborator management (duplicate of the routed version) |
| `SettingsSecrets` | standalone | Standalone secret management (duplicate of the routed version) |

The Issue/PR pages are referenced from `RepoPage` action links but have no corresponding Route — they represent work-in-progress (Issues/PRs are v2.0 features per API types in `api.ts`).
