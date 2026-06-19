# Gitpage — Agent Context

Self-hosted Git platform with Pages / App hosting, file manager, deploy logs, SSH shell. Like minimal GitHub/GitLab.

## Stack

- **Backend**: Rust (Axum + libgit2 + rusqlite) — no ORM, no async git2
- **Frontend**: React 19 + TypeScript + Vite — no state library, no React Context
- **Auth**: JWT (jsonwebtoken crate, `OnceLock`, `JWT_SECRET` env override) + argon2
- **DB**: SQLite via rusqlite (`data/gitpage.db`), WAL mode, `tokio::sync::Mutex`
- **Git**: libgit2 for reading tree/blob/commit/log; system `git http-backend` subprocess for push/pull/clone

## Commands

```bash
cargo build                     # Backend
cargo run                       # Dev server on :8080
cargo run -- config.toml        # Dev server with explicit config
cd frontend && npm run dev      # Frontend HMR on :5173 (proxies /api, /git, /pages to :8080)
cd frontend && npm run build    # tsc -b && vite build
./run.sh                        # Production: build + release, start on :8080
./seed.sh                       # Demo users (alice/alice123, bob/bob123) + repos
./test.sh                       # Legacy wrapper → test/run_all.sh (integration tests)
./test_all.sh                   # Full suite: cargo test + integration + Hurl API + Playwright E2E
cd frontend && npm test         # Playwright E2E (headless)
cd frontend && npm run test:headed  # Playwright E2E (headed)
```

- `test.sh` preserves existing `data/` — use `seed.sh` for fresh state (deletes `data/`)
- `test_all.sh` requires `hurl-bin` for API tests (skipped if absent); needs `docker` for Docker tests
- All shell tests use `bash + set -x`, no test framework; must not run concurrently with `seed.sh`

## Config (`config.toml`)

Sections: `[server]`, `[database]`, `[storage]`, `[jwt]`, `[ssh]`, `[cors]`, `[upload]`, `[apps]`, `[runtime]`, `[docker]`, `[secrets]`.

- `storage.base_path`: root of repos/staging/apps directories (default `"data"`)
- JWT secret: config value, overridable via `JWT_SECRET` env var
- `[secrets] encryption_key`: used for encrypting repo secrets at rest; falls back to JWT secret if empty
- SSH disabled by `[ssh] enabled = false`
- CORS: `allowed_origins = ["*"]` or specific origins
- Upload limit: `max_file_size` (default 10MB)
- Runtime: `[runtime] mode = "process"` (default) or `"docker"`
- Docker: `[docker] ssh_port_range_start/ssh_port_range_end` fixed host ports per user

## Disk Layout

All paths under `{storage.base_path}` (default `"data"`):

```
data/
├── gitpage.db              — SQLite database
├── repos/{owner}/{r}.git   — Bare git repos
├── staging/{owner}/{r}/    — File manager working tree
└── apps/{owner}/{r}/       — App deploy workspace
```

Config methods (`repo_path()`, `staging_path()`, `app_workspace_dir()`) derive from `storage.base_path`; `pages_dir()` appends `/repos`. Git http-backend uses `{storage.base_path}/repos` as `GIT_PROJECT_ROOT`.

## Route Fallback (order matters in `src/app.rs`)

1. `/git/{user}/{repo}/*` — git http-backend (push/pull, auto-deploys pages+apps on push)
2. `/pages/{user}/{repo}/*` — static pages hosting
3. `/app/{user}/{repo}/*` — reverse proxy to running app
4. `/*` — static files (`frontend/dist/` → `static/`) → SPA fallback

## Key Backend Modules

| File | Role |
|------|------|
| `src/main.rs` | Entry: config, DB, SSH script, app startup, `restore_apps_on_startup()` |
| `src/app.rs` | Router + fallback handler + `resolve_owner_and_repo()` |
| `src/config.rs` | Config structs from `config.toml` |
| `src/auth/mod.rs` | JWT create/verify, encryption key init |
| `src/db/mod.rs` (2313 lines) | All DB operations, migrations at startup (`run_migrations()`) |
| `src/db/models.rs` | Data structs |
| `src/git/mod.rs` | libgit2 tree/blob/log + git http-backend spawn |
| `src/deploy.rs` | App subprocess lifecycle (`AppProcessManager`) |
| `src/docker.rs` | Per-user container management (`DockerManager`) |
| `src/ssh.rs` | `regenerate_authorized_keys()` writes `~/.ssh/authorized_keys` + `~/.ssh/gitpage-shell` |
| `src/utils/mod.rs` | Utility helpers |
| `src/handlers/` | One file per domain (see below) |

### Handlers

`auth`, `repos`, `content`, `files`, `pages`, `apps`, `git_smart`, `ssh_keys`, `orgs`, `issues`, `pulls`, `settings`, `stars`

All share: `async fn(State, Extension<user_id>, Path/Query/Json) -> Result<Json, AppError>`.

## Repo Ownership

Repos have `owner_type` (`"user"` or `"org"`) and optional `org_id`. `resolve_repo()` in `content.rs` tries user lookup first, then org. Returns `(Repository, owner_name)`.

## Staging, Not Direct Git

`POST /api/repos/:repo_id/commit` builds a git tree + commit from staged files at `data/staging/{owner}/{repo}/`. Owner resolved from repo (user or org) before computing paths.

## v2.0+ Features

| Area | Backend | Frontend (all unrouted) |
|------|---------|------------------------|
| Issues + Labels + Comments | `handlers/issues.rs` | `IssueList`, `IssueDetail`, `IssueNew` |
| Pull Requests + Merge + Diff | `handlers/pulls.rs` | `PRList`, `PRDetail`, `PRNew` |
| Stars / Watches | `handlers/stars.rs` | Inline in `RepoPage` |
| Forks | `handlers/repos.rs` | Inline in `RepoPage` |
| Access Tokens | `handlers/settings.rs` | `SettingsTokensPage` (routed) |
| Collaborators | `handlers/settings.rs` | `RepoSettingsCollaboratorsPage` (routed), `SettingsCollaborators` (unrouted) |
| Secrets | `handlers/settings.rs` | `RepoSettingsSecretsPage` (routed), `SettingsSecrets` (unrouted) |
| Branch Protection | `handlers/settings.rs` | `RepoSettingsBranchProtectionPage` (routed), `SettingsBranches` (unrouted) |

Several issue/PR frontend pages exist but are **unregistered in `App.tsx`** — unreachable through normal navigation.

## Testing

| Suite | Command | Location |
|-------|---------|----------|
| Rust unit tests | `cargo test` | `src/` |
| Integration (14 scripts) | `test/run_all.sh` (or `test.sh`) | `test/0*.sh` |
| Hurl API tests | `tests/run_api_tests.sh` (via `hurl-bin`) | `tests/api/*.hurl` |
| Playwright E2E | `cd frontend && npm test` | `frontend/e2e/specs/` |

`test_all.sh` orchestrates all four sequentially. Hurl tests authenticate via `curl` first, then pass `--variable` args to `hurl-bin`. `auth.hurl` runs separately (creates its own user).

## Docker

| Mode | Build | Run | Test |
|------|-------|-----|------|
| No Docker | `cargo build` | `cargo run` / `./run.sh` | `./test.sh` / `./test_all.sh` |
| Docker | `docker build` | `./run_docker.sh` | `./test_docker.sh` |
| Docker runtime | — | `[runtime] mode = "docker"` | `./test_docker_mode.sh` |

- `Dockerfile`: multi-stage (Node → Rust → Debian slim)
- `Dockerfile.base`: dev tooling image
- Process mode apps are **lost on restart**; Docker mode re-deploys via `restore_apps_on_startup()` in `main.rs`
- Docker runtime creates per-user containers (`sleep infinity`) with SSH port mapping from `ssh_port_range_start..=ssh_port_range_end`

## Gotchas

- `seed.sh` starts its own server if none running (kills port 8080, deletes `data/`)
- Docker test scripts use isolated temp data dirs (`/tmp/gptest-docker-data`, `test_docker_mode_data`) — no host impact
- SSH: `~/.ssh/authorized_keys` and `~/.ssh/gitpage-shell` are auto-managed — don't edit manually
- libgit2 errors wrapped as `AppError::Internal` (Chinese messages)
- All frontend UI strings in Chinese
- `api.ts` `request<T>(method, path, body)` injects JWT from `localStorage`
