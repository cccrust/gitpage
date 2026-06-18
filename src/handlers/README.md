# Gitpage Handlers

Each handler module corresponds to a domain of the Gitpage API. All handlers share
the same signature pattern: `async fn(State, Extension, Path/Query/Json) -> Result<Json, AppError>`.

| File | Purpose |
|------|---------|
| mod.rs | Re-exports all handler modules |
| auth.rs | Register, login, me, change_password, update_profile, ssh_info |
| repos.rs | Repo CRUD, search, fork |
| content.rs | Git tree/blob/readme/commit browsing |
| files.rs | Staging area file manager |
| pages.rs | Pages config + deploy |
| apps.rs | Apps config + deploy logs |
| git_smart.rs | Static pages serving |
| ssh_keys.rs | SSH key CRUD |
| orgs.rs | Organization CRUD + members |
| issues.rs | Issues, labels, comments |
| pulls.rs | PR CRUD, merge, diff |
| settings.rs | Access tokens, collaborators, secrets, branch protection |
| stars.rs | Star/unstar, watch/unwatch |

## Route Registration

Routes are defined in `src/app.rs`. Handlers are pure logic — they receive state via
`axum::extract::State<AppState>`, authentication via `axum::Extension<user_id>`,
and return `Result<Json<Value>, AppError>`.

## Error Handling

All handlers return `AppError` — a centralized error enum that maps to HTTP status
codes automatically via `axum::response::IntoResponse`. See `_wiki/apperror-pattern.md`.

## Authentication

Most mutations require `axum::Extension(user_id)` injected by the JWT auth middleware.
Optional auth is expressed as `Option<axum::Extension<i64>>` for read endpoints that
need conditional access control (e.g. private repos).

## Owner Resolution

Many handlers must resolve a `username` path parameter to either a `User` or an `Organization`.
The pattern is: try user lookup first, then org. See `_wiki/owner-resolution.md` for
the full design rationale.
