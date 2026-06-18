# App.tsx — Router Architecture

## Overview

`App.tsx` is the root React component. It sets up a `BrowserRouter` from React Router v6, wraps every route in a shared `<Layout>` shell, and declares the entire route table (~30 entries). It is **not** a Redux provider, not a theme provider, and not a data-fetching orchestrator — it is purely a routing tree.

## React Router v6 Route Configuration

The route configuration uses the declarative JSX-based approach introduced in React Router v6:

```
<BrowserRouter>
  <Layout>
    <Routes>
      <Route path="/" element={<Dashboard />} />
      ...
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  </Layout>
</BrowserRouter>
```

Key characteristics:

- **`BrowserRouter`** — uses the History API for clean URLs (no `#` hash routing). The production Vite build serves `index.html` as a fallback for all paths, and the Axum backend has a catch-all `/*` handler that serves the static files with SPA fallback (see `_wiki/spa-fallback.md` and `_wiki/axum.md`).
- **`<Routes>`** — React Router v6's replacement for `<Switch>`. It automatically picks the most-specific matching route and stops evaluating after the first match. Child `<Route>` elements are evaluated depth-first.
- **`path` patterns** — uses `:id`, `:username`, `:branch` as dynamic segments. The `*` wildcard in `path="/repo/:id/*"` captures the remainder of the URL into `params['*']`, which `FileViewPage` splits to determine the resource type.

## Layout Component as Wrapper

The `<Layout>` component is rendered **outside** `<Routes>` but **inside** `<BrowserRouter>`. This means:

- Layout renders once and is shared across all pages (it does not remount on navigation).
- Layout has access to router hooks (`useLocation`, `useNavigate`) for highlighting the active link and handling logout.
- Layout calls `isLoggedIn()` on every render to decide whether to show the authenticated or anonymous navigation bar.
- There is no nested layout per section — the same top nav + bottom nav shell appears on every page.

This pattern is simple and avoids the complexity of outlet-based nested layouts. The downside is that there is no way to hide the nav for specific routes (like a full-screen editor) without adding conditional logic inside Layout itself.

## Route Parameter Matching

Route parameters are extracted with `useParams()` in each page component:

```
const { id } = useParams<{ id: string }>()
```

The project uses **string IDs** throughout the route definitions (e.g., `/repo/:id`), even though the backend expects numeric IDs. Pages parse the string to an integer with `parseInt(id)` and handle the `NaN` case with an error state. This is a convention choice — using `:id` rather than `:repoId(\\d+)` keeps route definitions simple at the cost of runtime validation in each component.

The wildcard route `/repo/:id/*` is particularly important:

```
<Route path="/repo/:id/*" element={<FileViewPage />} />
```

This catches all sub-paths of a repo that are not matched by a more specific route (like `/repo/:id/files` or `/repo/:id/settings`). `FileViewPage` accesses the wildcard via `useParams()['*']` and splits it into its meaningful parts: `tree|blob`/`branch`/`path`. For example, a URL like `/repo/5/blob/main/src/index.ts` results in `* = "blob/main/src/index.ts"`, which is parsed into branch=`main` and path=`src/index.ts`.

## Protected Routes vs Public Routes

There is **no route guard** in the traditional sense. The project does not use React Router's `Navigate` for authentication checks at the route level. Instead:

- **Public routes** (`/login`, `/register`) render their forms unconditionally. If a logged-in user visits `/login`, the form renders but does nothing unusual.
- **Semi-protected routes** call `isLoggedIn()` inside the component. If the user is not logged in, the component renders an `<div className="error-box">` with "請先登入" (please log in first). Examples: `UserSettingsPage`, `DockerStatusPage`, `SettingsBranches`, `SettingsCollaborators`, `SettingsSecrets`.
- **Dashboard** (`/`) is public but shows different content based on `isLoggedIn()` — a welcome callout for anonymous users, or the repo list for authenticated users.

This approach is intentionally loose. There is no redirect-to-login flow; protected pages just show an error message. The backend ultimately enforces authorization via JWT validation — the frontend merely provides a passable UX hint.

## The `*` Catch-All

The last route in the `<Routes>` block:

```
<Route path="*" element={<Navigate to="/" replace />} />
```

This catches any unrecognized path and redirects to the root (`/`). The `replace` prop ensures the history entry is replaced (no back-button to the broken URL). The `*` pattern at the end acts as a catch-all because `<Routes>` evaluates children in order and stops at the first match.

This catch-all is distinct from the Axum server's SPA fallback. The Axum backend serves the `index.html` for any unmatched path (see `_wiki/spa-fallback.md`). The React catch-all is a second layer that redirects truly unknown frontend paths to the root.

## Missing Routes

Several page components exist in the `pages/` directory but are **not registered** in `App.tsx`. The most notable are the Issues and Pull Requests pages:

- `IssueList` (expected route: `/repo/:id/issues`)
- `IssueDetail` (expected route: `/repo/:id/issues/:issueNumber`)
- `IssueNew` (expected route: `/repo/:id/issues/new`)
- `PRList` (expected route: `/repo/:id/pulls`)
- `PRDetail` (expected route: `/repo/:id/pulls/:prNumber`)
- `PRNew` (expected route: `/repo/:id/pulls/new`)

These are referenced from `RepoPage` via `<Link>` components (the "Issues" and "PRs" buttons), but clicking them leads to the `*` catch-all redirect back to `/`. This indicates that Issues and PRs are partially implemented — the API functions and page components exist, but the routes were never wired into `App.tsx`.

## Reference: Wiki

See `_wiki/spa-fallback.md` for the Axum handler that serves `index.html` for unmatched paths. See `_wiki/axum.md` for the overall server-side routing architecture, including the route fallback order (git → pages → app → static files → SPA fallback).
