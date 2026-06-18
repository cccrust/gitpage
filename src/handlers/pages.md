# Pages Handler — `pages.rs`

Handles Pages configuration CRUD and static site deployment. Gitpage Pages provides
GitHub Pages–like static site hosting.

## Pages Config CRUD

The config is stored in the `pages_configs` SQLite table (one row per repo):

| Field | Description | Default |
|-------|-------------|---------|
| `branch` | Git branch to deploy from | `"main"` |
| `source_dir` | Subdirectory within the repo to serve | `"/"` |
| `custom_domain` | Optional custom domain | `""` |
| `enabled` | Whether pages is active | `false` |

`update_pages_config()` upserts this record. Ownership is restricted to
`repo.user_id == user_id` (org admin support is not implemented here).

## Deploy via Tree Extraction

Both the auto-deploy on config save and the manual `deploy_pages_handler()` call
`git::deploy_pages()` which:

1. Cleans the pages output directory at `{storage.base_path}/repos/{owner}/{repo}/`
   (note: `pages_dir()` appends `/repos` for backwards compat).
2. Opens the bare repo, resolves HEAD to a commit, gets the tree.
3. If `source_dir != "/"`, navigates to the subtree at that path.
4. Recursively extracts all blobs from the tree into the output directory,
   preserving the directory structure.

This is a full checkout, not symlinks. See `_wiki/auto-deploy.md`.

## Auto-Deploy on Config Save

When `enabled = true`, the handler immediately runs the deploy after upserting the
config. If the deploy fails (e.g. branch doesn't exist), the config is still saved
and the error is returned as `deploy_error` in the response body (not as an HTTP
error — the config save itself is considered successful).

## Design Decisions

- **No incremental deploy**: Every deploy is a full tree extraction. For typical
  Gitpage sites (docs, blogs, portfolios) this is fast enough.
- **Backwards-compatible path**: `pages_dir()` uses `{base_path}/repos/{owner}/{repo}`
  to be consistent with earlier versions that stored pages alongside repos.
- **Owner resolution**: Unlike other handlers, `pages.rs` does not use
  `resolve_owner_name()` — it assumes user ownership only. Pages for org-owned repos
  would need `resolve_owner_and_repo()` from `app.rs`.
