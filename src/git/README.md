# Git — libgit2 Wrappers and HTTP Backend

## Overview

The `git/` module implements two complementary approaches for interacting with Git repositories:

1. **libgit2 (read operations)** — `mod.rs` provides safe Rust wrappers around the libgit2 C library for reading repository contents: tree traversal, blob extraction, commit log, and pages deployment.
2. **git http-backend (write operations)** — push/pull/clone operations are delegated to the system's `git http-backend` subprocess, which implements the Git HTTP Smart Protocol.

## Parallel Approach Rationale

libgit2 excels at programmatic tree/blob inspection (listing directories, reading files, extracting READMEs), which is needed for the file browser, pages hosting, and commit log UI. However, implementing the full Git HTTP Smart Protocol for push/pull in pure Rust would be both complex and error-prone. By delegating to `git http-backend`, Gitpage gets a standards-compliant, battle-tested implementation for the write path. The `init_bare_repo()` function also shells out to `git init --bare` for creating new repositories, since libgit2's bare repo initialization is less configurable and harder to debug in production.

## Module Structure

- `mod.rs` — all functions; no submodules beyond this single file
- `README.md` — this file

## References

- See `_wiki: git-http-smart-protocol.md` for the HTTP Smart Protocol design
- See `_wiki: libgit2.md` for libgit2 API patterns and gotchas
