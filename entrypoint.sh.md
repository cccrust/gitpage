# entrypoint.sh — Docker Container Entrypoint

## Overview

`entrypoint.sh` is the ENTRYPOINT for the Gitpage Docker image. It prepares the container environment at runtime and then launches the Gitpage server. SSH host keys are **not baked into the image** — they are generated on first container start to avoid sharing keys across deployments.

## Execution Sequence

### 1. SSH Host Key Generation

```bash
if [ ! -f /etc/ssh/ssh_host_rsa_key ]; then
    ssh-keygen -A 2>/dev/null
fi
```

`ssh-keygen -A` generates all missing host key types (RSA, ECDSA, Ed25519). The check `[ ! -f ... ]` avoids regenerating on container restart when `gitpage-ssh-keys` volume persists `/etc/ssh`.

**Why at runtime**: Baking host keys into the image would mean every container instance uses the same keys, which is a security risk. By generating at startup and persisting to a volume, each deployment gets unique keys that survive restarts.

### 2. Root Password

```bash
ROOT_PASS="${SSH_ROOT_PASSWORD:-gitpage}"
echo "root:$ROOT_PASS" | chpasswd -c SHA512
```

Default password is `gitpage`, overridable via the `SSH_ROOT_PASSWORD` environment variable. Used for SSH access as `root`.

### 3. Per-User SSH Account Creation

The `SSH_USERS` environment variable (format: `"user1:pass1,user2:pass2"`) drives automatic user account creation:

- Parses comma-separated `user:password` pairs.
- Creates system user accounts with home directories and `/bin/bash` shell.
- Writes a `~/.gitpage-sync` script that clones the user's bare repos from `data/repos/` on first login.
- Sources the sync script in `~/.bashrc` so repos are available when the user SSHes in.
- This bridges the gap between the git http-backend (bare repos in `data/repos/`) and SSH interactive access (user wants working clones).

### 4. SSH Daemon Startup

```bash
mkdir -p /run/sshd
/usr/sbin/sshd -D &
sleep 1
```

- `sshd -D &` starts the daemon in the foreground but backgrounds it so the script can continue.
- `sleep 1` gives sshd time to initialize before Gitpage starts.

### 5. Gitpage Server Launch

```bash
exec gitpage "$@"
```

`exec` replaces the shell process with the Gitpage server, so signals (SIGTERM, SIGINT) propagate correctly to the Rust binary. The `"$@"` passes any `CMD` arguments from the Dockerfile.

## Why This Sequence Matters

1. **SSH keys must exist before sshd starts** — sshd refuses to start without host keys.
2. **sshd must be running before Gitpage** — Gitpage's SSH subsystem integration expects the daemon to be available (though Gitpage itself handles SSH via its own mechanism, having the standard SSH daemon available enables interactive shell access).
3. **User accounts must exist before gitpage starts** — not strictly required for the HTTP API, but necessary if users SSH in immediately after container boot.

## Signal Handling

Because the final line uses `exec`, the Gitpage binary becomes PID 1 and receives all signals directly. There is no intermediate shell process intercepting signals. This is important for:
- `docker stop` (SIGTERM → graceful shutdown)
- `docker restart` (SIGTERM → cleanup → restart)

The sshd daemon runs in the background and is not explicitly shut down — Docker's SIGTERM will terminate it as a side effect when the container stops.

## Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `SSH_ROOT_PASSWORD` | `gitpage` | Password for root SSH access |
| `SSH_USERS` | _(empty)_ | Create user accounts, format: `"user1:pass1,user2:pass2"` |

## References

- `Dockerfile` — copies this script and sets it as ENTRYPOINT.
- `Dockerfile.base` — pre-installs openssh-server and configures sshd (PermitRootLogin, PasswordAuthentication).
- `run_docker.sh` — passes `SSH_USERS` environment variable.
