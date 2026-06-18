# git/mod.rs — libgit2 Wrappers and HTTP Backend

## Theoretical Background

### Dual Approach: libgit2 Reads vs. http-backend Writes

The module embraces a pragmatic architecture where **reads** and **writes** use entirely different mechanisms:

- **libgit2** — a C library with Rust bindings (`git2` crate) that embeds a Git implementation in-process. Used for all read-only operations: listing files in a tree, reading blob content, extracting README, walking commit history (revwalk), and extracting trees for pages deployment. libgit2 provides direct, in-memory access to Git objects without spawning subprocesses.
- **git http-backend** — a standard CGI program distributed with Git that implements the Git HTTP Smart Protocol. Used for push, pull, and clone operations where the client speaks the Git wire protocol over HTTP. Gitpage spawns it as a subprocess, passes environment variables (`GIT_PROJECT_ROOT`, `PATH_INFO`, `REQUEST_METHOD`, `CONTENT_TYPE`, `REMOTE_USER`), pipes the request body to stdin, and relays the stdout response back to the client.

### Git HTTP Smart Protocol Implementation

The `handle_git_backend()` function acts as a thin CGI wrapper. It:

1. Sets `GIT_PROJECT_ROOT` to the repos directory so git http-backend can locate bare repos.
2. Sets `GIT_HTTP_EXPORT_ALL=1` to allow all repos to be fetched (access control is handled by the router, not git).
3. Passes `REMOTE_USER` for authenticated identity.
4. Parses the http-backend response by splitting on `\r\n\r\n` to separate HTTP headers from body.
5. Handles the `Status:` header from http-backend for non-200 responses.
6. Reconstructs an Axum `Response` with the parsed status, headers, and body.

### libgit2 Tree Operations

- **list_directory** — opens a bare repo, resolves a branch ref (falling back to HEAD), finds the commit, peels to its tree, optionally descends into a subtree via `tree.get_path()`, then iterates entries. Hidden files (dotfiles) are filtered out, and results are sorted with directories first, then alphabetically.
- **get_file_content** — similar tree resolution path, then peels the entry to a blob and returns its raw bytes. MIME type is guessed from the file extension via `mime_guess`.
- **get_readme** — tries common README filename variants (`README.md`, `readme.md`, `Readme.md`) via repeated calls to `get_file_content`.
- **get_commit_log** — uses `revwalk` to walk commits in reverse chronological order, returning short SHA, message, author, and formatted timestamp.

### Pages Deployment via Tree Extraction

`deploy_pages()` extracts a full directory tree from a bare repo to the filesystem. It resolves a branch+source_dir, peels to a tree, cleans the output directory, then recursively walks the tree writing every blob to disk at the corresponding path. The `walk_tree_for_pages()` helper handles recursion into subtrees and directory creation. This approach avoids checking out the entire working tree — only the specified source directory is deployed.

### Staging Commit via TreeBuilder

The file manager workflow (editing files through the web UI, not via git push) requires programmatic commit creation:

1. Files are stored in a staging directory on disk (`data/staging/{owner}/{repo}/`).
2. `commit_staging()` opens the bare repo, reads the existing branch tip (if any) to get the parent tree, then calls `build_tree_from_dir()`.
3. The resulting tree is committed with the parent as ancestor, advancing the branch ref.
4. The staging directory is cleared afterward.

### build_tree_from_dir Recursion Algorithm

`build_tree_from_dir()` converts a filesystem directory into a git tree object:

1. Starts a `TreeBuilder`, optionally seeded from the parent tree (so existing tracked files outside the staging area are preserved).
2. Sorts directory entries alphabetically.
3. For each subdirectory: recursively calls itself, passing the parent tree's corresponding subtree (if any) to preserve files inside that subdirectory that weren't modified.
4. For each file: reads the content, creates a blob, inserts it into the tree.
5. Writes the final tree object to the repository and returns its OID.

This overlay approach means the staging commit only needs to contain the files that were actually modified through the web UI — all other files in the repo are inherited from the parent commit's tree.

### Auto-Deploy Trigger on Push

In `app.rs`, after a successful git push (detected by the http-backend response), the system automatically triggers pages re-deployment and/or app restart if pages_config or apps_config have that repo enabled. This happens synchronously in the request handler — the git push response is held until deployment completes.

### Why Both Approaches Coexist

libgit2 is fast and convenient for read-heavy operations (browsing repos, serving pages), but implementing the full push/pull protocol in libgit2 would require manual packfile negotiation. The http-backend subprocess is the standard, reliable way to handle Git smart protocol traffic, used by GitLab, Gitea, and essentially every self-hosted Git platform. The split also provides a natural security boundary: the subprocess runs as a separate process with its own memory space.

## References

- See `_wiki: git-http-smart-protocol.md` for HTTP Smart Protocol details
- See `_wiki: libgit2.md` for libgit2 API patterns and known limitations
