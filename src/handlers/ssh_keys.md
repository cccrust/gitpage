# SSH Keys Handler — `ssh_keys.rs`

Handles SSH public key CRUD for repo-level deploy keys. SSH keys enable
password-less Git operations (clone, push, pull) over the SSH protocol.

## SSH Key Format Validation

`validate_public_key()` checks the raw key string:

1. **Non-empty** — Rejects blank input.
2. **Prefix check** — The first space-delimited token must start with one of:
   - `ssh-rsa` — RSA (2048/4096-bit)
   - `ssh-ed25519` — Ed25519 (recommended)
   - `ecdsa-sha2-` — ECDSA (P-256/P-384/P-521)

Other key types (DSA, `ssh-dss`) are rejected. This is a format-level check —
the cryptographic validity of the key is not verified at upload time. A user could
upload a malformed key that passes this check; it would simply fail at SSH auth time.

## `authorized_keys` Regeneration

Every key mutation (add/delete) triggers `regenerate_authorized_keys()`:

1. Reads all SSH keys from the database.
2. Writes `~/.ssh/authorized_keys` with a `command="gitpage-shell"` prefix on each
   line, binding each key to the restricted shell.
3. Writes the `gitpage-shell` script to `~/.ssh/`.

This ensures the filesystem state is always consistent with the database. The
regeneration is fire-and-forget — failures are logged via `tracing::warn!` but
not returned to the user. See `_wiki/ssh-chroot.md`.

## Org Admin Permission Checks

Unlike other handlers that check `repo.user_id == user_id`, `ssh_keys.rs` uses
`check_repo_permission()` which also allows org admins:

1. **Direct owner** — `repo.user_id == user_id` passes immediately.
2. **Org admin** — If `repo.owner_type == "org"`, members with `role == "admin"`
   are also authorized.
3. **Fallback** — Otherwise returns 401.

This is because SSH keys are deploy keys that both repo owners and org admins
need to manage. Regular org members (non-admin) cannot manage keys.

## Design Decisions

- **Repo-scoped keys**: SSH keys are attached to repos (not users). A key
  uploaded for repo A cannot access repo B. This is a deploy-key model, similar
  to GitHub's deploy keys.
- **No user SSH keys**: Personal user SSH keys (for `git clone` over SSH across
  all repos) are not implemented. The current model is strictly per-repo deploy keys.
- **Key name**: Each key has a human-readable name (e.g. "staging-server") for
  identification in the UI.
