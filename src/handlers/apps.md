# Apps Handler — `apps.rs`

Handles App configuration CRUD, manual deploy triggers, deploy log viewing, and
app lifecycle management (start/stop). Gitpage Apps provides dynamic web application
hosting (Node.js, Rust, etc.).

## Apps Config CRUD

The config is stored in the `apps_configs` SQLite table (one row per repo):

| Field | Description | Default |
|-------|-------------|---------|
| `branch` | Git branch to deploy from | `"main"` |
| `source_dir` | Subdirectory containing the app | `"/"` |
| `build_command` | Build command (e.g. `npm run build`) | `""` |
| `start_command` | Start command (e.g. `node server.js`) | `""` |
| `env_vars` | JSON-encoded environment variables | `"{}"` |
| `enabled` | Whether the app is active | `false` |

When `enabled` is toggled to `false`, the handler stops the running app (via
`crate::deploy::stop_app()`), deletes the config, and unregisters from the
`AppProcessManager`.

## Deploy via Build Pipeline

`do_deploy()` orchestrates the full deploy via `crate::deploy::deploy_app()`:

1. **Checkout** — Git branch is checked out into the workspace at
   `{storage.base_path}/apps/{owner}/{repo}/`.
2. **Build** — `build_command` is executed in the workspace (or auto-detected:
   `npm install`, `cargo build --release`).
3. **Start** — `start_command` is run and the process is registered with
   `AppProcessManager` (process mode) or `docker exec` (Docker mode).

A `DeployLog` is created before deployment and updated on success/failure. See
`_wiki/auto-deploy.md` for the full pipeline.

## Deploy Log Recording

- `create_deploy_log()` inserts a new log with `status = "running"`.
- On success: `update_deploy_log(id, "success", log_output)`.
- On failure: `update_deploy_log(id, "failed", error_msg)`.
- `list_deploys()` returns all logs for a repo.
- `get_deploy_log()` returns a single log with cross-check against `repo_id`.

## Auto-Deploy on Config Save

When `enabled = true`, `update_apps_config()` calls `do_deploy()` synchronously
(not spawned). This means the HTTP response waits for the deploy to complete.
This differs from the Pages auto-deploy which is fire-and-forget. The rationale
is that App deploy can fail for many reasons (build errors, missing deps) and the
user needs immediate feedback.

## Design Decisions

- **Process vs Docker mode**: The `deploy.rs` module handles both modes
  transparently via the `state.docker` option. See `_wiki/docker-runtime.md` and
  `_wiki/process-vs-docker.md`.
- **App proxy**: Running apps are reverse-proxied via `/app/{user}/{repo}/*` routes
  in `src/app.rs`. See `_wiki/reverse-proxy-app.md`.
- **No org support**: Like Pages, the current App handlers check `repo.user_id == user_id`,
  not org membership.
