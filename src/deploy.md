# Deployment Module

## Overview

The `deploy.rs` module implements the dynamic application deployment pipeline — the process of taking source code from a bare git repository and turning it into a running application. This is the "App Hosting" feature, analogous to Heroku or Vercel's deployment system, as distinct from static Pages hosting (which simply serves pre-built files).

The deployment pipeline consists of five sequential stages:

```
checkout_source() → detect_project_type() → resolve_commands() → run_build() → start_app()
```

Each stage feeds into the next, with deploy logs accumulated along the way for user-facing output.

## AppProcessManager

### Role

`AppProcessManager` is the in-memory registry of all running applications. It tracks each app's process ID, assigned port, and current status. This manager is the single source of truth for what applications are running on the server at any moment.

### Port Allocation

Port allocation uses a `AtomicU16` sequential allocator with collision detection:

1. Read the current atomic counter value as the candidate port
2. Check if the candidate is in the set of already-allocated ports
3. If free, return it (the caller will start the application on this port)
4. If taken, increment and try again, up to the configured `port_range_end`
5. If no port is available in the range, return an error

This is a simple but effective strategy for a single-server deployment. It does not handle race conditions between allocation and actual port binding, so a failed start will leave the port in the "used" set until explicitly unregistered.

### Process Lifecycle

```
allocate_port() ──► register() ──► update_status(Running)
                         │
                    [child exits]   update_status(Stopped)
                         │
                    [kill signal]   update_status(Stopped)
                         │
                    unregister()
```

### Background Monitoring

When an application starts in process mode, `tokio::spawn` creates a background task that waits on the child process:

- If the child exits normally, status is set to `Stopped`
- If the child exits with an error code, status is set to `Error`
- If the monitor task itself fails, status is set to `Error` with the error message

This background monitoring is essential for detecting crashed applications and updating the proxy layer accordingly.

## Project Type Detection

Before building, the system must determine what kind of project it is dealing with. Detection is file-based:

- **Node.js**: Presence of `package.json` in the project root
- **Rust**: Presence of `Cargo.toml` in the project root

If neither file is found, deployment is rejected with an error. This design explicitly limits supported project types to Node.js and Rust. Future expansion would add detection for Python (`requirements.txt`, `pyproject.toml`), Go (`go.mod`), or static site generators.

The detection is intentionally simple — one file check per type. This avoids the complexity of content inspection and works reliably for well-structured projects.

## Build Command Resolution

Commands for building and starting are resolved with a priority system:

1. **Config override**: If the repository's `AppsConfig` has explicit `build_command` or `start_command`, those are used verbatim
2. **Default by type**: Otherwise, sensible defaults are chosen:

| Type | Build Command | Start Command |
|------|--------------|---------------|
| Node.js | `npm install` | `npm start` |
| Rust | `cargo build --release` | `./target/release/{binary_name}` |

### Rust Binary Name Resolution

For Rust projects, the start command requires knowing the binary name. This is extracted by parsing `Cargo.toml` looking for the `name` field:

```
Cargo.toml → find line starting with "name =" → extract quoted value → "./target/release/{name}"
```

If `Cargo.toml` cannot be read or the name field is missing, it falls back to `./target/release/app`.

## Source Checkout from Bare Git Repo

### Why Use Git CLI Rather Than libgit2?

The checkout stage uses the system `git` command rather than libgit2 because:

1. **Bare repo support**: `git --work-tree checkout` is the standard way to extract files from a bare repository
2. **Path filtering**: The `--` separator and path arguments allow selective checkout of specific directories
3. **Reliability**: The git CLI is thoroughly tested for all edge cases in file extraction

### Checkout with Subdirectory Support

When `source_dir` (configured per app) is not the root (`/`), the checkout performs a two-step process:

1. Checkout the full tree to a temporary `source` directory
2. If `source_dir` is specified, move the contents of `source_dir` to the `source` directory root
3. Clean up the temporary intermediate directory

This allows monorepo setups where the application code lives in a subdirectory (e.g., `packages/webapp`).

```
bare repo (data/repos/user/repo.git)
        │
        ▼  git --work-tree checkout -f main --
 source/ (full checkout)
        │
        ▼  move subdirectory up
 source/ (only source_dir contents)
```

## Build Execution and Deploy Log Capture

### Process Mode

In process mode, the build runs as a subprocess on the host machine:

1. Spawn `sh -c "{build_command}"` with `current_dir` set to `source/`
2. Capture stdout and stderr
3. Format as `$ {command}\n{stdout}\n{stderr}`
4. If the process exits with non-zero status, return the captured log as an error

### Docker Mode

In Docker mode, the build runs inside the user's container via `docker exec`. The `DockerManager.exec_build()` method changes to `/workspace/{repo}/source/` before executing the build command. The log capture is identical, but execution is isolated from the host environment.

### Deploy Log Structure

The deploy log is a text document containing:

1. The checkout command and result
2. The detected project type
3. The resolved build and start commands
4. The full build output (stdout + stderr)
5. The assigned port number
6. The start result

This log is persisted to the `deploy_logs` SQLite table and displayed on the frontend.

## App Start/Stop Lifecycle

### Start (Process Mode)

1. Kill any existing process for this repository
2. Spawn `sh -c "{start_command}"` with `current_dir` set to `source/`
3. Inject `PORT` and `HOST` environment variables
4. Parse and inject user-defined `env_vars` from JSON
5. Register the process in `AppProcessManager`
6. Spawn a background monitor task
7. Wait 500ms then perform a health check

### Start (Docker Mode)

1. Kill any existing process on the target port inside the container
2. Create a detached `docker exec` running the start command
3. Set environment variables including `PORT` and `HOST=0.0.0.0`
4. Register the app in `AppProcessManager` (with pid=0 since Docker exec has no meaningful PID)
5. Wait 500ms then check process status via `lsof` inside the container

### Stop

1. Look up the app in `AppProcessManager`
2. If it's running or deploying, kill the process
3. In process mode: send `SIGKILL` to the PID, then use `lsof -ti :port | xargs kill -9` to ensure port cleanup
4. In Docker mode: execute `lsof -ti :port | xargs -r kill -9` inside the container
5. Update status to `Stopped`

## Health Check Strategies

### Why Health Checks Are Important

After starting an application, the system needs to verify it is actually accepting connections before routing traffic to it. A health check prevents the common scenario where the app appears to start but crashes immediately due to a configuration error.

### Process Mode Health Check

Uses an HTTP GET request to `http://127.0.0.1:{port}/`:

```python
GET / → 200 OK → App is healthy
GET / → Connection refused → App may need more time or failed
```

This is a true application-level check — it verifies the HTTP server is listening and responding.

### Docker Mode Health Check

Uses `lsof` inside the container to check if a process is listening on the application port:

```bash
lsof -i :{port} -t 2>/dev/null | head -1
```

This is a process-level check — it verifies something is bound to the port, but not necessarily that it's an HTTP server responding correctly.

### Polling Behavior

Both strategies poll up to 10 times with 500ms intervals (5 seconds total). The first success stops the polling. This balances responsiveness against false negatives from slow-starting applications.

## Difference Between Process Mode and Docker Mode Builds

| Aspect | Process Mode | Docker Mode |
|--------|-------------|-------------|
| Execution context | Host OS environment | Per-user container |
| Tooling | Host's Node/Rust versions | Container's fixed toolchain |
| Isolation | Shares host filesystem, env vars | Isolated filesystem, env vars |
| Dependencies | Installed globally or locally in workspace | Installed in container, isolated per user |
| Network | Binds to `127.0.0.1` | Binds to `0.0.0.0` inside container |
| Port conflict | Unique across all apps on host | Unique per container (container-internal) |
| Deploy log capture | Direct stdout/stderr pipe | Bollard exec output stream |

Despite these differences, the deployment API is identical for both modes. The `docker: Option<&DockerManager>` parameter is checked at each stage — if `Some`, the Docker path is taken; if `None`, the process path is taken.

## Related Wiki Pages

- [_wiki/auto-deploy.md](../../_wiki/auto-deploy.md) — Full pipeline including git push triggers and staging commits
- [_wiki/process-vs-docker.md](../../_wiki/process-vs-docker.md) — Detailed comparison of the two runtime modes
- [_wiki/docker-runtime.md](../../_wiki/docker-runtime.md) — Docker mode container lifecycle and exec details
- [_wiki/reverse-proxy-app.md](../../_wiki/reverse-proxy-app.md) — How running apps are proxied from `/app/{user}/{repo}/*`
- [_wiki/apperror-pattern.md](../../_wiki/apperror-pattern.md) — How deployment failures are wrapped and propagated
