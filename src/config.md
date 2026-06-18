# Configuration Module

## Overview

The `config.rs` module defines the complete configuration structure for Gitpage. It reads a TOML-formatted configuration file at startup and deserializes it into strongly-typed Rust structs using `serde`. The configuration covers every subsystem: server networking, database path, storage layout, authentication, SSH, CORS, upload limits, application hosting, runtime mode, Docker settings, and encryption secrets.

## TOML Configuration Parsing

### Why TOML?

Gitpage uses TOML (Tom's Obvious, Minimal Language) for its configuration file because:

1. **Readability**: TOML is designed to be unambiguous and easy to read, making it suitable for hand-edited configuration files
2. **Section structure**: The `[section]` hierarchy maps naturally to nested Rust structs
3. **serde integration**: The `toml` crate works seamlessly with `serde::Deserialize`, requiring no manual parsing logic
4. **Type safety**: Numbers, booleans, strings, and arrays are distinguished syntactically, preventing type confusion

### Parsing Flow

```
config.toml (file on disk)
        │
        ▼
  fs::read_to_string()
        │
        ▼
  toml::from_str::<Config>()
        │
        ▼
  Config struct (fully validated in memory)
```

The `from_file()` method reads the entire file and panics on any parse error. This is intentional: a server with an invalid configuration should not start at all rather than start in a broken state.

### Serde Attributes

The struct definitions use `#[serde(default)]` on sections that have sensible defaults. Sections without this annotation (like `[server]`, `[database]`, `[storage]`, `[jwt]`) are **required** — the parser will fail if they are missing. This design ensures that critical configuration is never accidentally omitted.

## Config Sections and Their Roles

### `[server]` — Network Bind

Defines the host and port for the HTTP server. The host defaults to `127.0.0.1` in development but should be `0.0.0.0` in production to accept external connections.

### `[database]` — SQLite Path

The single `path` field points to the SQLite database file. Gitpage uses WAL mode for concurrent read performance. The path is relative to the working directory unless an absolute path is given.

### `[storage]` — Data Root

The `base_path` setting is the **single root directory** for all Gitpage data files. See the dedicated section below for why all paths derive from this one value.

### `[jwt]` — Authentication

Contains the JWT signing secret and token expiration duration. The `effective_secret()` method checks the `JWT_SECRET` environment variable first, falling back to the config file value.

### `[ssh]` — SSH Shell

Controls whether the SSH shell (`gitpage-shell`) is enabled. SSH is enabled by default, serving git operations over SSH protocol.

### `[cors]` — Cross-Origin Requests

Defines `allowed_origins` for CORS headers. Defaults to `["*"]` which allows all origins — suitable for development but should be locked down in production.

### `[upload]` — File Limits

Sets `max_file_size` for the file manager uploads. Defaults to 10 MB. This prevents resource exhaustion from maliciously large file uploads.

### `[apps]` — App Hosting

Controls the port range for user-deployed applications (`port_range_start` to `port_range_end`). Defaults to 4000–65535. Applications bind to these ports on localhost.

### `[runtime]` — Execution Mode

Selects between `"process"` (default) and `"docker"` execution modes. Process mode runs user applications as child processes on the host; Docker mode runs them inside per-user containers.

### `[docker]` — Container Settings

When runtime mode is Docker, this section specifies the base image, network mode, resource limits (memory, CPU shares), and SSH port range for per-user containers.

### `[secrets]` — Encryption

Contains the `encryption_key` used for AES-256-GCM encryption of CI/CD secrets. If left empty, the JWT secret is used as a fallback (not recommended for production).

## Path Helper Methods

All path methods derive paths from `storage.base_path`, ensuring consistency across the entire codebase.

| Method | Path Pattern | Purpose |
|--------|-------------|---------|
| `repo_path()` | `{base}/repos/{user}/{repo}.git` | Bare git repository for push/pull |
| `user_repos_path()` | `{base}/repos/{user}` | All repos belonging to a user |
| `pages_dir()` | `{base}/repos/{user}/{repo}/pages` | Static pages output directory |
| `app_workspace_dir()` | `{base}/apps/{user}/{repo}` | App build workspace |
| `working_tree_path()` | `{base}/{user}` | Legacy working tree (rarely used) |
| `staging_path()` | `{base}/staging/{user}/{repo}` | File manager staging area |

### Why `{base}/repos` appears in both `repo_path` and `pages_dir`

The `repo_path` returns the bare git repository on disk (a `.git` directory), while `pages_dir` returns a subdirectory within the same logical path where static pages are extracted. This convention means that for every bare repo at `repos/{user}/{repo}.git`, the deployable pages live at `repos/{user}/{repo}/pages/`. The `pages_dir` is a logical subdirectory of the repo's user namespace, not the git object store.

## Environment Variable Override Pattern

The `effective_secret()` method on `JwtConfig` implements a standard override pattern: check for an environment variable first, and if not set, fall back to the config file value. This pattern allows:

- **Development convenience**: A default value in `config.toml` works out of the box
- **Production security**: The actual secret can be injected via environment variable or secrets manager without ever touching a file on disk
- **Runtime immutability**: The environment variable is read once at startup and stored in `OnceLock`, not queried on every request

## Why `storage.base_path` Is the Root

The `storage.base_path` acts as a single configuration point for all data directories. This design has several advantages:

1. **Portability**: Changing one value relocates all data — useful for switching between local development (`./data`) and production (`/var/lib/gitpage`)
2. **Backup simplicity**: A single directory tree contains repos, staging, apps, and the database
3. **Permission management**: One directory needs to be writable by the gitpage process
4. **Docker bind mounts**: A single volume mount in `docker-compose.yml` captures all persistent state

The directory layout under `base_path` is:

```
{base_path}/
├── gitpage.db          — SQLite database (at base_path root)
├── repos/              — Bare git repos for push/pull
├── staging/            — File manager working trees
└── apps/               — App build workspaces
```

Note that the database path is configured separately in `[database] path`, so the database can be placed outside the base path if desired (e.g., on a dedicated volume for performance).

## Related Wiki Pages

- [_wiki/axum.md](../../_wiki/axum.md) — How the Axum framework uses `Config` for app state
- [_wiki/rusqlite.md](../../_wiki/rusqlite.md) — Database connection configured from `[database]`
- [_wiki/wal-mode.md](../../_wiki/wal-mode.md) — SQLite WAL mode enabled at startup
- [_wiki/onceLock-init.md](../../_wiki/onceLock-init.md) — How config values seed global `OnceLock` variables
- [_wiki/process-vs-docker.md](../../_wiki/process-vs-docker.md) — How `[runtime] mode` selects execution backends
- [_wiki/owner-resolution.md](../../_wiki/owner-resolution.md) — How `resolve_repo()` uses config paths for disk access
