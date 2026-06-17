# Gitpage — Agent Context

Self-hosted Git platform with Pages / App hosting, file manager, deploy logs, SSH shell. Like minimal GitHub/GitLab.

## Stack

- **Backend**: Rust (Axum + libgit2 + rusqlite) — no ORM, no async git2
- **Frontend**: React 19 + TypeScript + Vite — no state library
- **Auth**: JWT (jsonwebtoken crate, global `OnceLock`) + argon2
- **DB**: SQLite via rusqlite (`data/gitpage.db`), WAL mode, `tokio::sync::Mutex`
- **Git**: libgit2 for reading tree/blob/commit/log; system `git http-backend` subprocess for push/pull/clone

## Commands

```bash
cargo build                     # Backend
cargo run                       # Dev server on :8080
cargo run -- config.toml        # Dev server with explicit config
cd frontend && npm run dev      # Frontend HMR on :5173 (proxies /api, /git, /pages to :8080)
cd frontend && npm run build    # tsc -b && vite build
./run.sh                        # Production: build frontend + backend release, start
./test.sh                       # Integration test (deletes data/, starts fresh)
./seed.sh                       # Demo users (alice/alice123, bob/bob123) + repos
```

## Config (`config.toml`)

Sections: `[server]`, `[database]`, `[storage]`, `[jwt]`, `[ssh]`, `[cors]`, `[upload]`, `[apps]`, `[runtime]`, `[docker]`.

- `storage.base_path`: root of repos/staging/apps directories (default `"data"`)
- JWT secret: config value, overridable via `JWT_SECRET` env var
- SSH disabled by setting `[ssh] enabled = false`
- CORS: `allowed_origins = ["*"]` or specific origins
- Upload limit: `max_file_size` (default 10MB)
- App port range: `port_range_start` / `port_range_end`
- Runtime mode: `[runtime] mode = "process"` (default) or `"docker"`
- Docker config: `[docker] base_image`, `network`, `memory_limit`, `cpu_shares`

## Disk Layout

All paths are under `{storage.base_path}` (default `"data"`):

```
data/
├── gitpage.db              — SQLite database
├── repos/{u}/{r}.git       — Bare git repos
├── staging/{u}/{r}/        — File manager working tree
└── apps/{u}/{r}/           — App deploy workspace
```

- Config methods like `repo_path()`, `staging_path()`, `app_workspace_dir()` all derive from `storage.base_path`
- `pages_dir()` appends `/repos` for backwards consistency
- Git http-backend uses `{storage.base_path}/repos` as `GIT_PROJECT_ROOT`

## Route Fallback (order matters in `src/app.rs`)

1. `/git/{user}/{repo}/*` — git http-backend (push/pull)
2. `/pages/{user}/{repo}/*` — static pages hosting
3. `/app/{user}/{repo}/*` — reverse proxy to running app
4. `/*` — static files (frontend/dist/ → static/) → SPA fallback

## Key Backend Modules

| File | Role |
|------|------|
| `src/auth/mod.rs` | JWT create/verify, `JWT_SECRET` global `OnceLock` |
| `src/config.rs` | Config structs from `config.toml` |
| `src/deploy.rs` | App subprocess lifecycle (`AppProcessManager`) |
| `src/docker.rs` | Per-user container management (`DockerManager`) |
| `src/ssh.rs` | `regenerate_authorized_keys()` writes `~/.ssh/authorized_keys` + `~/.ssh/gitpage-shell` |
| `src/git/mod.rs` | libgit2 tree/blob/log + git http-backend spawn |
| `src/app.rs` | Routes + fallback handler + auto-deploy on git push |
| `src/handlers/` | One file per domain (auth, repos, content, pages, files, ssh_keys, apps, orgs) |
| `src/db/mod.rs` | All DB operations, migrations at startup |

## Repo Ownership: Users & Orgs

Repos have `owner_type` (`"user"` or `"org"`) and optional `org_id`. Content routes resolve `:username` against both users and orgs. Repository disk paths use the owner name: `data/repos/{owner}/{repo}.git`, `data/staging/{owner}/{repo}/`.

## Content Route Resolution

The `resolve_repo()` helper in `content.rs` tries user lookup first, then org. Returns `(Repository, owner_name)` where `owner_name` is the resolved user/org name used for filesystem paths.

## Org Features (v1.0.1)

- `organizations` + `organization_members` tables
- Org CRUD (`handlers/orgs.rs`)
- Member management (admin/member roles)
- Repo ownership by org (stored at `data/repos/{org}/{repo}.git`)
- SSH key management respects org admin permissions
- Auto-deploy uses `resolve_owner_and_repo` in `app.rs`
- Frontend pages: OrgList, OrgCreate, OrgDetail, OrgSettings, OrgMembers
- Routes: `/orgs`, `/org/:name`, `/org/:name/settings`, `/org/:name/members`

## Files: Staging, Not Direct Git

Staging area at `data/staging/{owner}/{repo}/`. `POST /api/repos/:repo_id/commit` builds a git tree + commit from staged files. The owner is resolved from the repo (user or org) before computing paths. Staging dirs created/deleted alongside repos.

## Frontend Notes

- All user-facing strings in Chinese
- `api.ts` has `request<T>(method, path, body)` — injects JWT from `localStorage`. Org API: `listMyOrgs`, `createOrg`, `getOrg`, `updateOrg`, `deleteOrg`, `listOrgRepos`, `listOrgMembers`, `addOrgMember`, `removeOrgMember`
- Routes defined in `App.tsx`, pages in `frontend/src/pages/`
- Components: `Layout.tsx` (top + bottom nav), `MarkdownView.tsx`, `Spinner.tsx`, `Pagination.tsx`

## Testing

```bash
./test.sh                # Integration test (no Docker)
./test_docker.sh         # Integration test inside Docker containers
./test_docker_mode.sh    # Docker runtime mode test (per-user containers, exec build/start)
```

All use `bash + set -x`, no test framework. Must not run concurrently with seed.sh.

## Docker

Two modes fully supported:

| Mode | Build | Run | Test |
|------|-------|-----|------|
| No Docker | `cargo build` | `cargo run` / `./run.sh` | `./test.sh` |
| Docker | `docker build` | `./run_docker.sh` | `./test_docker.sh` |

Docker **runtime mode** (`config.toml`: `[runtime] mode = "docker"`) creates per-user containers on registration via `ensure_user_container()` in `docker.rs`. Containers run `sleep infinity` and expose SSH port 22. Build/start/stop of user apps is delegated to container exec commands.

- `Dockerfile` — multi-stage: Node → Rust → Debian slim runtime
- `Dockerfile.base` — dev tooling image (Python, Rust, Node.js, uv, opencode)
- `run_docker.sh` — builds image, mounts `data/` volume, runs on `:8080` + SSH on `:2222`
- `test_docker.sh` — builds image, starts container on `:18080`, runs full test suite
- `test_docker_mode.sh` — tests per-user container creation, named volume, staging bind, container exec build/start
- `entrypoint.sh` — container entrypoint: generates SSH host keys, starts sshd, then gitpage
- `.dockerignore` — excludes build artifacts, git history, scripts

## Project Structure

```
src/main.rs            — Entry: config, DB, SSH script, app startup
src/app.rs             — Router + fallback handler (Git/Pages/App proxy)
data/
├── gitpage.db         — SQLite
├── repos/{u}/{r}.git  — Bare git repos
├── staging/{u}/{r}/   — File manager working tree
└── apps/{u}/{r}/      — App build workspace
_doc/                  — Version docs + API reference (api.md)
migrations/init.sql    — Stale; actual migrations run from src/db/mod.rs
config.toml            — All configuration
Dockerfile             — Multi-stage Docker build
.dockerignore          — Docker build context exclusions
entrypoint.sh          — Container entrypoint (sshd + gitpage)
run.sh                 — Prod start (no Docker): builds frontend + backend release
test.sh                — Integration test (no Docker)
seed.sh                — Demo data
run_docker.sh          — Docker build + run
test_docker.sh         — Integration test inside Docker
frontend/vite.config.ts— Dev proxy: /api, /git, /pages → :8080
```

## Gotchas

- `test.sh` preserves existing `data/` (no `rm -rf data`) — use `seed.sh` for fresh state
- `seed.sh` starts its own server if none running (deletes `data/` via `rm -rf data`)
- `test_docker.sh` uses isolated temp data dir (`/tmp/gptest-docker-data`), no impact on host
- `test_docker_mode.sh` uses `test_docker_mode_data` temp dir, no impact on host
- App processes are lost on server restart (DB config persists, subprocesses don't)
- SSH: `~/.ssh/authorized_keys` and `~/.ssh/gitpage-shell` are auto-managed — don't edit manually
- libgit2 errors are wrapped as `AppError::Internal` in Chinese
- Docker runtime mode containers run `sleep infinity` — must keep running for `docker exec` to work
- Config path methods (`staging_path`, `app_workspace_dir`) derive from `storage.base_path`; `repo_path` and `pages_dir` append `/repos`
