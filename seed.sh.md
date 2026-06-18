# seed.sh — Demo Data Seeding Script

## Overview

`seed.sh` populates Gitpage with demo users, repositories, and content. It is used for development, testing, and demonstration purposes. It starts its own server instance (if one is not already running), creates everything through the REST API, and cleans up.

## Why It Starts Its Own Server

`seed.sh` interacts with Gitpage exclusively through the HTTP API (`/api/auth/register`, `/api/repos`, etc.). This requires a running server. Rather than requiring the user to start one manually:

1. It checks if a server is already listening on `http://localhost:8080/api/auth/me`.
2. If not, it kills anything on port 8080, **deletes the `data/` directory**, and starts a fresh `cargo run --release` in the background.
3. Waits 4 seconds for startup, then proceeds.

This means **running `./seed.sh` destroys existing data** — it deletes `data/` to ensure a clean state.

## Execution Flow

### 1. Helpers

```bash
api(method, path, token, body)    # curl wrapper with auth header
login_or_register(user)           # tries login first, falls back to register
repo(name, desc, private)         # creates a repo using current $TOKEN
push(user, repo, filename, msg)   # inits a local git repo, pushes via http
```

`login_or_register` is notable: it tries login first so the script is idempotent — running it twice won't fail on duplicate registration.

### 2. User: alice

| Field | Value |
|-------|-------|
| Username | `alice` |
| Password | `alice123` |
| Repos | `blog` (public), `dotfiles` (public), `secret-project` (private) |

- Pushes `README.md` to `blog` via git push.
- Creates a full Rust project structure in `secret-project` (Cargo.toml, src/main.rs).
- Adds a demo SSH key to `blog`.

### 3. User: bob

| Field | Value |
|-------|-------|
| Username | `bob` |
| Password | `bob123` |
| Repos | `my-notes` (public), `portfolio` (public, Pages enabled) |

- Pushes Markdown notes content to `my-notes`.
- Creates an `index.html` portfolio site.
- Enables Gitpage Pages on `portfolio` via `PUT /api/pages/5` (repo ID 5 — bob's second repo).

### 4. Cleanup

```bash
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT
```

A temporary working directory at `/tmp/gpseed` is used for git init/push operations. It is removed when the script exits.

## Output

After completion, the script prints a summary:

```
Users:
  alice / alice123
  bob   / bob123

Repos:
  alice/blog            (public, SSH key added)
  alice/dotfiles        (public)
  alice/secret-project  (private)
  bob/my-notes          (public)
  bob/portfolio         (public, Pages enabled)

Pages: http://localhost:8080/pages/bob/portfolio/
```

## Key Design Decisions

- **Uses the HTTP API, not direct DB inserts** — this validates the full registration and repo creation paths.
- **Starts its own server** — removes the "must be running" prerequisite at the cost of a `rm -rf data` hard reset.
- **Idempotent login** — `login_or_register` enables re-running without registration errors.
- **Temporary work directory** — avoids polluting the project tree with demo git repos.
- **SSH key demo** — generates and adds an Ed25519 key as a real-world example.

## References

- `AGENTS.md` — documents `./seed.sh` as "Demo users + repos".
- `test.sh` — preserves existing `data/` unlike seed.sh.
- `run.sh` — how to start the server manually before seeding.
