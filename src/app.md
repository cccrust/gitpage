# `app.rs` — Router, Middleware, and Fallback Architecture

## Overview

`app.rs` is the central routing and request-processing module. It defines the `AppState` shared struct, builds the complete Axum `Router` with all API routes, applies authentication middleware, and implements the catch-all fallback handler that routes Git/Pages/App/static/SPA traffic.

## AppState — Shared State Design

`AppState` is the single shared state object passed to every handler via Axum's `with_state()`. It is `#[derive(Clone)]` and contains:

| Field | Type | Purpose |
|-------|------|---------|
| `db` | `Database` | SQLite connection pool (actually `Arc<Mutex<Connection>>`, clone is cheap) |
| `config` | `Arc<Config>` | All configuration values (paths, ports, limits) |
| `jwt_expires_hours` | `u64` | JWT expiry duration, extracted from config at startup |
| `app_manager` | `AppProcessManager` | Running app lifecycle manager (inner `Arc`) |
| `docker` | `Option<DockerManager>` | Docker runtime manager, `None` in process mode |

The design keeps state immutable after initialization — no field is ever mutated after the router is built. Mutation happens inside the fields themselves (DB rows, process manager state), not on `AppState`.

## Axum Router Organization

The `create_app()` function builds a `Router` with three layers:

1. **API routes** — explicit `.route()` calls for each REST endpoint
2. **Auth middleware** — applied via `.layer(middleware::from_fn(auth_middleware))`
3. **CORS layer** — applied last (outermost)

Routes are categorized broadly:

- `POST /api/auth/register|login` — public auth endpoints
- `GET /api/auth/me` — authenticated user info
- `PUT /api/auth/password` — password change
- `GET|POST|DELETE /api/repos` — repository CRUD
- `GET /api/:username/:repo_name/tree|blob|readme|commits` — content browsing
- `PUT|DELETE /api/repos/:repo_id/files` — file manager
- `GET|PUT /api/pages/:repo_id` — pages config
- `GET|PUT|DELETE /api/apps/:repo_id` — app config
- `POST /api/apps/:repo_id/deploy` — app deployment
- `GET|POST /api/orgs` — org management
- `GET|POST|PUT|DELETE /api/repos/:repo_id/issues` — issue tracking
- `GET|POST /api/repos/:repo_id/pulls` — pull requests
- `GET|POST|DELETE /api/user/tokens` — personal access tokens
- `GET|POST|DELETE /api/repos/:repo_id/collaborators` — collaborator management
- `GET|POST|DELETE /api/repos/:repo_id/secrets` — encrypted secrets
- `GET|POST|DELETE /api/repos/:repo_id/branch-protections` — branch protection rules
- `PUT|DELETE /api/repos/:repo_id/star|watch` — social features

## Auth Middleware Pattern

The `auth_middleware` function implements a selective auth strategy:

**Public paths** (no auth required):
- CORS preflight (`OPTIONS`) requests
- Non-API paths (static files, pages, git)
- `POST /api/auth/login` and `POST /api/auth/register`
- `GET /api/repos/:id` (single repo lookup)
- All `GET` requests to `/api/*` except `/api/auth/me` and `/api/repos` (list user repos)

**Authenticated-only paths** (all others):
- Missing/invalid `Authorization: Bearer <token>` header returns 401 with Chinese error message

On successful verification, the middleware injects the authenticated user's `user_id` (as `i64`) and `username` (as `String`) into the request's extension map. Handlers extract these via `axum::Extension<i64>` and `axum::Extension<String>`.

**Try-auth on public paths**: Even for public GET requests, the middleware attempts to verify an auth token if one is present, allowing handlers to optionally show private repo content to authorized users.

## Fallback Handler Routing Priority

The `fallback_handler` is the last resort for unmatched paths. It implements a 5-level priority system:

### 1. `/git/{user}/{repo}/{*path}` — Git HTTP Smart Protocol

Receives push/pull requests by spawning `git http-backend` as a subprocess with the correct environment variables (`GIT_PROJECT_ROOT`, `PATH_INFO`, `REQUEST_METHOD`, `CONTENT_TYPE`, etc.). The handler:
- Parses the owner name and repo name from the URL
- Resolves the bare repo path and checks existence
- Forwards the HTTP method, content type, and body to the git subprocess
- Parses the git response headers (including `Status:`) and body
- **Auto-deploys**: if the request is a push (`application/x-git-receive-pack-request`) and succeeds, two background tasks are spawned to redeploy pages and apps

### 2. `/pages/{user}/{repo}/{*path}` — Static Pages Hosting

Serves pre-deployed static files from the pages output directory. Delegates to `handlers::git_smart::serve_pages()` which resolves files with MIME type detection and index.html fallback for directory paths.

### 3. `/app/{user}/{repo}/{*path}` — Reverse Proxy to User Apps

Proxies HTTP requests to running user applications:
- Resolves the repo (tries user then org)
- Looks up the app process in `AppProcessManager`
- In Docker mode, resolves the container's IP address via `get_container_ip()`; otherwise uses `127.0.0.1`
- Forwards the request using `reqwest` with the app's port and path
- Returns 502 if the app is not running or unreachable

### 4. Static Files

Serves files from `frontend/dist/` first, then `static/`. Each directory is tried in order; the first match wins. Files are served with correct MIME types via `mime_guess`.

### 5. SPA Fallback

If the path contains no dot (indicating it's not a file request), `index.html` is served from whichever static directory exists. This enables the React SPA's client-side routing to work for any URL like `/repos/123` or `/orgs/my-org`.

### 404

If none of the above match, returns a Chinese "page not found" message.

## Auto-Deploy Logic on Push

When a successful git push is detected in the fallback handler, two `tokio::spawn` tasks run concurrently:

### `auto_deploy_pages()`
1. Resolves the repo owner (user or org)
2. Fetches the pages config from DB
3. If pages are enabled, calls `git::deploy_pages()` which checks out the configured branch/source_dir from the bare repo into the pages output directory using libgit2

### `auto_deploy_app()`
1. Resolves the repo owner
2. Fetches the app config from DB
3. If the app is enabled, creates a deploy log entry (status: running)
4. Calls `deploy::deploy_app()` to checkout source, detect project type, build, allocate port, and start the process
5. Updates the deploy log with success/failure status and output

This auto-deploy design means users only need to `git push` to trigger deployment — no separate CI/CD pipeline is needed.

## Content Route Resolution

The `resolve_owner_and_repo()` helper (used by the app proxy and auto-deploy functions) resolves a `:username` path segment against both users and organizations:
1. Tries user lookup first (matches GitHub's behavior)
2. Falls back to org lookup
3. Returns the `(Repository, owner_name)` tuple where `owner_name` is the resolved name used for filesystem paths

This pattern is also replicated in `handlers/content.rs::resolve_repo()` for content browsing routes.

## CORS Configuration

The CORS layer is built from the `[cors]` config section. If `allowed_origins` contains `"*"`, permissive mode is used (all origins allowed). Otherwise, specific origins are parsed as `HeaderValue` and whitelisted. Allowed methods include GET, POST, PUT, DELETE, OPTIONS; allowed headers include Content-Type and Authorization.

## Reference

- `_doc/v0.1.md` — Original Axum router design, Git HTTP backend
- `_doc/v0.2.md` — SPA fallback for React frontend
- `_doc/v0.3.md` — Pages serving via fallback handler
- `_doc/v0.6.md` — Reverse proxy for user apps
- `_doc/v0.7.md` — Auto-deploy on push
- `_doc/v1.0.md` — Stable routing architecture
- `_doc/v1.0.1.md` — Org-aware content resolution
- `_doc/v1.2.md` — Docker container proxy target
- `_doc/api.md` — Complete route table
