# test_docker_mode.sh — Docker Runtime Mode Test

## Overview

`test_docker_mode.sh` tests Gitpage's **Docker runtime mode** (`[runtime] mode = "docker"` in config.toml). In this mode, each registered user gets a dedicated Docker container (running `sleep infinity`) for building and running their deployed apps. This is the most complex test suite — it validates container lifecycle, SSH port allocation, app build/start, and state restoration.

## What It Tests

### 1. Per-User Container Creation

When a user registers (`POST /api/auth/register`), Gitpage's `docker.rs` calls `ensure_user_container()` which:

- Creates a named Docker container (`gitpage-{username}`).
- Publishes SSH port 22/tcp to a unique host port from the configured range (22500–22599).
- Creates a named volume (`gitpage-home-{username}`) for the user's home directory.
- Binds the staging directory from the host.

**Test**: Verifies the container is `running` after registration.

### 2. SSH Port Isolation

Each user gets a different host port mapped to container port 22. This enables concurrent SSH access to different user containers.

**Test**: Verifies the port is:
- Published (not empty).
- Within the configured range.
- Different for two different users (`test` vs `alice`).

### 3. Named Volume Persistence

The user's home directory is a Docker named volume — it survives container removal and recreation.

**Test**: Verifies `gitpage-home-test` volume exists.

### 4. Staging Bind Mount

The `data/staging/{user}/{repo}/` directory on the host is bind-mounted into the container at `/workspace/`. This is how app source code reaches the container for builds.

**Test**: `docker exec` verifies `/workspace/` is accessible.

### 5. Container Exec Build/Start

When a user enables an app (`PUT /api/apps/{id}`), Gitpage runs:
- `docker exec {container} {build_command}` (e.g., `npm install`)
- `docker exec {container} {start_command}` (e.g., `node server.js`)

**Test**: Creates a Node.js app (`server.js`), enables it, verifies the process runs inside the container.

### 6. App Proxy

After the app starts, Gitpage's reverse proxy (`/app/{user}/{repo}/`) forwards requests to the container's IP and assigned port.

**Test**: `curl /app/test/myapp/` returns `Hello from container\n`.

### 7. Restore on Startup

When the Gitpage server restarts, `restore_apps_on_startup()` in `main.rs`:
- Re-discovers running user containers via Docker API.
- Re-allocates SSH port mappings from running containers.
- Restarts apps whose repository still exists.

**Test**:
- Kills the Gitpage server process.
- Waits for clean shutdown.
- Restarts the server.
- Polls the API until `GET /api/apps/{id}` returns status `"running"`.
- Verifies the app proxy still works.

## Execution Flow

### Phase 1: Setup

1. **Backup config.toml**, write a test-specific config with `[runtime] mode = "docker"`.
2. **Build backend** with `cargo build` (debug).
3. **Ensure Docker is running** — fail fast if not.
4. **Ensure base image exists** — build `gitpage-dev-base:latest` if missing.

### Phase 2: Test Registration → Container

1. Start Gitpage with the Docker mode config.
2. Register user `test` — triggers `ensure_user_container()`.
3. Inspect container: verify SSH port published, status running.
4. Verify named volume exists.

### Phase 3: Test App Lifecycle

1. Create repo `myapp` via API.
2. Push a Node.js app (`package.json` + `server.js`).
3. Enable app via API (build: `npm install`, start: `node server.js`).
4. Wait 8 seconds for build + start.
5. Verify `node` process runs inside the container.
6. Verify container IP is assigned.
7. Verify API tree endpoint works.

### Phase 4: Restart & Restore

1. Kill the server (`kill $SERVER_PID`).
2. Wait for clean shutdown (`pkill -f gitpage`).
3. Restart the server.
4. Poll API for app status → expect `"running"`.
5. Hit the app proxy → expect HTTP 200 with `"Hello from container"`.

### Phase 5: Second User Isolation

1. Clean up the test user's container and volume.
2. Register user `alice`.
3. Verify Alice's container has a **different** SSH port than the test user had.

### Cleanup

```bash
cleanup() {
    kill $SERVER_PID
    docker rm -f gitpage-test gitpage-alice
    docker volume rm gitpage-home-test gitpage-home-alice
    rm -rf /tmp/gptest-docker-mode-data
    mv config.toml.bak config.toml   # restore original config
}
```

The cleanup restores the original `config.toml` from the backup.

## Isolation

- **Data directory**: `/tmp/gptest-docker-mode-data` — temp directory, deleted on cleanup.
- **Config**: Backup and restore of `config.toml`.
- **Containers**: Explicitly named (`gitpage-test`, `gitpage-alice`), removed on cleanup.
- **Volumes**: Named (`gitpage-home-test`, `gitpage-home-alice`), removed on cleanup.

## Usage

```bash
# Requires Docker daemon running
./test_docker_mode.sh
```

## References

- `AGENTS.md` — Docker section and testing documentation.
- `test.sh` / `test_docker.sh` — Other test scripts.
- `src/docker.rs` — Docker container management implementation.
- `src/main.rs` — `restore_apps_on_startup()`.
- `src/deploy.rs` — App process lifecycle (process mode equivalent).
