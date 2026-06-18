# Repos Handler — `repos.rs`

Handles repository CRUD, search, fork, and listing. Repos can be owned by either
a user or an organization.

## Repo Creation with Owner Type

1. **Input validation** — Name must not be empty.
2. **Owner resolution** — If `req.org_name` is provided, the repo is created under
   that organization (requires the org to exist). Otherwise the repo belongs to the
   authenticated user.
3. **Duplicate check** — Prevents naming collisions within the same owner namespace.
4. **DB insert** — `state.db.create_repo()` records `owner_type` (`"user"` / `"org"`)
   and optional `org_id`.
5. **Filesystem setup** — A bare Git repo is initialized at
   `{storage.base_path}/repos/{owner}/{repo}.git` and the staging directory at
   `{storage.base_path}/staging/{owner}/{repo}/`.

See `_wiki/owner-resolution.md` for the full ownership model.

## Permission Checks for Private Repos

Private repos are enforced at the content layer (`content.rs`), but repo ownership
and deletion in `repos.rs` use a similar pattern:

- **User-owned repos**: Only `repo.user_id == user_id` can modify/delete.
- **Org-owned repos**: Members with `role == "admin"` or the original creator
  can modify/delete. `list_org_members()` is checked for each operation.
- The `resolve_owner_name()` helper computes the filesystem owner name from a
  repo record by checking `owner_type` first.

## Fork Implementation

1. **Guard checks** — The user must not already have a fork (checked by scanning
   `forked_from` field) and must not have a repo with the same name.
2. **DB record** — A new repo is created with `forked_from = source_id`, and the
   `set_repo_forked_from()` call links it.
3. **Bare clone** — The source bare repo is cloned using `git clone --bare`. This
   is done via `std::process::Command` rather than libgit2 because libgit2's
   clone is designed for non-bare repos.
4. **Cleanup on failure** — If the clone fails, the DB record and partial filesystem
   paths are cleaned up before returning an error.

## Search with Pagination

`search_repos()` accepts `q`, `page`, and `page_size` query params:

- Defaults: `page=1`, `page_size=20`
- Bounds: `page_size` is clamped to 1..100
- DB returns `(repos, total)` — pagination is server-side via SQL `LIMIT/OFFSET`.
- Response includes `total`, `page`, `page_size`, `total_pages` for frontend
  pagination UI.

## Other Endpoints

- `list_user_repos` — Returns all repos for the authenticated user (requires auth).
- `list_public_repos` — Returns all public repos for a given username, checking both
   user and org namespaces.
- `get_repo` — Looks up a repo by user/org name + repo name.
- `get_repo_by_id` — Looks up by numeric ID, resolves owner name.
- `delete_repo` — Removes bare repo, staging, pages dir, app workspace, kills running
   apps, then deletes the DB record.
- `update_repo_handler` — Allows rename (which moves filesystem paths), description,
   and visibility changes.
