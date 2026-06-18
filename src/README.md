# src/ — Backend Source Code

## Overview

The backend is a Rust web server built on Axum that implements a self-hosted Git platform. It provides REST APIs for authentication, repository management, file editing, static page hosting, app deployment, SSH access, organization management, issues, pull requests, and social features (stars/watches). The server also serves Git HTTP Smart Protocol via `git http-backend`, hosts static Pages, and reverse-proxies running user apps.

## File Map

| File | Purpose |
|------|---------|
| `main.rs` | Application entry point — loads config, initializes DB/JWT/SSH/Docker, starts HTTP server |
| `app.rs` | Axum router with all API routes, auth middleware, fallback handler (Git/Pages/App proxy/static/SPA), auto-deploy on push |
| `config.rs` | All configuration structs deserialized from `config.toml`; path helper methods for repos, staging, pages, apps |
| `deploy.rs` | App lifecycle management — build/start/stop subprocesses, port allocation, project type detection (Node.js/Rust) |
| `docker.rs` | Per-user Docker container management — create/start containers, exec build/start/stop commands, SSH port mapping |
| `ssh.rs` | Writes `~/.ssh/authorized_keys` with per-repo command restrictions and `~/.ssh/gitpage-shell` handler script |

## Subdirectories

### `auth/`
| File | Purpose |
|------|---------|
| `mod.rs` | JWT token creation and verification using `jsonwebtoken` crate; global `OnceLock` for secret and encryption key |

### `db/`
| File | Purpose |
|------|---------|
| `mod.rs` | All database operations (users, repos, orgs, issues, PRs, stars, SSH keys, deploy logs, settings); schema migrations at startup |
| `models.rs` | All Rust structs mapping to database tables and API request/response types |

### `git/`
| File | Purpose |
|------|---------|
| `mod.rs` | libgit2 operations (list refs, read file content, list directory, get README, deploy pages, commit staging, commit log); `git http-backend` subprocess invocation for push/pull |

### `handlers/`
| File | Purpose |
|------|---------|
| `mod.rs` | Module re-exports for all handler files |
| `auth.rs` | User register, login, me, change password, profile, SSH info endpoints |
| `repos.rs` | CRUD for repositories, search, fork, list public/user repos |
| `content.rs` | Browse repo tree, read file content, README, commit log via libgit2 |
| `pages.rs` | Get/update/trigger deploy for static Pages hosting |
| `apps.rs` | Get/update/delete/trigger deploy for App hosting; list deploy logs |
| `files.rs` | File manager over staging area — read/write/delete/mkdir/move/status/commit |
| `git_smart.rs` | Serve static Pages files from the filesystem with MIME detection |
| `ssh_keys.rs` | Add/list/delete SSH keys per repo; regenerate `authorized_keys` |
| `orgs.rs` | CRUD for organizations; member management (add/remove/list) |
| `issues.rs` | List/create/get/update/delete issues, labels, comments |
| `pulls.rs` | List/create/get/update/merge pull requests, diff viewing |
| `settings.rs` | Access tokens, repo collaborators, secrets (AES-GCM encrypted), branch protections |
| `stars.rs` | Star/unstar, watch/unwatch repos; stargazer/watch status |

### `templates/`
| File | Purpose |
|------|---------|
| (empty) | Reserved for future server-side template rendering |

### `utils/`
| File | Purpose |
|------|---------|
| `mod.rs` | Module re-exports |
| `errors.rs` | `AppError` enum (NotFound, Unauthorized, BadRequest, Internal, Conflict) with Axum `IntoResponse` impl; `From` conversions for `rusqlite`, `git2`, `std::io` errors |

## Reference

For theoretical background on core subsystems, see the `_doc/` directory:
- `v0.1.md` — JWT auth, Axum router, Git HTTP backend
- `v0.3.md` — Pages hosting
- `v0.6.md` — App hosting (subprocess mode)
- `v0.7.md` — File manager staging area
- `v0.9.md` — SSH shell access
- `v1.0.md` — Stable release architecture
- `v1.2.md` — Docker container mode
- `v2.0.md` — Issues + Pull Requests
- `v2.1.md` — Settings (tokens, secrets, collaborators, branch protection)
- `v2.2.md` — Star/Watch/Fork social features
- `api.md` — Full REST API reference
