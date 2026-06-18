# Dockerfile — Multi-Stage Build

## Overview

The Dockerfile uses a **three-stage build** to produce a minimal runtime image for Gitpage. This keeps the final image small by excluding build toolchains (Rust, Node.js) from the production layer.

## Stage 1: `frontend` — Node.js build

- **Base**: `node:22-bookworm-slim`
- **Workdir**: `/app/frontend`
- **Steps**:
  1. Copy only `package.json` + `package-lock.json` and run `npm ci` — this layer is cached until dependencies change.
  2. Copy the rest of the frontend source.
  3. Run `npx vite build` to produce static assets in `frontend/dist/`.
- **Output**: The `dist/` directory is consumed by Stage 2 (for embedding) and Stage 3 (for serving).

## Stage 2: `backend` — Rust build

- **Base**: `rust:1-slim-bookworm`
- **Workdir**: `/app`
- **Key optimization — dependency caching**:
  1. Install system build deps (`pkg-config`, `libssl-dev`, `cmake`) — cached as a single layer.
  2. Copy `Cargo.toml` + `Cargo.lock`, create a dummy `src/main.rs`, and run `cargo fetch` + `cargo build --release`. This **downloads and compiles all crate dependencies** into a cached layer. The dummy source is then removed.
  3. Copy the real `src/` and the frontend `dist/` from Stage 1.
  4. `touch src/main.rs` and run the final `cargo build --release`. Because only `src/` changed (not `Cargo.toml`), cargo reuses the cached dependency artifacts and only compiles the application code.
- **Outcome**: Iterative rebuilds (code changes only) skip the expensive dependency compilation step.

## Stage 3: `runtime` — Debian slim

- **Base**: `gitpage-dev-base:latest` (built from `Dockerfile.base`)
- **Installation**: This base image contains:
  - `git`, `openssh-server`, `openssh-client` — for SSH access and git operations.
  - `nodejs`, `npm` — for running Node-based apps deployed by users.
  - Python 3.12 + `uv` — for Python-based user apps.
  - Rust toolchain (via `rustup`) — for building user apps inside the container.
  - `opencode` CLI — for interactive development.
- **Artifacts copied from earlier stages**:
  - `/app/target/release/gitpage` from Stage 2 → `/usr/local/bin/gitpage`
  - `/app/frontend/dist/` from Stage 1 → `/app/frontend/dist/`
  - `config.toml` → `/app/config.toml`
  - `entrypoint.sh` → `/entrypoint.sh`
- **Runtime configuration**:
  - `WORKDIR /app`
  - `VOLUME ["/app/data"]` — persistent data (SQLite DB, bare repos, staging, apps).
  - `EXPOSE 22 8080` — SSH and HTTP ports.
  - `ENTRYPOINT ["/entrypoint.sh"]` — the entrypoint generates SSH host keys at runtime (they are not baked into the image).
- **Image size**: The final image is ~700 MB (Debian slim + runtime deps + all toolchains) but crucially does **not** contain the Rust or Node build artifacts from Stages 1 and 2.

## Why multi-stage?

- **Separation of concerns**: Each stage has only what it needs.
- **Minimal final image**: Build toolchains (rustc, cargo, npm, node build deps) are not in the runtime layer.
- **Layer caching efficiency**: Dependency layers only rebuild when `Cargo.lock` or `package-lock.json` changes, not on every source edit.
- **Deterministic builds**: The final image is self-contained with all runtime dependencies explicitly declared.

## Building

```bash
# First time (downloads toolchains into base image):
docker build -t gitpage-dev-base:latest -f Dockerfile.base .
docker build -t gitpage:latest .

# Subsequent code-only changes:
docker build -t gitpage:latest .   # reuses cached layers
```

## References

- `Dockerfile.base` — base image with all dev toolchains pre-installed.
- `entrypoint.sh` — container entrypoint (SSH key generation, user setup, sshd start).
