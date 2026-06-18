# Files Handler — `files.rs`

Handles staging area CRUD operations — the file manager that works outside the
Git object model. Users can upload, delete, move, and create files in a staging
directory before committing them to the bare repository.

## Staging Area CRUD Operations

All operations work on `{storage.base_path}/staging/{owner}/{repo}/`:

| Endpoint | Action |
|----------|--------|
| `tree` | List directory contents (with metadata: name, is_dir, size, updated_at) |
| `raw` | Read file content with proper MIME type |
| `write_file` | Upload/create a file (binary bytes body) |
| `delete_file` | Remove a file or directory |
| `mkdir` | Create a new directory |
| `move_file` | Rename or move a file/directory |

The staging directory is a plain filesystem tree — not a Git working tree. There
is no `.git` directory inside staging. This design means the file manager operates
at full filesystem speed for uploads and listing, with Git operations deferred to
commit time.

## Path Traversal Prevention (`safe_path`)

All file path endpoints use `safe_path(base, file_path)` which:

1. **Rejects `..`** — Any path containing `..` is rejected with
   `AppError::BadRequest("不允許的路徑跳躍")`.
2. **Strips leading `/`** — Normalizes `//foo/bar` to `foo/bar`.
3. **Concatenates** — Joins the staging base path with the cleaned relative path.

The `move_file` endpoint has additional hardened checks on both `from` and `to`.
This prevents directory traversal attacks that could read/write files outside the
staging area.

## Status Computation (Diff between Staging and HEAD)

`status()` returns all files currently in the staging area as `"added"` changes.
The current implementation (`list_staging_changes()`) walks the staging directory
recursively and reports every file as `change_type: "added"`.

This is a simplified model — it does not compute a true diff against HEAD. The
rationale is that the staging area is the source of truth: any file present in
staging will be committed. For true diff viewing, the frontend can use the content
handler to browse the committed tree.

## Staging Commit with Auto-Deploy

`commit()` builds a git tree + commit from staged files and pushes it to the
bare repository:

1. **Validation** — Commit message must not be empty. Staging must have at least
   one file (checked via `list_staging_changes()`).
2. **Tree construction** — `git::commit_staging()` walks the staging directory and
   uses libgit2 to:
   - Create blobs for each file
   - Build tree objects for directory structure
   - Create a commit on `default_branch` with the user as author
3. **Auto-deploy** — Two background tasks are spawned:
   - `auto_deploy_pages()` — Extracts files from the repo to the pages output dir
   - `auto_deploy_app()` — Checks out, builds, and starts the app

See `_wiki/staging-area.md` for the design rationale behind the separate staging
filesystem. See `_wiki/auto-deploy.md` for the deployment pipeline.

## Owner Resolution

The `resolve_owner_name()` helper in this file follows the same pattern as elsewhere:
if `repo.owner_type == "org"`, look up the org by ID; otherwise look up the user.
This ensures the correct filesystem path regardless of ownership type.
