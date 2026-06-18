# Orgs Handler — `orgs.rs`

Handles organization CRUD and member management. Organizations allow groups of
users to collaboratively own repositories and manage access collectively.

## Organization CRUD with Ownership

### Creation

`create_org()`:

1. **Validation** — Name must be >= 2 chars.
2. **Uniqueness check** — Name must not clash with an existing org or user (shared
   namespace: users and orgs occupy the same name space for URL resolution).
3. **DB insert** — Org is created with `owner_id` set to the creating user.
4. **Auto-join** — The creator is added as an `"admin"` member via
   `add_org_member()`.

### Read / Update / Delete

- `get_org()` — Public lookup by name, returns org + owner username.
- `update_org()` — Allows the org owner or any admin member to change
  `display_name` and `description`. Permission check via `get_user_org_role()`.
- `delete_org()` — Only the org owner (`org.owner_id == user_id`) can delete.
  This is stricter than update — even admins cannot delete the org.

## Member Management (Admin/Member Roles)

Two roles are supported, stored in `organization_members`:

| Role | Permissions |
|------|-------------|
| `admin` | Can update org settings, add/remove members, create/manage repos |
| `member` | Read-only org membership, can access private org repos |

### Add/Remove Members

- `add_member()` — Requires admin or owner role. Add a user by username with
  optional role (defaults to `"member"`).
- `remove_member()` — Requires admin or owner role. Cannot remove the org owner.
  Targets by `user_id` (not username) to avoid ambiguity.

The `get_user_org_role()` helper fetches all members and finds the matching user's
role. This is an O(n) lookup per call — acceptable for typical org sizes.

## Org Repo Listing

`list_org_repos()` returns all repos under the org (public and private) using
`db.list_org_repos_with_orgname()`. The frontend handles filtering by visibility.

`list_my_orgs()` returns all orgs the authenticated user belongs to, including
their role in each org.

## Design Decisions

- **User/org namespace collision prevention**: `create_org()` checks both org and
  user namespaces to prevent ambiguity in `resolve_repo()`.
- **Owner is supreme**: The org owner (`owner_id`) has all permissions and cannot
  be removed. This prevents org lockout.
- **Flat role model**: Only two roles (admin/member). No fine-grained permissions
  like GitHub's "maintain", "triage", etc.
- **No org deletion cascading**: `delete_org()` deletes the org record but does not
  clean up repos. Repo cleanup must happen before deletion via the repos handler.

See `_wiki/owner-resolution.md` for how orgs integrate with content routing.
