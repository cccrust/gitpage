# run_docker.sh — Docker Production Runner

## Overview

`run_docker.sh` builds and runs Gitpage in a Docker container for production use. It handles the full lifecycle: building the base image (once), building the application image, preparing the data directory, and launching the container with proper volume mounts and port mappings.

## Execution Flow

### 1. Base Image Check

```bash
if ! docker image inspect "$BASE_IMAGE" >/dev/null 2>&1; then
  docker build -t "$BASE_IMAGE" -f Dockerfile.base .
fi
```

The base image (`gitpage-dev-base:latest`) contains all dev toolchains (Python, Rust, Node.js, SSH server). It is large (~1 GB) and slow to build (5–15 min first time). The script only builds it if it does not exist, so it is effectively a **one-time cost**.

### 2. App Image Build

```bash
if [ "$1" = "--build" ] || ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  docker build -t "$IMAGE" .
fi
```

The app image (`gitpage:latest`) is the multi-stage build from `./Dockerfile`. It copies artifacts from the Node and Rust stages into the base image. The script:

- Rebuilds unconditionally if `--build` is passed.
- Rebuilds if no image exists yet.
- Skips the build if the image already exists (fast startup).

### 3. Data Directory Preparation

```bash
mkdir -p "$DATA_DIR/repos" "$DATA_DIR/staging" "$DATA_DIR/apps"
```

Creates the host-side directories that will be mounted into the container. This avoids permission issues when Docker creates them automatically with root ownership.

### 4. Container Launch

```bash
docker run --rm --name gitpage \
  -p "$PORT:8080" \
  -p "$SSH_PORT:22" \
  -v "$DATA_DIR:/app/data" \
  -v "gitpage-ssh-keys:/etc/ssh" \
  -e RUST_LOG=info \
  -e SSH_USERS="alice:alice123,bob:bob123" \
  gitpage:latest
```

## Volume Persistence Patterns

### Bind Mount: `$DATA_DIR:/app/data`

| Property | Value |
|----------|-------|
| Host path | `$SCRIPT_DIR/data/` |
| Container path | `/app/data` |
| Contents | SQLite DB, bare repos (`repos/`), staging (`staging/`), apps (`apps/`) |
| Purpose | **Persistent state** — survives container restart, removal, and image update |

This is the most critical volume. If deleted, all repositories, user accounts, and app deployments are lost.

### Named Volume: `gitpage-ssh-keys:/etc/ssh`

| Property | Value |
|----------|-------|
| Volume name | `gitpage-ssh-keys` |
| Container path | `/etc/ssh` |
| Contents | SSH host keys (`ssh_host_*`) |
| Purpose | **Persistent SSH host keys** — prevent "WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED" on container restart |

Without this volume, every `docker run` would generate new host keys, causing SSH client warnings about host key mismatch. The named volume ensures keys are stable across restarts.

## Port Mapping Rationale

| Mapping | Purpose |
|---------|---------|
| `$PORT:8080` | HTTP API and frontend. Configurable via `PORT` env var (default `8080`). Mapping to 8080 avoids requiring root for privileged ports. |
| `$SSH_PORT:22` | SSH access. Configurable via `SSH_PORT` env var (default `2222`). Container's internal SSH daemon listens on port 22; the host maps it to a non-privileged port. |

The same container serves both HTTP and SSH — no separate SSH bastion is needed.

## Usage

```bash
# First run (builds everything):
./run_docker.sh

# Rebuild app image and start:
./run_docker.sh --build

# Custom ports:
PORT=9090 SSH_PORT=2222 ./run_docker.sh
```

## References

- `build_docker.sh` — Simplified build-only script (manual steps).
- `Dockerfile` — Multi-stage app image.
- `Dockerfile.base` — Base image with dev toolchains.
- `entrypoint.sh` — Container entrypoint (SSH key gen, user setup).
