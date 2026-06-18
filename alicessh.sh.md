# alicessh.sh — Quick SSH Helper

## Overview

A minimal two-line helper script for SSH access to the Gitpage Docker container as user `alice`.

## Commands

```bash
ssh alice@localhost -p 2222                                    # Interactive SSH session
ssh -L 3000:localhost:3000 alice@localhost -p 2222              # SSH with local port forwarding
```

The first line opens a standard SSH session. The second creates a tunnel forwarding host port 3000 to localhost:3000 inside the container — useful for accessing a user app's development server running on port 3000 inside the container.

## Usage

```bash
# Requires Docker container running with SSH on port 2222 (default in run_docker.sh)
./alicessh.sh

# Or use directly:
ssh -L 3000:localhost:3000 alice@localhost -p 2222
```

## Reference

- `run_docker.sh` — passes `SSH_USERS="alice:alice123,bob:bob123"` and maps port 2222 → container port 22.
- `entrypoint.sh` — creates the `alice` user account with password `alice123`.
