# test.sh — Integration Test (Host)

## Overview

`test.sh` runs the full Gitpage integration test suite directly on the host machine (no Docker). It exercises the REST API, git push/pull via HTTP, and Gitpage Pages deployment.

## Why No Test Framework

The test suite uses **bare `bash` + `set -x`** — no pytest, no Jest, no Rust test harness. This is a deliberate choice:

- **Zero dependencies**: The test only needs `curl`, `python3`, `git`, and `bash` — all commonly available.
- **Transparency**: Every command is printed via `set -x`, so you see exactly what's happening.
- **Simplicity**: No test framework to learn or install. The pattern is: run a command, parse JSON with inline Python, check for expected values.
- **Integration focus**: These tests validate the system end-to-end (server + API + git HTTP backend + file system), which is poorly suited to unit test frameworks.

The tradeoff is no structured assertion library, no test isolation, and no parallel execution.

## Test Flow

### Setup

1. Cleanup any previous test data and kill stale server processes.
2. Create `data/repos/` directory.
3. Build the backend with `cargo build` (debug mode).
4. Start server in background, wait 3 seconds.

### Tests (numbered 1–41)

| # | Test | What It Validates |
|---|------|-------------------|
| 1 | Register | `POST /api/auth/register` creates user |
| 2 | Login | `POST /api/auth/login` returns JWT |
| 3 | Me | `GET /api/auth/me` returns user profile |
| 4 | Create repo | `POST /api/repos` creates repo with given name |
| 5 | List repos | `GET /api/repos` returns user's repos |
| 6 | Public repos | `GET /api/users/{user}/repos` (unauthenticated) |
| 7 | Git push | `git push` over HTTP to git http-backend |
| 8 | Tree listing | `GET /api/{user}/{repo}/tree?branch=main` |
| 9 | Subdirectory tree | Tree with `path=src` filter |
| 10 | Blob + Markdown | Blob endpoint with `is_markdown` flag + rendered HTML |
| 11 | Raw file | Blob endpoint content for a `.rs` file |
| 12 | README endpoint | `GET /api/{user}/{repo}/readme` with rendered output |
| 13 | Commits | `GET /api/{user}/{repo}/commits/main` — commit list |
| 14 | Clone | `git clone` over HTTP |
| 15 | Push second commit | Push a new file to existing repo |
| 16 | Verify second commit | Commit list shows 2 entries |
| 17 | Push index.html | Add an HTML file for Pages test |
| 18 | Enable Pages | `PUT /api/pages/{id}` — configure Pages |
| 19 | Check Pages config | `GET /api/pages/{id}` returns config |
| 20 | Serve Pages | `GET /pages/{user}/{repo}/` returns index.html |
| 21 | Redeploy | `POST /api/pages/{id}/deploy` |
| 22–38 | Org tests | Create org, duplicate name rejection, org repo, org git push, org clone, member management, conflict with existing username, delete org repo |
| 39–40 | Delete repo | `DELETE /api/repos/{id}` then verify |
| 41 | Auth rejection | Unauthenticated request returns error |

### Cleanup

```bash
cleanup() {
    pkill -f gitpage 2>/dev/null
    rm -rf /tmp/gptest-*
}
trap cleanup EXIT
```

- Kills the Gitpage server process.
- Removes temporary test directories (`/tmp/gptest-repo`, `/tmp/gptest-clone`, etc.).
- The `data/` directory is **preserved** — running `test.sh` multiple times accumulates state.

## Key Design Decisions

- **Preserves `data/`**: Unlike `seed.sh`, the server is not restarted fresh. This means you can run tests against evolving data state.
- **No Docker**: Operates directly on the host. Requires Rust toolchain and all runtime dependencies.
- **Inline Python for JSON parsing**: `python3 -c "import sys,json;..."` extracts fields from API responses without installing `jq`.
- **`set -x`**: Every command is echoed before execution, making failures easy to debug.
- **Exit on first failure**: `set -x` alone does not exit on error; the script relies on individual command failures being visible. Many commands use `-sf` with curl (silent, fail on HTTP error).

## Usage

```bash
./test.sh
```

## References

- `AGENTS.md` — Testing section documents `./test.sh`.
- `test_docker.sh` — Same test suite running inside Docker.
- `test_docker_mode.sh` — Docker runtime mode tests (per-user containers).
- `seed.sh` — For fresh state before testing.
