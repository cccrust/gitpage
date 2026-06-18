# Git Smart Handler — `git_smart.rs`

Serves static files from the Pages output directory. This is the read-side of the
Pages hosting feature — the counterpart to `pages.rs` which writes the output.

## Static File Serving from Pages Output Directory

`serve_pages(pages_dir, path)` is called by the fallback handler in `src/app.rs`
when the request path matches `/pages/{user}/{repo}/*`:

1. **Path construction** — Joins `pages_dir` with the request path.
2. **Exact file check** — If the joined path exists and is a file, it is served
   directly.
3. **Directory index** — If the path is empty or ends with `/`, the handler looks
   for an `index.html` file in that directory. This enables clean URLs like
   `/pages/alice/blog/` serving `index.html`.
4. **Fallback** — If none of the above match, the file path as-is is attempted
   (for extension-less files).

Once the target is resolved, the file is read asynchronously (`tokio::fs::read`)
and served with the correct MIME type (via `mime_guess`).

## SPA Fallback for Pages

For SPA deployments, the frontend app (`index.html`) handles routing client-side.
The `serve_pages` function returns `404` for truly missing files — it does **not**
implement automatic SPA fallback (returning `index.html` for all 404s).

The SPA fallback is implemented at a higher level in `src/app.rs`'s fallback handler
for the main frontend SPA. If you need SPA fallback for Pages (e.g. for a React
app deployed as Pages), this must be wired at the caller level. See
`_wiki/spa-fallback.md` for the general concept.

## Design Decisions

- **Simple file serving**: No caching headers, no directory listing, no URL
  rewriting. The handler is intentionally minimal — it reads and serves files.
- **Async reads**: Uses `tokio::fs::read` to avoid blocking the async runtime
  on disk I/O.
- **No security restrictions**: The `pages_dir` is already scoped per user/repo
  by the caller in `app.rs`. Path traversal into `..` is prevented because the
  caller constructs `pages_dir` from config values, not user input.
