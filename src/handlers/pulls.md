# Pulls Handler — `pulls.rs`

Handles Pull Request CRUD, 3-way merge, and diff computation. Follows the GitHub
PR model where a PR proposes changes from a head branch to a base branch, possibly
across different repos (cross-repo PRs / forks).

## PR CRUD with Cross-Repo Support

Each PR tracks:

| Field | Description |
|-------|-------------|
| `number` | Auto-incrementing per repo (like issues) |
| `head_repo_id` | The repo containing the source branch (may differ from base) |
| `head_ref` | Source branch name |
| `base_ref` | Target branch name |
| `state` | `"open"`, `"closed"`, or `"merged"` |

**Cross-repo PRs** are supported natively: `head_repo_id` can refer to a fork
repo owned by a different user. This enables the fork-and-PR workflow.

### Permission Model

Same as issues: `repo.user_id == user_id` or `owner_type == "org"` grants write
permission. This is lenient for org repos (any member can create PRs).

## 3-Way Merge via libgit2 `merge_trees`

`merge_pr()` implements the core merge logic using libgit2 directly (not the `git`
CLI):

1. **Open both repos** — Opens the base and head bare repos with
   `git2::Repository::open_bare()`.
2. **Resolve refs** — `refname_to_id()` resolves `refs/heads/{branch}` to commit OIDs.
3. **Find merge base** — `base_repo.merge_base(base_commit, head_commit)` finds the
   common ancestor commit.
4. **Get trees** — Extract tree objects from base, head, and ancestor commits.
5. **Merge trees** — `base_repo.merge_trees(&base_tree, &head_tree, &ancestor_tree, &opts)`.
   This performs the 3-way merge in memory (no working tree needed).
6. **Conflict detection** — If `index.has_conflicts()`, the merge is rejected with
   "合併衝突，無法自動合併". The handler does not attempt conflict resolution.
7. **Write tree** — `index.write_tree_to(&base_repo)` creates the merged tree object.
8. **Create merge commit** — A new commit is created with two parents (base and head),
   and the base branch ref is fast-forwarded to it.

The merge commit message follows the convention:
`Merge pull request #{n} from {head_owner}/{head_repo}: {title}`.

See `_wiki/three-way-merge.md` for the algorithm in depth. The libgit2 approach
was chosen over `git merge` CLI to avoid subprocess overhead and gain direct tree
manipulation access.

## Diff Computation Between Base and Head

`get_pr_diff()` computes the file-level diff between the base and head branches:

1. Resolves both branches to tree objects (same as merge preparation).
2. Calls `base_repo.diff_tree_to_tree(&base_tree, &head_tree, None)`.
3. Iterates `diff.deltas()` and maps git2 `Delta` variants to strings:
   - `Added` → "added"
   - `Deleted` → "deleted"
   - `Modified` → "modified"
   - `Renamed` → "renamed"
   - `Copied` → "copied"
4. Returns a list of `DiffEntry` (status + old_path + new_path).

The diff is file-level only — there is no line-by-line patch in the response.
The frontend could compute line diffs by fetching both blob contents.

## Design Decisions

- **Bare repo operations**: All Git operations work on bare repos. No checkout
  or working tree is needed for merge or diff.
- **Sync git2 + async Rust**: Git operations are synchronous and blocking. They
  are wrapped in a `{ }` scope block so that `git2::Repository` objects are
  dropped before any `.await` point, preventing issues with the non-Send `Repository`
  type crossing async boundaries.
- **No fast-forward shortcut**: Even if base == ancestor (trivial fast-forward),
  the merge creates a merge commit. This avoids losing PR metadata in the commit
  graph.
- **No squash merge**: Only merge commits are supported. Squash and rebase are
  not implemented.
