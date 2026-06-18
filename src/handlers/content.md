# Content Handler — `content.rs`

Handles Git tree browsing, blob reading, README rendering, and commit log retrieval.
This is the read-side of the Git data model.

## `resolve_repo()` Owner Resolution

All content endpoints use `resolve_repo()` to map a `(username, repo_name)` pair to
a `(Repository, owner_name)` tuple:

1. **Try user first** — `db.find_user_by_username()` then `db.find_repo_by_name()`.
2. **Try org second** — `db.find_org_by_name()` then `db.find_org_repo_by_name()`.
3. **Private repo check** — If the repo is private, access is denied unless the
   authenticated user is the owner (user type) or an org member (org type).
4. **Fallback** — Returns `AppError::NotFound` if neither user nor org match.

See `_wiki/owner-resolution.md` for the full design.

## Git Tree Browsing via libgit2

`list_directory()` uses `git::list_directory(&repo_path, branch, path)` which
internally uses libgit2's `tree` iteration:

1. Opens the bare repo with `git2::Repository::open_bare()`.
2. Resolves the branch to a commit, then to a tree object.
3. Walks the tree entries, returning `(name, is_dir)` pairs.

All reads are local to the bare repo — no checkout needed. See
`_wiki/libgit2.md` for the object model.

## Blob Reading with Optional Markdown Rendering

`get_file_content()`:

1. Resolves the blob from the tree via `git::get_file_content()`.
2. Returns raw bytes + detected MIME type.
3. If the path ends in `.md` or `.markdown`, the content is rendered to HTML:

   - **Math protection**: `$$...$$` (display) and `$...$` (inline) LaTeX expressions
     are extracted with regex before Markdown parsing (pulldown-cmark would mangle
     underscores and carets), then restored as KaTeX-compatible HTML after rendering.
   - **Markdown parsing**: Uses `pulldown_cmark::Parser` + `html::push_html`.
   - **Result**: Both raw `content` (string) and `rendered` (HTML) are returned,
     along with `is_markdown` flag for the frontend.

## README Resolution

`get_readme()`:

1. Tries common README filenames (`README.md`, `Readme.md`, `readme.md`, `README`,
   `README.markdown`, `README.rst`, `README.txt`) by checking existence in the
   repo's root tree.
2. Renders the found README to HTML via `render_markdown()`.
3. Returns `{ has_readme: false }` if none found (non-error response).

## Commit Log with Revwalk

`list_commits()` uses `git::get_commit_log()` which wraps libgit2's `Revwalk`:

1. Opens bare repo, resolves branch to OID.
2. Creates a `Revwalk`, pushes the branch head, and iterates commits.
3. Returns up to 50 entries with `(sha, message, author, time)`.
4. Sorting is by commit time descending (most recent first).

## Private Repo Access Control

The `user_id: Option<axum::Extension<i64>>` pattern on content endpoints allows
optional auth. For private repos:

- **User-owned**: Only `repo.user_id` matches the authenticated user.
- **Org-owned**: Any org member (regardless of role) has read access.
- **Unauthenticated** access to private repos returns 401.

This differs from the mutation permission model in `repos.rs` / `settings.rs`
where org admin role is required for writes.
