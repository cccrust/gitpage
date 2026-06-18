# Database — SQLite Data Layer

## Overview

The `db/` module provides the complete SQLite-backed persistence layer for Gitpage. It is organized into two files:

- **`mod.rs`** — the `Database` struct with all CRUD operations, connection management, and schema migrations
- **`models.rs`** — all data model structs with `Serialize`/`Deserialize` derives for JSON API serialization

## Architecture

### WAL Mode

The database connection is opened in **WAL (Write-Ahead Logging)** mode (`PRAGMA journal_mode=WAL`). WAL mode allows concurrent reads and writes without blocking, which is essential for a web server where multiple requests may hit the database simultaneously. It also provides better crash recovery characteristics than the default rollback journal.

### Async Mutex Design

Because rusqlite's `Connection` is not `Send` (and therefore cannot be used directly across `.await` points in async Rust), the connection is wrapped in `Arc<tokio::sync::Mutex<Connection>>`. This allows the `Database` struct to be `Clone + Send + Sync`, shared across Axum handlers, while ensuring only one SQL operation executes at a time. The `tokio::sync::Mutex` is held only for the duration of each individual method call, not across request lifetimes.

## Module Structure

- `mod.rs` — Database struct, migrations, all CRUD methods (~1800 lines)
- `models.rs` — Data model structs (~330 lines)
- `README.md` — this file

## References

- See `_wiki: wal-mode.md` for SQLite WAL mode trade-offs
- See `_wiki: rusqlite.md` for rusqlite API patterns (query_row, query_map, execute, params! macro)
