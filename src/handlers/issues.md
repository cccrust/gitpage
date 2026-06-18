# Issues Handler — `issues.rs`

Handles issue CRUD, label management, and comment threads. Follows the GitHub
Issues model where each issue has a repo-scoped auto-incrementing number, a
state (open/closed), and threaded comments.

## Issue CRUD with Auto-Incrementing Numbers

Each repo has its own issue number sequence, managed by `db.next_issue_number()`:

- The database tracks `MAX(number) + 1` per repo.
- Numbers are integers, starting from 1.
- Once assigned, numbers are never reused (even if an issue is deleted).

### Create

`create_issue()` requires write permission:

- **User-owned repos**: `repo.user_id == user_id`.
- **Org-owned repos**: Any user can create (checked via `owner_type == "org"` —
  this is broader than other handlers which require admin role).

Labels can be attached at creation time via `label_ids`. Labels must already exist
for the repo.

### Read / Update / Delete

- `get_issue()` — No auth required (public issues).
- `update_issue()` — Allows changing title, body, state, assignee, and labels.
  Permission check same as create.
- `delete_issue()` — Soft-deletes or hard-deletes depending on DB implementation.
  Permission check same as create.

## Label System

Labels are repo-scoped with a name and color:

- `list_labels()` — Return all labels for a repo.
- `create_label()` — Create a label. Default color `#0366d6` (GitHub blue).
- `delete_label()` — Remove a label. Does not remove it from issues (the
  `issue_labels` junction table handles that).

Labels are associated with issues via the `issue_labels` junction table.
`set_issue_labels()` replaces all labels for an issue (delete old, insert new).

## Comment Threads

Comments are stored in a `comments` table linked to issues:

- `add_comment()` — Create a comment on an issue. Requires auth. Empty body
  is rejected.
- `list_comments()` — Return all comments for an issue, ordered by creation time.
  No auth required.

There is no nested/threaded reply support — comments are flat.

## State Management

Issues have a `state` field with two values:

| State | Meaning |
|-------|---------|
| `"open"` | Issue is active and visible |
| `"closed"` | Issue has been resolved or won't be done |

Transition from open to closed (and vice versa) is done via `update_issue()`.
No merge-like restrictions (e.g., can close without a PR).

## Design Decisions

- **No cross-repo issues**: Issues are scoped to a single repo. Cross-repo
  references (`#123`) are handled by the frontend Markdown renderer only.
- **No issue templates**: Unlike GitHub, there is no configurable issue template
  system.
- **No issue search/filter**: The `ListIssuesQuery` only supports state filter
  (`?state=open` / `?state=closed`). Full text search is not implemented.
- **Assignee is optional**: `assignee_id` is a nullable field.
