# Gitpage — Agent Context

## Project Overview
Self-hosted Git platform with static Pages hosting, App hosting (like Vercel), Dropbox-style file manager, deploy logs, and SSH shell access.

- **Backend**: Rust (Axum + libgit2 + rusqlite), no ORM, no async runtime for git2
- **Frontend**: React 19 + TypeScript + Vite, no state management library
- **Auth**: JWT (jsonwebtoken crate) + argon2 password hashing
- **Database**: SQLite via rusqlite (single file: `data/gitpage.db`)
- **Git**: libgit2 (git2 crate), bare repos at `data/repos/{user}/{repo}.git`
- **Template engine**: None — backend serves JSON, frontend SPA at `/`

## Build & Run
```bash
# Backend
cargo build
cargo run

# Frontend (separate terminal, for dev)
cd frontend && npm run dev

# Frontend build (for production)
cd frontend && npm run build

# Test
./test.sh
./seed.sh      # Creates demo users + repos
```

## Key Backend Architecture

### Routes (`src/app.rs`)
- **Auth**: `POST /api/auth/register`, `POST /api/auth/login`, `GET /api/auth/me`, `PUT /api/auth/password`
- **Repos**: `GET/POST /api/repos`, `GET/DELETE /api/repos/:id`, `PUT /api/repos/:id`
- **Public**: `GET /api/users/:username/repos`, `GET /api/users/:username/profile` (GET + PUT)
- **Content**: `GET /api/:username/:repo_name/tree`, `/blob`, `/readme`, `/commits/:branch`
- **Pages**: `GET/PUT /api/pages/:repo_id`, `POST /api/pages/:repo_id/deploy`
- **Apps**: `GET/PUT/DELETE /api/apps/:repo_id`, `POST /api/apps/:repo_id/deploy`, `GET /api/apps/:repo_id/deploys[/:deploy_id]`
- **Files (staging)**: `GET /api/repos/:repo_id/tree|raw`, `PUT/DELETE /api/repos/:repo_id/files`, `POST mkdir|move|commit`
- **SSH Keys**: `GET/POST /api/repos/:repo_id/ssh-keys`, `DELETE /api/repos/:repo_id/ssh-keys/:key_id`
- **Search**: `GET /api/repos/search?q=&page=&page_size=`

### Fallback handler (order matters)
1. `/git/{user}/{repo}/{*path}` — Git HTTP Smart Protocol (push/pull/clone)
2. `/pages/{user}/{repo}/{*path}` — Static Pages hosting
3. `/apps/{repo_id}/{*path}` — Reverse proxy to running app processes
4. `/{path}` — Serve `static/` files, then SPA fallback (`index.html`)

### Database (`src/db/mod.rs`)
- Tables: `users`, `repositories`, `pages_config`, `apps_config`, `deploy_logs`, `ssh_keys`
- Migrations run automatically on startup
- All DB methods are async (wrapped in `tokio::sync::Mutex`)

### Error Handling (`src/utils/errors.rs`)
- Custom `AppError` enum with `BadRequest`, `Unauthorized`, `NotFound`, `Conflict`, `Internal`
- Error messages are in Chinese
- `AppError` implements `IntoResponse` for Axum

### Key Modules
| File | Purpose |
|------|---------|
| `src/auth/mod.rs` | JWT create/verify |
| `src/config.rs` | Config structs from `config.toml` |
| `src/deploy.rs` | App build/start lifecycle (subprocess) |
| `src/ssh.rs` | `regenerate_authorized_keys()` — writes `~/.ssh/authorized_keys` + `~/.ssh/gitpage-shell` |
| `src/git/mod.rs` | Low-level git2 operations (clone, fetch, log, read tree) |
| `src/app.rs` | Route setup + `fallback_handler` (Git smart, Pages proxy, App proxy, SPA) |
| `src/handlers/` | One file per domain (auth, repos, apps, pages, files, ssh_keys, content, git_smart) |

### Important Signatures
- `deploy_app(repo_path, workspace, apps_config) -> Result<(u16, String), AppError>` — returns (port, log)
- `commit_staging(username, repo, message) -> Result<(), AppError>` — builds git tree from staging dir
- `regenerate_authorized_keys(state: &AppState) -> Result<(), AppError>` — regenerates all SSH access

## Frontend Architecture

### Routes (React Router)
| Path | Page |
|------|------|
| `/` | Dashboard (repo list + search) |
| `/login`, `/register` | Auth |
| `/new` | Create repo |
| `/repo/:id` | Repo page (overview, clone URL, commits) |
| `/repo/:id/files` | File explorer |
| `/repo/:id/files/edit?path=` | File editor |
| `/repo/:id/pages` | Pages settings |
| `/repo/:id/app` | App settings |
| `/repo/:id/deploys[/:deployId]` | Deploy logs |
| `/repo/:id/settings` | Repo settings |
| `/repo/:id/ssh` | SSH key management |
| `/u/:username` | User profile |
| `/settings` | User settings (profile + password) |

### API Module (`src/api.ts`)
- `request<T>(method, path, body)` — generic fetch wrapper, injects JWT from `localStorage`
- Exports typed functions: `login`, `register`, `listRepos`, `getRepo`, `createRepo`, `deleteRepo`, `listTree`, `listCommits`, etc.
- All user-facing strings are in Chinese

### Components
- `Layout.tsx` — Top nav + bottom nav (mobile) + container
- `MarkdownView.tsx` — Renders markdown HTML
- `Spinner.tsx` — Unified loading indicator
- `Pagination.tsx` — Prev/Next pagination

## Key Design Decisions

### v0.6 — Direct Subprocess (no Docker)
App processes are spawned directly. `AppProcessManager` (`HashMap<RepoId, ProcessHandle>`) tracks running apps. Processes are lost on server restart (DB config persists).

### v0.7 — Dropbox-style File Manager
Files in `data/staging/{username}/{repo}/`. Staging is the working area; `commit_staging()` builds a git commit from staged files. Created/deleted alongside repos.

### v0.8 — Deploy Logs
`deploy_logs` table stores `(id, repo_id, status, started_at, finished_at, log_output)`. `deploy_app` returns `(port, log_str)`, callers persist the log.

### v0.9 — SSH Shell via OpenSSH
Each SSH key is bound to one repo. System OpenSSH handles connections; `~/.ssh/authorized_keys` has `command="/path/to/gitpage-shell"` restrictions. The `gitpage-shell` script `cd`s to the repo's staging dir. `regenerate_authorized_keys()` rewrites the entire file.

### v0.10 — Chinese Error Messages
All error messages unified to Chinese. Backend: `AppError` display + all handlers. Frontend: all catch-block messages in 16 pages.

### v0.11 — Search & UI Polish
- Search supports pagination (`page`/`page_size`)
- Search results include `username` (JOIN with users table)
- Clone URL shown on repo page
- `Spinner` + `Pagination` components
- Uniform loading states across all pages

## Version Roadmap
```
v0.1–0.9: Core features (complete)
v0.10:   Chinese errors + user settings (complete)
v0.11:   Search pagination + UI polish (complete)
v0.12:   Repo management + security config
v0.13:   README + API docs
v1.0:    Stable release (no Docker)
v1.1+:   Docker containerization
```

## Critical Context
- `data/` directory: `repos/` (bare git), `staging/` (working dirs), `apps/` (app workspaces), `db` and config live here
- `~/.ssh/authorized_keys` + `~/.ssh/gitpage-shell` are auto-managed (SSH)
- JWT secret in `config.toml` or env var `JWT_SECRET`
- `run.sh` starts both backend and frontend build
- Hot reload: `cargo run` for backend, `cd frontend && npm run dev` for frontend

## Testing
```bash
./test.sh    # Current test script
```
No formal test framework. Tests use bash + `set -x`.
