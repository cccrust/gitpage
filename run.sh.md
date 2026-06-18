# run.sh — Production Startup Script

## Overview

`run.sh` is the simplest way to build and start Gitpage from source on a fresh machine. It performs a full build of both frontend and backend, then starts the server on port 8080.

## Execution Flow

### 1. Port Cleanup

```bash
lsof -ti tcp:$PORT | xargs kill -9 2>/dev/null || true
sleep 1
```

Kills any existing process holding port 8080. This makes the script idempotent — running it twice in quick succession works cleanly.

### 2. Frontend Build

```bash
cd frontend
npm install --silent 2>/dev/null
npx vite build
cd ..
```

- `npm install` (silenced) ensures dependencies are installed.
- `npx vite build` produces the production static bundle in `frontend/dist/`.
- The Rust build later embeds these static files into the binary via `include_dir!()` or the backend serves them from the filesystem.

### 3. Backend Build

```bash
cargo build --release 2>&1 | tail -3
```

Compiles the Rust backend in release mode. Only the last 3 lines of output are shown to keep the log concise.

### 4. Server Start

```bash
exec cargo run --release
```

`exec` replaces the shell process so the binary handles signals directly. The server binds to `0.0.0.0:8080` (configured in `config.toml`).

## Why Build Both Every Time

Unlike a CI/CD pipeline where pre-built artifacts are deployed, `run.sh` is designed for:

- **Quick local deployment**: Clone the repo, run `./run.sh`, get a running instance.
- **Development iteration**: Changing frontend or backend code is immediately reflected on next run.
- **Fresh machine setup**: No assumptions about pre-existing binaries or Docker.

The cost is a ~1-2 minute build each time. For zero-downtime production, use the Docker workflow (`run_docker.sh`) or set up a CI pipeline.

## Use Cases

| Scenario | Recommendation |
|----------|---------------|
| First-time setup on a dev machine | `./run.sh` |
| Quick demo / testing | `./run.sh` |
| Production with CI | Docker (see `run_docker.sh`) |
| Production without Docker | Modify to build once, deploy binary |

## Notes

- Requires Rust toolchain and Node.js to be installed on the host.
- Uses `cargo run --release`, not `cargo run` — the release binary is orders of magnitude faster.
- The `config.toml` in the project root is used; modify it before running to change ports, DB path, etc.
- Static files from `frontend/dist/` are served by the backend; the backend uses a SPA fallback.

## References

- `AGENTS.md` — Commands section documents `./run.sh` as production startup.
- `config.toml` — All server configuration.
- `Dockerfile` — Containerized alternative with multi-stage build.
