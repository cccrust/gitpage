# Auth Module

## Overview

The `auth/` module handles all authentication and encryption key management for the Gitpage platform. It provides JWT (JSON Web Token) creation and verification for stateless API authentication, as well as AES-256-GCM encryption key initialization for protecting user secrets stored in the database.

## JWT Authentication

### Why JWT?

Gitpage uses JWT rather than session-based authentication because:

1. **Stateless**: No server-side session store is needed. Token carries all user identity information.
2. **Decoupled**: Authentication logic is independent of the database — no query needed per request.
3. **Cross-platform**: The same token works across the API, WebSocket connections, and git HTTP operations.

### Token Structure

Each JWT contains three standard claims:

- `sub` (subject): user ID as a 64-bit integer, used to look up the user in database queries
- `username`: plain text username, used for logging and display in API responses
- `iat` (issued at): Unix timestamp of token creation
- `exp` (expiration): Unix timestamp after which the token is invalid

The token is signed with HMAC-SHA256 (HS256) using a secret key initialized at server startup. Since the payload is only base64-encoded (not encrypted), sensitive information like passwords is never placed in the claims.

### Creation and Verification Flow

```
User Login       create_token()       Signed JWT        Client stores
   │                  │                   │              in localStorage
   ▼                  ▼                   ▼                    │
Credentials ──► Verify password ──► Build Claims ──► HS256 ───┘
                                    (sub, username,        │
                                     iat, exp)             │
                                                    ┌──────┘
                                                    ▼
                                            API Request with
                                            Authorization: Bearer <token>
                                                    │
                                                    ▼
                                          verify_token() ──► Claims
                                              │                  │
                                         Check exp        Inject user_id
                                                          into request
                                                          extensions
```

Every protected API handler extracts the authenticated user's ID from Axum's `Extension<i64>` middleware layer. The `username` claim is used for direct display in responses.

### Security Considerations

- Token expiration is configurable via `[jwt] expires_in_hours` in `config.toml`
- The secret can be overridden at runtime via the `JWT_SECRET` environment variable
- No refresh token mechanism is currently implemented — the client must re-authenticate after expiry
- The `Validation::default()` in `jsonwebtoken` checks `exp` automatically but does not validate `iss` or `aud`

## Encryption Key Management

### Purpose

The encryption key is used for AES-256-GCM authenticated encryption of sensitive data stored in SQLite, primarily CI/CD secrets (environment variables). The key is derived via SHA-256 from either:

1. The `[secrets] encryption_key` config value (preferred), or
2. The JWT secret as a fallback

Using a separate encryption key from the JWT secret follows the principle of key separation: the signing key and the encryption key serve different purposes and should be independent.

### Key Derivation

```
Raw key string (config.toml)
        │
        ▼
    SHA-256 hash
        │
        ▼
    32 bytes (256 bits)
        │
        ▼
    AES-256-GCM key
```

SHA-256 ensures the key is exactly 32 bytes regardless of the input string length, and provides a uniform distribution of bits suitable for AES-256.

## The OnceLock Pattern

### Problem

Rust's global variables must be initialized at compile time, but JWT secrets and encryption keys are loaded from a configuration file at runtime. These values are read-only after initialization and need to be accessible from anywhere in the codebase.

### Solution

`std::sync::OnceLock<T>` provides a thread-safe container for values that are set exactly once and then immutable. This is superior to:

- `lazy_static!` / `once_cell::sync::Lazy` — third-party dependencies now unnecessary since Rust 1.70
- `Mutex<T>` — unnecessary locking overhead when the value never changes after init
- Passing through every function signature — pollutes interfaces

### Initialization Sequence

In `main.rs`, the initialization follows a strict order:

1. Load `config.toml` → `Config` struct
2. Call `init_jwt_secret()` with `config.jwt.effective_secret()` 
3. Call `init_encryption_key()` with the configured encryption key
4. After this point, all handlers can safely call `JWT_SECRET.get()` and `ENCRYPTION_KEY.get()`

### Thread Safety

`OnceLock` internally uses `std::sync::Once` which provides:

- **Single-initialization guarantee**: Only the first `.set()` call succeeds; subsequent calls are no-ops
- **Memory ordering fence**: After `.set()` completes, all subsequent `.get()` calls from any thread see the fully initialized value
- **Lock-free read**: Once initialized, `.get()` is an atomic load with no contention

## Module Structure

```
src/auth/
├── mod.rs        — JWT create/verify, encryption key init, Claims struct
├── README.md     — This file
└── _wiki/        — See wiki entries for deeper dives:
    ├── jwt-auth.md        — JWT theory, structure, and middleware integration
    ├── aes-256-gcm.md     — AES-256-GCM authenticated encryption details
    └── onceLock-init.md   — OnceLock pattern, alternatives, and test considerations
```

## Related Wiki Pages

- [_wiki/jwt-auth.md](../../_wiki/jwt-auth.md) — JWT structure, middleware flow, security risk analysis
- [_wiki/aes-256-gcm.md](../../_wiki/aes-256-gcm.md) — GCM mode theory, nonce reuse risk, database schema for encrypted secrets
- [_wiki/onceLock-init.md](../../_wiki/onceLock-init.md) — OnceLock vs LazyLock, initialization order, thread safety
- [_wiki/apperror-pattern.md](../../_wiki/apperror-pattern.md) — How auth errors are wrapped into the unified error type
