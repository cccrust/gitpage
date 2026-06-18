# db/models.rs — Data Model Design

## Theoretical Background

### Serialization for JSON API

All model structs derive both `Serialize` and `Deserialize` from `serde`, enabling automatic JSON serialization/deserialization when used with Axum's `Json<T>` extractor and response type. The `#[serde]` attributes use default field naming (snake_case), matching the SQL column names and the JSON API convention.

### The UserPublic Pattern

The `User` struct contains sensitive fields like `password_hash` that must never be exposed to API clients. The `UserPublic` struct provides a safe projection:

- **Fields exposed**: `id`, `username`, `bio`, `avatar_url`, `created_at`
- **Fields omitted**: `email`, `password_hash`
- **Conversion**: `impl From<User> for UserPublic` provides a zero-fuss conversion, used at the API boundary (e.g., auth responses, stargazer lists).

### Repository Ownership Model

The `Repository` struct supports two ownership models via the `owner_type` field:

- **`owner_type = "user"`** — repository owned by a regular user. `owner_id` is effectively `user_id`. The `org_id` field is `None`.
- **`owner_type = "org"`** — repository owned by an organization. `org_id` references the `organizations` table. `user_id` still stores the creating user (the org admin who created it).

This dual-ownership design allows the same `repositories` table to serve both models without separate tables. Content routes use the `resolve_repo()` helper to check both users and orgs by name. Disk paths use the owner name from the resolved entity, so repos live at `data/repos/{username}/{repo}.git` or `data/repos/{orgname}/{repo}.git`.

### Repository Stats (Stars, Forks, Watch Counts)

The `Repository` struct includes denormalized counter fields:

- **`stars_count`** — synchronized via `SELECT COUNT(*) FROM stars WHERE repo_id = ?` on every star/unstar operation
- **`forks_count`** — incremented/decremented when a fork is created/deleted
- **`watch_count`** — synchronized similarly to stars

These denormalized counters avoid expensive COUNT queries on every repository listing, at the cost of keeping them updated on mutations. The `stars` and `watches` tables serve as the source of truth; the repository columns are materialized caches.

### Request vs Response Structs

Separate structs exist for request payloads (e.g., `LoginRequest`, `CreateRepoRequest`) and response data (e.g., `AuthResponse`). Request structs derive only `Deserialize` (since they're parsed from JSON bodies), while response structs derive only `Serialize` (since they're returned as JSON). `AuthResponse` is a notable composite that bundles a `token` string and a `UserPublic` struct.

### Composite Response Structs

Several wrapper structs combine related data for API efficiency:

- `IssueWithAuthor` — combines `Issue` with `author_username` and `labels`, avoiding separate API calls for author and label data
- `PullRequestWithAuthor` — combines `PullRequest` with `author_username`, `head_repo_name`, and `head_repo_owner`, resolving the head repo's owner name (which may be a user or an org)
- `OrganizationWithRole` — extends `Organization` with the caller's `role` in that organization
- `OrgRepoResult` — extends `Repository` with the organization name for display
- `EnabledAppWithOwner` — wraps `AppsConfig` with resolved `username` and `repo_name` for app process management

### Social Features

- `Star` — composite primary key of `(user_id, repo_id)` with a `created_at` timestamp
- `Watch` — similar composite key with a `watch_type` field (`"participating"` or `"all"`)

### Settings Models (v2.1)

- `AccessToken` — stores a token hash (not the raw token), with `token_prefix` for UI display, optional `expires_at`, and `last_used_at` tracking
- `RepoCollaborator` — links a user to a repo with a permission level (`read`/`write`/`admin`), includes denormalized `username` and `avatar_url` for display
- `RepoSecret` — stores encrypted secrets (the `encrypted_value` field is `Vec<u8>`, excluded from serialization in list responses)
- `BranchProtection` — describes protection rules for branch patterns, including `require_pr`, `require_approvals`, and `dismiss_stale_reviews`

## References

- See `_wiki: owner-resolution.md` for the owner resolution algorithm in content routes
