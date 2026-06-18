# `main.rs` — Application Startup Sequence

## Overview

`main.rs` is the single entry point for the Gitpage server. It follows a strictly ordered initialization sequence where each step depends on the output of the previous one. The design guarantees that all subsystems are fully initialized before the HTTP listener binds.

## Startup Sequence

### 1. Logging

`tracing_subscriber` is initialized with an `EnvFilter` that defaults to `info`. This must be first so all subsequent initialization steps produce structured logs. The filter can be overridden via the `RUST_LOG` environment variable for debugging.

### 2. Configuration Loading

The config file path defaults to `config.toml` and can be overridden via the first CLI argument. `Config::from_file()` reads and deserializes the TOML file into the typed `Config` struct (see `config.rs`). The config is wrapped in `Arc` for thread-safe sharing across the entire application.

### 3. Storage Directories

Four subdirectories are created under `storage.base_path`:
- `repos/` — bare Git repositories
- `apps/` — app build workspace directories
- `staging/` — file manager working trees

Creating these early ensures no filesystem errors occur when subsystems later try to write to them.

### 4. SSH Setup (if enabled)

When `[ssh] enabled = true`, the server writes a `gitpage-shell` script to `~/.ssh/`. This script is the SSH command handler that, when a user connects via SSH, places them in their repo's staging directory. It also optionally runs `SSH_ORIGINAL_COMMAND` for single-command execution. The script is only written if it does not already exist, preserving manual modifications. It is made executable on Unix systems.

This step also ensures `~/.ssh/` exists as a directory.

### 5. Database Initialization

`Database::new()` opens (or creates) the SQLite file at the configured path and enables WAL mode and foreign keys via pragmas. The connection is wrapped in `Arc<Mutex<Connection>>` for concurrent access.

Immediately after opening, `run_migrations()` runs all schema migrations idempotently. Migrations are inline SQL statements covering all tables: users, repositories, organizations, pages_config, apps_config, deploy_logs, ssh_keys, issues, issue_labels, pull_requests, stars, watches, access_tokens, repo_collaborators, repo_secrets, branch_protection. Column additions (forked_from, stars_count, forks_count, watch_count, port) are also applied with `ALTER TABLE` wrapped in `.ok()` since they may already exist.

A notable migration replaces the old `UNIQUE(user_id, name)` constraint on repositories with partial unique indexes for user-owned and org-owned repos separately, enabling the org re-ownership feature.

### 6. JWT and Encryption Initialization

JWT secret is resolved via `JwtConfig::effective_secret()`, which prefers the `JWT_SECRET` environment variable over the config file value. This allows secret rotation without modifying config files.

Two global `OnceLock` singletons are initialized:
- `JWT_SECRET` — used by `auth::create_token()` and `auth::verify_token()` for signing and validating JWTs
- `ENCRYPTION_KEY` — derived by SHA-256 hashing the config's `encryption_key` (or falling back to the JWT secret), used for AES-256-GCM encryption of repo secrets and other sensitive data

`OnceLock` is chosen over `lazy_static` or `OnceCell` because it can be initialized exactly once at startup and panics if used before initialization, catching ordering bugs at runtime.

### 7. App Process Manager

`AppProcessManager` is created with the configured port range (`port_range_start..port_range_end`). It manages:
- In-memory map of running app processes (keyed by repo_id)
- Atomic port allocator for sequential port assignment
- Thread-safe access via `tokio::sync::Mutex`

This manager is the single source of truth for what user apps are currently running.

### 8. Docker Initialization

If `[runtime] mode = "docker"`, a `DockerManager` is created by:
1. Connecting to the local Docker daemon via `bollard`
2. Pulling the base image if not present
3. Scanning existing `gitpage-*` containers to rebuild the SSH port allocation map

If Docker connection fails (daemon not running, permission denied), the server logs a warning and falls back to process mode rather than crashing. This graceful degradation allows the same config to work in both environments.

### 9. `restore_apps_on_startup()` Background Task

This is spawned as a non-blocking `tokio::spawn` task so the server can start serving immediately while apps are being restored. It:

1. Queries all enabled app configs (with user/repo owner names via JOIN)
2. For each app with a persisted port:
   - **Docker mode**: checks if the app is still running in the container; if yes, re-registers it in the process manager; if not, re-deploys
   - **Process mode**: skips restoration because subprocesses are lost on server restart (the user must redeploy manually)

This design corrects a limitation in earlier versions where restarted servers had no record of which apps were running and on which ports.

### 10. AppState Construction

All initialized components are assembled into the `app::AppState` struct:
- `db` — Database handle
- `config` — Shared config Arc
- `jwt_expires_hours` — Extracted from config for token creation
- `app_manager` — App process lifecycle manager
- `docker` — Optional Docker manager

`AppState` is `Clone` (all fields are Arc/Mutex wrappers or clones), which is required by Axum's `with_state()` method.

### 11. Router Creation and Server Start

`app::create_app(state)` builds the complete Axum router with all API routes, middleware, and fallback handler. The server binds to `host:port` using `tokio::net::TcpListener` and serves via `axum::serve`.

## Why This Sequence Matters

- **Config before everything** — all subsystems depend on config values (paths, ports, secrets)
- **Storage dirs before DB/SSH** — both may write to storage paths
- **DB before JWT** — JWT user data comes from DB queries
- **JWT before AppState** — the state exposes JWT config
- **AppManager before Docker** — Docker init may register fake app processes during restore
- **Restore after all systems** — restore needs DB, AppManager, Docker, and Config to be available
- **Server last** — no requests arrive before all subsystems are ready

## Reference

- `_doc/v0.1.md` — Initial architecture, JWT design
- `_doc/v1.0.md` — Stable startup flow
- `_doc/v1.2.md` — Docker runtime mode
- `_doc/v1.3.md` — Container resource limits and health checks
- `_doc/v0.6.md` — App hosting (process mode)
- `_doc/v0.9.md` — SSH shell script generation
- `_doc/dockerQA.md` — Docker design decisions
