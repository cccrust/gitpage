# api.ts — HTTP Client Architecture

## Overview

`api.ts` is the sole HTTP communication layer for the entire frontend. It provides a generic `request<T>()` wrapper around the bare `fetch` API, along with ~50 typed convenience functions that mirror every backend REST endpoint. There is no secondary HTTP client, no interceptors, no middleware — just a single request pipeline that every page component imports from.

## The `request<T>(method, path, body?)` Pattern

The core abstraction is a single generic function:

```
request<T>(method, path, body?) → Promise<T>
```

- `method` — any HTTP verb (`"GET"`, `"POST"`, `"PUT"`, `"DELETE"`)
- `path` — relative URL path (e.g. `"/api/auth/login"`). A `BASE` constant is prepended (empty string `''` in development, can be set for proxied deployments).
- `body` — optional payload, serialized as JSON via `JSON.stringify()`
- Returns — a `Promise<T>` where `T` is the expected response shape

This pattern avoids per-endpoint boilerplate for headers, token injection, status checking, and JSON parsing. Every API function in the file is a thin closure that calls `request<T>()` with the correct type parameter and path.

## JWT Token Injection

The JWT token is stored in `localStorage` under the key `"token"`. Three small utility functions manage it:

- `getToken()` — reads `localStorage.getItem('token')`
- `setToken(t)` — writes to `localStorage.setItem('token', t)`
- `clearToken()` — removes it via `localStorage.removeItem('token')`
- `isLoggedIn()` — shorthand for `!!getToken()`

Inside `request()`, if a token exists, it is attached as an `Authorization: Bearer <token>` header. This means **every** API call automatically carries the JWT when the user is logged in. There is no per-call opt-out — public endpoints (like login, register, public repo listing) simply ignore the token on the server side.

### Why localStorage and Not a State Library

The JWT is kept in `localStorage` rather than in a React context or state store for two reasons:

1. **Persistence across page reloads** — localStorage survives full page refreshes, which a JavaScript variable would not.
2. **No state management library** — the project deliberately avoids Redux, Zustand, React Context, or any global store. Keeping the token in localStorage means it can be read synchronously from any module without a React component tree. The `isLoggedIn()` function is called from `Layout.tsx` directly, outside any provider.

The trade-off is that React components do not reactively re-render when the token changes. The login/logout flow forces a full navigation (`navigate('/')` or `navigate('/login')`) after `setToken`/`clearToken`, which naturally causes a re-render.

## Error Handling and Response Parsing

The `request()` function handles three layers:

1. **HTTP-level errors** — if `res.ok` is false (status outside 200–299), the response body is read as text. An attempt is made to parse it as JSON and extract an `.error` field (the convention used by all backend handlers via `AppError`). If that fails, the raw text or the HTTP status code is used as the fallback error message.
2. **Network errors** — `fetch` throws `TypeError` on network failures. These propagate as unhandled promise rejections and must be caught by the caller.
3. **JSON parsing** — successful responses are assumed to be valid JSON. There is no validation or runtime type checking — the `T` generic is a TypeScript compile-time assertion only.

The resulting error is thrown as `new Error(message)` with a string message. All page components catch errors in `.catch()` handlers and display them via an `<div className="error-box">` or `alert()`.

### Non-JSON Responses (File Operations)

Two functions—`getRawFile()` and `writeFile()`—bypass the `request()` wrapper entirely because they deal with raw `Response` objects (for binary file downloads) or non-JSON `Blob`/string bodies. These manually construct `fetch` calls with the JWT header, preserving the same authentication pattern but without the JSON parsing wrapper.

## Typed API Functions

Every backend endpoint has a corresponding exported function in `api.ts`:

| Category | Functions |
|----------|-----------|
| **Auth** | `register`, `login`, `me`, `changePassword` |
| **Repos** | `listRepos`, `createRepo`, `getRepo`, `deleteRepo`, `listPublicRepos` |
| **Content** | `listTree`, `getBlob`, `getReadme`, `listCommits` |
| **Pages** | `getPagesConfig`, `updatePagesConfig`, `deployPages` |
| **Apps** | `getAppsConfig`, `updateAppsConfig`, `deployApps`, `deleteAppsConfig` |
| **Deploy Logs** | `listDeploys`, `getDeployLog` |
| **Working Tree** | `listWorkingTree`, `getRawFile`, `writeFile`, `deleteFile`, `mkdir`, `moveFile`, `getStatus`, `commitRepo` |
| **Issues** | `listIssues`, `createIssue`, `getIssue`, `updateIssue`, `deleteIssue` |
| **Labels** | `listLabels`, `createLabel`, `deleteLabel` |
| **Comments** | `listComments`, `addComment` |
| **Pull Requests** | `listPRs`, `createPR`, `getPR`, `updatePR`, `mergePR`, `getPRDiff` |
| **Fork** | `forkRepo` |
| **Stars** | `starRepo`, `unstarRepo`, `getStarStatus`, `listStargazers`, `listUserStars` |
| **Watches** | `watchRepo`, `unwatchRepo`, `getWatchStatus` |
| **SSH Keys** | `listSshKeys`, `addSshKey`, `deleteSshKey` |
| **Docker/SSH Info** | `getSshInfo` |
| **User** | `updateProfile` |
| **Tokens** | `listTokens`, `createToken`, `deleteToken` |
| **Collaborators** | `listCollaborators`, `addCollaborator`, `removeCollaborator` |
| **Secrets** | `listSecrets`, `createSecret`, `deleteSecret` |
| **Branch Protection** | `listBranchProtections`, `createBranchProtection`, `deleteBranchProtection` |
| **Organizations** | `listMyOrgs`, `createOrg`, `getOrg`, `updateOrg`, `deleteOrg`, `listOrgRepos`, `listOrgMembers`, `addOrgMember`, `removeOrgMember` |

Each function accepts domain-specific parameters (numbers, strings, option bags) and returns a typed response object. None of them handle pagination state, caching, or deduplication — that is left entirely to the calling React component.

## Why No State Management Library

The project's frontend is deliberately minimal:

- **No React Context** — there is no auth context, no user context, no theme context. The JWT token is read directly from `localStorage`. User profile data is fetched on demand in each page that needs it (e.g., `UserSettingsPage` calls `me()`, `RepoPage` calls `me()` to check star status).
- **No cache layer** — every mount of a page re-fetches its data. The `useEffect` dependency array ensures re-fetching when route params change, but there is no stale-while-revalidate, no request deduplication, no optimistic updates.
- **No global store** — each component owns its own `useState` for loading, error, and data states. The closest thing to shared state is `isLoggedIn()` being called from the Layout component and from individual pages, but each call is an independent imperative check.
- **Philosophy** — the backend is the single source of truth. The frontend is a thin rendering layer with minimal client-side logic. This keeps the bundle small, the mental model simple, and avoids the complexity of synchronizing a client-side store with server state.

## Utility Functions

Beyond the API functions, `api.ts` exports:

- `isTextFile(name: string): boolean` — checks a file extension against a hardcoded `Set` of known text extensions (`.txt`, `.md`, `.js`, `.rs`, `.py`, `.json`, etc.). Used by `FileExplorerPage` to decide whether to open a file in the editor or download it as binary.
- `SshInfo` interface — returned by `getSshInfo()`, used by `DockerStatusPage` to display container connection details.

## Reference: Wiki

See `_wiki/jwt-auth.md` for the backend JWT creation and verification flow. The frontend does not decode or inspect the JWT payload — it treats the token as an opaque string that the backend validates on each request.
