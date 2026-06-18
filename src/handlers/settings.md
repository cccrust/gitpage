# Settings Handler — `settings.rs`

Handles repository settings: access tokens, collaborators, secrets, and branch
protection rules. These are stored alongside the repo but are not part of the
Git object model.

## Access Tokens (`gpt_` Prefix, SHA-256 Hashing)

`create_token()` generates a personal access token for API authentication:

1. **Random generation** — 40 alphanumeric characters via `rand::thread_rng()`.
2. **Prefix** — The token is prefixed with `gpt_` (Gitpage token) for easy
   identification. The first 12 characters are stored as a `prefix` for display
   in the UI (e.g. `gpt_a1b2c3d4...`).
3. **SHA-256 hashing** — The full token is hashed with SHA-256 and stored as hex.
   The raw token is returned only once at creation time.
4. **Scopes** — Currently `"repo"` is the only scope (full repo access).

Token verification happens in the auth middleware: incoming `Bearer gpt_xxx` tokens
are hashed with SHA-256 and compared against stored hashes. This means if the
database is leaked, tokens cannot be reversed (except via brute force on the 40-char
random string, which is infeasible).

## Collaborators with Permission Levels

Collaborators are users who have access to a private repo without being the owner:

- `add_collaborator()` — Only the repo owner can add collaborators. Permission
  defaults to `"write"` (the only level currently implemented).
- `list_collaborators()` — Returns all collaborators for a repo.
- `remove_collaborator()` — Only the repo owner can remove.

Collaborator permissions are stored in the `collaborators` table. The permission
check during content access would need to consult this table — currently private
repo access is limited to `repo.user_id` and org members.

## Secrets with AES-256-GCM Encryption

Secrets are encrypted environment variables used during App deployment:

1. **Encryption** (`encrypt_secret()`):
   - Retrieves the 256-bit encryption key from `auth::get_encryption_key()` (derived
     from the JWT secret via SHA-256).
   - Generates a random 96-bit nonce via `Aes256Gcm::generate_nonce()`.
   - Encrypts the plaintext in-place using `AES-256-GCM`.
   - Prepends the nonce to the ciphertext (nonce || ciphertext).
2. **Decryption** (`decrypt_secret()`):
   - Splits the stored data at 12 bytes (nonce size).
   - Decrypts with the same key. Returns `AppError` on failure (wrong key or
     tampered data).

AES-256-GCM provides authenticated encryption: any tampering with the ciphertext
will cause decryption to fail. See `_wiki/aes-256-gcm.md` for the full cryptographic
background.

## Branch Protection Rules

Branch protection prevents direct pushes to sensitive branches:

| Field | Description |
|-------|-------------|
| `pattern` | Glob pattern matching branch names (e.g. `main`, `release/*`) |
| `require_pr` | Require all changes to come via PR (default: `true`) |
| `require_approvals` | Number of required PR approvals (default: `1`) |
| `dismiss_stale_reviews` | Dismiss approvals when new commits are pushed (default: `true`) |

The CRUD is straightforward (`create_branch_protection`, `list_branch_protections`,
`delete_branch_protection`). Enforcement is a future concern — the rules are
currently stored but not checked during push operations.

## Design Decisions

- **Symmetric encryption at rest**: Secrets are encrypted with a key derived from
  the same JWT secret used for auth tokens. This means the encryption key is always
  available (no key management) but if the JWT secret leaks, all secrets are
  compromised.
- **No UI for permission management**: The collaborator and branch protection
  features have backend support but may not have full frontend coverage.
- **Scope-limited tokens**: Token scopes are stored as a simple string. The only
  scope currently recognized is `"repo"`.
