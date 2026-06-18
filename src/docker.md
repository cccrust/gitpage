# Docker Module

## Overview

The `docker.rs` module implements the Docker runtime mode for Gitpage, where each registered user gets a dedicated Docker container. User applications are built and executed inside these containers via `docker exec`, providing process isolation, consistent build environments, and SSH access.

The central type is `DockerManager`, which wraps the Bollard Docker Engine API client and manages the lifecycle of per-user containers, SSH port allocations, and exec-based command execution for building, starting, stopping, and monitoring applications.

## Bollard Crate for Docker Engine API

### What Is Bollard?

Bollard is a pure-Rust asynchronous client for the Docker Engine API. It communicates with the Docker daemon over its REST API (typically via Unix socket at `/var/run/docker.sock`). Unlike shelling out to the `docker` CLI, Bollard operates without forking subprocesses, making it significantly faster for high-frequency operations like exec commands and container queries.

### Why Bollard Instead of Docker CLI?

| Factor | Bollard (HTTP API) | Docker CLI (subprocess) |
|--------|-------------------|----------------------|
| Latency | ~5-50ms per call | ~80-200ms (includes fork/exec overhead) |
| Error handling | Typed Rust errors | String parsing of CLI output |
| Streaming | Native async streams | Piped stdout/stderr |
| Type safety | Full API types from `bollard::models` | Stringly-typed JSON |
| Connection | Direct Unix socket | Goes through Docker CLI → Unix socket |

### Key Bollard APIs Used

- `create_container` / `start_container` — Container lifecycle
- `list_containers` — Query existing gitpage containers (with name filters)
- `create_exec` / `start_exec` — Execute commands inside running containers
- `stop_container` / `remove_container` — Cleanup

### Connection and Initialization

`DockerManager::connect()` performs three steps at startup:

1. Connect to the local Docker daemon via `Docker::connect_with_local_defaults()`
2. Pull the configured base image (if not already present locally) via `create_image`
3. Rebuild in-memory SSH port mappings from any existing `gitpage-*` containers

The connection uses the default Docker socket path — `/var/run/docker.sock` on Linux, `~/.docker/run/docker.sock` on macOS. No authentication or TLS is handled, as Gitpage assumes the Docker daemon is on the same host.

## Per-User Container Lifecycle

### Container Naming Convention

Each user's container is named `gitpage-{username}`. This naming convention is used consistently across all Docker API calls and is how containers are discovered during startup recovery.

### Creation (`ensure_user_container`)

```
ensure_user_container("alice")
        │
        ├── list_containers(filters: name=gitpage-alice)
        │       │
        │       ├── Container exists + running → return (record SSH port)
        │       ├── Container exists + stopped → start_container()
        │       └── Container does not exist → create_container()
        │
        └── (if creating)
            ├── find_free_port()
            ├── generate_password(12)
            ├── build ContainerCreateBody with:
            │   ├── image, cmd: ["sh", "-c", "useradd ...; sleep infinity"]
            │   ├── port bindings (22 → host_port)
            │   ├── bind mounts (workspace, home volume)
            │   └── resource limits (memory, cpu)
            ├── create_container()
            ├── start_container()
            └── record SSH port + password in memory
```

The container runs `sleep infinity` as its primary process — it is designed to stay alive indefinitely, not to run a specific application. All application execution happens through `docker exec` against this persistent container.

### Container Removal

`remove_container()` stops (if running) and removes the container, cleaning up its volumes (`v: true`). It also removes the user's SSH port and password from the in-memory maps.

### Lifecycle States

```
Non-existent → Created → Running ←→ Stopped
                              │
                         (exec commands)
```

## SSH Port Allocation and Management

### Why Expose SSH?

Each container runs an SSH server (sshd, started at container boot) mapped to a unique host port. This allows users to SSH directly into their container for debugging, inspecting files, or running ad-hoc commands — a feature that distinguishes Gitpage from platforms like Heroku or Vercel.

### Port Allocation Algorithm

1. Maintain a `HashMap<String, u16>` mapping usernames to allocated host ports
2. When allocating for a user, collect the set of all ports currently assigned to *other* users
3. Iterate through the configured `ssh_port_range_start..=ssh_port_range_end`
4. Return the first port not in the used set
5. If the range is exhausted, fall back to `ssh_port_range_start` (this is a bounded-error edge case)

### Password Management

SSH passwords are randomly generated (12 characters, alphanumeric lowercase) using `OsRng` for cryptographically secure randomness. Passwords are stored in a `HashMap<String, String>` in memory and exposed through the API for users to retrieve. They are **never persisted to disk** — a server restart causes password loss, requiring container recreation or manual password reset.

### SSH User Account

When the container starts, the entrypoint script creates a Unix user account matching the gitpage username and sets the password:

```bash
useradd -m {username}
echo '{username}:{password}' | chpasswd
```

## Container IP Retrieval for Reverse Proxy

When proxying requests to `/app/{user}/{repo}/*`, the router needs to know the target IP address. In process mode, this is always `127.0.0.1`. In Docker mode, the application runs inside the container, so the proxy must target the container's internal IP.

### IP Discovery

1. List running containers filtered by name `gitpage-{username}`
2. Access `container.network_settings.networks` from the container summary
3. Iterate over the network map (typically `"bridge"` or a custom network)
4. Return the `ip_address` field, filtering out empty or `0.0.0.0` values

The container IP is stable for the container's lifetime but changes if the container is recreated. It is looked up dynamically on each proxy request rather than cached, ensuring correctness across container restarts.

### Proxy Configuration

```
User request: /app/alice/myapp/*
        │
        ▼
    AppProcessManager.get(repo_id) → port
        │
        ▼
    DockerManager.get_container_ip("alice") → container_ip
        │
        ▼
    Reverse proxy to http://{container_ip}:{port}/*
```

## Exec Command Pattern

The exec command pattern is the core mechanism for interacting with containers. Docker's exec API creates a new process inside the container's namespaces.

### Pattern: Attached (Build, Status Check)

Used for commands where output is needed:

1. `create_exec()` with `attach_stdout: true, attach_stderr: true`
2. `start_exec()` with no detach option
3. Collect the `Stream<LogOutput>` into a String
4. Return the combined stdout/stderr

This is used for `exec_build()` and `exec_command()`.

### Pattern: Detached (Start Application)

Used for long-running processes that should continue after the API call returns:

1. `create_exec()` with `attach_stdout: false, attach_stderr: false`
2. `start_exec()` with `detach: true`
3. Receive `StartExecResults::Detached` — no output stream
4. Return immediately

This is used for `exec_start_detached()`. The application process runs as a child of the container's init process (`sleep infinity`).

### Pattern: Status Check with Polling

Used after starting an application to verify it's listening:

1. Execute `lsof -i :{port} -t 2>/dev/null | head -1` inside the container
2. If output is non-empty, a process is bound to the port
3. If empty, sleep 500ms and retry, up to 10 attempts

This approach works across all application types (Node.js, Rust, Python, etc.) since it checks for port binding rather than HTTP response.

## Container Restart Recovery

### The Problem

When Gitpage restarts, the Docker containers continue running (they were started with `docker run`, not as subprocesses). However, the in-memory SSH port mappings and password allocations are lost. 

### Recovery Strategy

In `DockerManager::connect()`, after connecting to the Docker daemon, the recovery process:

1. List all running containers with names matching `gitpage-*`
2. For each container:
   a. Extract the username from the container name (`gitpage-alice` → `alice`)
   b. Inspect the container's port bindings
   c. Find the port mapping for `22/tcp` to determine the SSH host port
   d. Insert the `(username, host_port)` pair into `port_allocations`
3. After recovery, `ensure_user_container()` finds the existing container and skips creation

This recovery is best-effort — SSH passwords cannot be recovered (they are not persisted). Users would need to reset their SSH credentials via the API.

### App Restore Integration

In `main.rs`, `restore_apps_on_startup()` runs after Docker connection recovery. It queries the database for all apps that have a configured port, then re-deploys them by checking out source, rebuilding, and restarting inside the container. This ensures applications are automatically restored after a server restart.

## Random Password Generation

### Algorithm

Passwords are generated from a restricted character set (`a-z`, `0-9`) using `OsRng` (operating system entropy source):

1. Define the character set: `abcdefghijklmnopqrstuvwxyz0123456789` (36 characters)
2. For each of the 12 positions, pick a random index from the set
3. Concatenate into the final password

### Security Properties

- **Entropy**: `log2(36^12) ≈ 62 bits` — sufficient for a temporary SSH password
- **Source**: `OsRng` provides cryptographically secure randomness (from `/dev/urandom`)
- **No uppercase**: Avoids confusion between characters like `O`/`0` and `I`/`l` in password display
- **No special characters**: Ensures compatibility with shell password commands (`chpasswd`)

## Named Volumes and Bind Mounts

### Bind Mount: Workspace Directory

The host path `{base_path}/apps/{username}` is bind-mounted to `/workspace` inside the container. This is where application source code lives and where builds output their artifacts. Using a bind mount (rather than copying files) means:

- No data duplication between host and container
- Build artifacts survive container removal
- The host filesystem is directly accessible from the container

### Named Volume: Home Directory

Each user has a named volume `gitpage-home-{username}` mounted to `/home/{username}` inside the container. This volume:

- Persists SSH host keys, shell history, and user-specific config
- Survives container recreation
- Is cleaned up when `remove_container` is called with `v: true`

### Why Separate Mounts?

The workspace bind mount and home volume serve different purposes:

- **Workspace**: Performance-sensitive, shared with the host, contains transient build artifacts
- **Home**: Named volume for stateful data, isolated per user, preserved across container restarts

## Resource Limits

### Memory Limit

Memory is configured via `[docker] memory_limit` in `config.toml` (e.g., `"1g"`, `"512m"`, `"256m"`). The `parse_memory_limit()` function parses the string:

- `"1g"` → `1 * 1024^3` bytes
- `"512m"` → `512 * 1024^2` bytes  
- `"256k"` → `256 * 1024` bytes
- `"1000000000"` → `1000000000` bytes (no suffix)

This value is passed to the Docker container's `HostConfig.memory` field, which enforces a hard memory limit via cgroups. Applications that exceed this limit are OOM-killed.

### CPU Shares

CPU shares (default 512, half of the Docker default 1024) control the relative CPU scheduling priority. A container with 512 shares gets half the CPU time of a container with 1024 shares under contention. This is a **relative weight**, not an absolute limit.

## Related Wiki Pages

- [_wiki/docker-runtime.md](../../_wiki/docker-runtime.md) — Docker mode architecture, container setup, and exec patterns
- [_wiki/bollard.md](../../_wiki/bollard.md) — Bollard crate internals, stream handling, error types
- [_wiki/process-vs-docker.md](../../_wiki/process-vs-docker.md) — Comparison table and mode selection guidance
- [_wiki/ssh-chroot.md](../../_wiki/ssh-chroot.md) — SSH chroot mechanism for git operations
- [_wiki/auto-deploy.md](../../_wiki/auto-deploy.md) — How Docker mode fits into the auto-deploy pipeline
