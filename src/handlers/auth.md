# Auth Handler — `auth.rs`

Handles user registration, login, profile management, password changes, and SSH
connection info retrieval.

## Registration Flow

1. **Validation** — Username must be >= 3 chars, password >= 6 chars. Rejected early
   with `AppError::BadRequest` before any DB or crypto work.
2. **Argon2 Hashing** — A random salt is generated via `OsRng`, then
   `Argon2::default().hash_password()` produces the password hash string. Argon2 was
   chosen for its memory-hard property (resists GPU/ASIC attacks). See
   `_wiki/argon2.md` for details.
3. **DB Insert** — `state.db.create_user()` stores username, email, and hash. A UNIQUE
   constraint on username/email is caught and mapped to `AppError::Conflict`.
4. **Docker Container** — If runtime mode is `"docker"`, `docker.ensure_user_container()`
   is called to create a per-user container (`gitpage-{username}`). See
   `_wiki/docker-runtime.md`.
5. **JWT Creation** — A signed JWT (HS256) is created from `UserPublic` and returned
   alongside the user object. See `_wiki/jwt-auth.md`.

## Login Flow

1. **User Lookup** — `db.find_user_by_username()`. If absent, a generic
   "使用者名稱或密碼錯誤" error is returned (no user enumeration).
2. **Argon2 Verify** — `PasswordHash::new()` parses the stored hash, then
   `argon2.verify_password()` checks the candidate. Timing-safe comparison is
   handled by the Argon2 crate.
3. **JWT Creation** — Same as registration. The token includes user ID, username,
   and expiration (configurable via `jwt_expires_hours`).

## Password Change Flow

1. Authenticated user provides `current_password` + `new_password`.
2. `current_password` is verified against stored hash (same Argon2 verify path).
3. `new_password` is re-hashed with a fresh Argon2 salt and stored.

## SSH Info Endpoint

Returns runtime mode (`process` vs `docker`) and — when Docker mode is active — the
user's SSH port, password, and container name. Used by the frontend to display
SSH connection instructions. See `_wiki/docker-runtime.md`.

## Profile Management

- `update_profile` — Allows the authenticated user to update their `bio` and
  `avatar_url`. Authorization check ensures `username` in path matches the
  authenticated user's username.
- `get_user_profile` — Public endpoint returning user info + list of public repos.
  No auth required.

## Design Decisions

- **No user enumeration**: Login returns the same error whether the user exists or
  not. Registration returns 409 on duplicate. This is a standard security practice.
- **Argon2 defaults**: Uses the crate defaults (3 iterations, 64 MiB memory, 4
  parallel lanes), which are considered secure as of 2026.
- **JWT statelessness**: The server does not store sessions. Token validity is
  verified purely via HMAC signature. Revocation would require a blocklist, which
  is not currently implemented.
