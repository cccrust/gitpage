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
cd frontend && npm run dev      # Frontend HMR on :5173 (proxies /api, /git, /pages to :8080)
cd frontend && npm run build    # tsc -b && vite build
./run.sh                        # Production: build frontend + backend release, start
./test.sh                       # Integration test (deletes data/, starts fresh)
./seed.sh                       # Demo users (alice/alice123, bob/bob123) + repos
```

## Config (`config.toml`)

Sections: `[server]`, `[database]`, `[storage]`, `[jwt]`, `[ssh]`, `[cors]`, `[upload]`, `[apps]`.

- JWT secret: config value, overridable via `JWT_SECRET` env var
- SSH disabled by setting `[ssh] enabled = false`
- CORS: `allowed_origins = ["*"]` or specific origins
- Upload limit: `max_file_size` (default 10MB)
- App port range: `port_range_start` / `port_range_end`

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
| `src/ssh.rs` | `regenerate_authorized_keys()` writes `~/.ssh/authorized_keys` + `~/.ssh/gitpage-shell` |
| `src/git/mod.rs` | libgit2 tree/blob/log + git http-backend spawn |
| `src/app.rs` | Routes + fallback handler + auto-deploy on git push |
| `src/handlers/` | One file per domain |
| `src/db/mod.rs` | All DB operations, migrations at startup |

## Files: Staging, Not Direct Git

Staging area at `data/staging/{user}/{repo}/`. `POST /api/repos/:repo_id/commit` builds a git tree + commit from staged files. Staging dirs created/deleted alongside repos.

## Frontend Notes

- All user-facing strings in Chinese
- `api.ts` has `request<T>(method, path, body)` — injects JWT from `localStorage`
- Routes defined in `App.tsx`, pages in `frontend/src/pages/`
- Components: `Layout.tsx` (top + bottom nav), `MarkdownView.tsx`, `Spinner.tsx`, `Pagination.tsx`

## Testing

```bash
./test.sh    # bash + set -x, no test framework, deletes data/ and starts fresh
```
Test removes `data/` and kills existing `gitpage` processes. Must not run concurrently with seed.sh.

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
run.sh                 — Prod start: builds frontend + backend release
seed.sh                — Demo data
test.sh                — Integration test
frontend/vite.config.ts— Dev proxy: /api, /git, /pages → :8080
```

## Gotchas

- `test.sh` deletes `data/` at start — don't run with active data
- `seed.sh` starts its own server if none running (also deletes `data/` via `rm -rf data`)
- App processes are lost on server restart (DB config persists, subprocesses don't)
- SSH: `~/.ssh/authorized_keys` and `~/.ssh/gitpage-shell` are auto-managed — don't edit manually
- libgit2 errors are wrapped as `AppError::Internal` in Chinese
