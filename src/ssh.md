# SSH — Authorized Keys Management

## Theoretical Background

### Authorized Keys with command= Restrictions

OpenSSH's `authorized_keys` file supports a `command=` option that forces execution of a specific command whenever the key is used for authentication. Gitpage leverages this to restrict SSH access to a custom shell script (`gitpage-shell`) that intercepts all SSH connections and enforces per-repo, per-user authorization before any git operation proceeds.

### SSH Security Hardening

Each entry in the generated `authorized_keys` includes the following restrictions:

- **no-port-forwarding** — prevents the user from creating TCP tunnels through the server
- **no-X11-forwarding** — blocks X11 graphical forwarding
- **no-agent-forwarding** — prevents forwarding of the SSH agent connection

These restrictions ensure that SSH keys registered for git push/pull access cannot be used for general shell access or network tunneling. The key can *only* execute the gitpage-shell script, and only for the specific user+repo combination embedded in the command string.

### The gitpage-shell Script Concept

The `command=` directive invokes a script located at `~/.ssh/gitpage-shell` with two arguments: the username and the repo name (e.g., `command="/home/git/.ssh/gitpage-shell" "alice" "my-repo"`). The script is responsible for verifying that the authenticated user matches the requested git repository, then invoking `git-shell -c "$SSH_ORIGINAL_COMMAND"` to handle the actual git upload-pack/receive-pack. This effectively creates a sandboxed SSH gateway: the SSH key is tied to a specific git repository, and nothing else.

### Database ↔ Filesystem Interaction

SSH keys are stored in the `ssh_keys` SQLite table, linked to a `user_id` and `repo_id`. The `regenerate_authorized_keys()` function performs a full rebuild of `~/.ssh/authorized_keys` by querying all keys from the database via `get_all_ssh_keys()`, which joins across `ssh_keys`, `users`, and `repositories`. This function is called whenever a key is created or deleted, ensuring the filesystem representation always mirrors the database state. The `~/.ssh/` directory is created automatically if absent.

## References

- See `_wiki: ssh-chroot.md` for chroot/jail isolation patterns
