# test_docker.sh — Integration Test (Inside Docker)

## Overview

`test_docker.sh` runs the integration test suite inside a Docker container. It builds the Docker images, starts a container with the Gitpage server, and executes the same API/git workflow as `test.sh` but against the containerized instance.

## Differences from `test.sh`

| Aspect | `test.sh` | `test_docker.sh` |
|--------|-----------|-------------------|
| Environment | Host machine | Docker container |
| Build | `cargo build` (debug) | `docker build` (release) |
| Data directory | `./data/` (preserved) | `/tmp/gptest-docker-data` (auto-cleaned) |
| Port | 8080 | 18080 |
| SSH keys volume | None | `gptest-ssh-keys` named volume |
| Base image | N/A | Built from `Dockerfile.base` |
| Org tests | Yes (22–38) | No (simplified) |

## Execution Flow

### 1. Build Images

```bash
docker build -t gitpage-dev-base:latest -f Dockerfile.base .
docker build -t gitpage:test .
```

The base image is built first (dev toolchains), then the app image. Both are cached for subsequent runs.

### 2. Start Container

```bash
docker run -d --name gptest-docker \
  -p 18080:8080 \
  -v /tmp/gptest-docker-data:/app/data \
  -v gptest-ssh-keys:/etc/ssh \
  -e RUST_LOG=info \
  gitpage:test
```

- Host port 18080 → container port 8080.
- Host temp directory mounted at `/app/data` for persistence.
- Named volume `gptest-ssh-keys` persists SSH host keys across container restarts.

### 3. Test Sequence (simplified from `test.sh`)

Tests 1–21 cover: register → login → me → create repo → list repos → public repos → git push → tree → blob → readme → commits → clone → push second commit → verify → push index.html → enable Pages → check config → serve Pages → redeploy → delete repo → auth rejection.

Notable omission: **No org tests** — the Docker test covers core functionality only.

### 4. Cleanup

```bash
cleanup() {
    docker rm -f gptest-docker
    docker volume rm gptest-ssh-keys
    rm -rf /tmp/gptest-docker-data
}
trap cleanup EXIT
```

**Isolated data**: The test uses `/tmp/gptest-docker-data` — a temp directory that is deleted on cleanup. This guarantees zero impact on the host's Gitpage data.

## Why This Exists

- **CI/CD validation**: Tests that the Docker image works correctly before deployment.
- **Environment consistency**: Eliminates "works on my machine" issues — the test runs in exactly the same environment as production.
- **Release qualification**: Validates the multi-stage build produced a functional image.

## Usage

```bash
# Requires Docker
./test_docker.sh
```

## References

- `AGENTS.md` — Testing section documents `./test_docker.sh`.
- `test.sh` — Host-based version with org tests.
- `test_docker_mode.sh` — Tests Docker runtime mode (per-user containers).
- `Dockerfile` — The image under test.
- `Dockerfile.base` — Base image with dev toolchains.
