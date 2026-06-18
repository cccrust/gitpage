# db/mod.rs — Database CRUD and Migrations

## Theoretical Background

### WAL Mode Initialization

On construction, `Database::new()` sets two PRAGMAs immediately after opening the connection:

- **`journal_mode=WAL`** — enables Write-Ahead Logging. In WAL mode, readers do not block writers and writers do not block readers (except when there are concurrent writers, which is prevented by the Mutex). WAL also tends to be faster for read-heavy workloads and provides better concurrency characteristics for a web server handling simultaneous requests.
- **`foreign_keys=ON`** — enables SQLite's foreign key enforcement. SQLite does not enforce foreign keys by default for backward compatibility; this PRAGMA must be set on every connection.

### Migration Strategy (IF NOT EXISTS + ALTER TABLE)

Migrations use a progressive enhancement strategy rather than a versioned migration system:

1. **IF NOT EXISTS** — new tables are created with `CREATE TABLE IF NOT EXISTS`, so they are only created if missing. This handles fresh installs.
2. **ALTER TABLE ADD COLUMN** — new columns on existing tables are added with `ALTER TABLE ... ADD COLUMN`. The `.ok()` call swallows errors for columns that already exist, making this idempotent.
3. **Complex migrations** — when a schema change is too involved for ADD COLUMN (e.g., removing or changing a UNIQUE constraint), a table rename/rebuild pattern is used: create a new table, copy data, drop old, rename. Foreign keys are temporarily disabled during this process to avoid dependency issues.

This approach avoids maintaining a separate migration version table or migration files — the schema evolves implicitly with the code.

### Arc\<Mutex\<Connection\>\> Design Rationale

rusqlite's `Connection` is not `Send`, which means it cannot be held across `.await` points in async code. By wrapping it in `Arc<tokio::sync::Mutex<Connection>>`:

- The `Database` struct becomes `Clone` (shared across all Axum handlers via Axum's state injection)
- Each DB method locks the mutex, performs the SQL operations, and releases the lock within a single `.await`-free block
- Contention is low because the mutex is held for very short durations (SQLite queries are fast)
- No connection pooling is needed — SQLite is a single-writer database anyway

### Query Patterns

Three main patterns are used throughout:

1. **`query_row`** — for queries returning a single row (or none). Used for lookups like `find_user_by_username`, `get_pages_config`. The `QueryReturnedNoRows` error is caught and mapped to `None`.
2. **`prepare + query_map`** — for queries returning multiple rows. A prepared statement is created, `query_map` applies a row-mapping closure, and the result is collected into `Vec<T>`. This pattern avoids allocating intermediate structures.
3. **`execute`** — for INSERT/UPDATE/DELETE operations. The number of affected rows is returned and can be checked for existence (e.g., `affected > 0` maps to `Ok(false)` for delete operations on non-existent rows).

### Transaction Handling

Most operations are single-statement and rely on SQLite's implicit autocommit. For operations that require atomicity across multiple statements (e.g., updating a star count: INSERT into stars table + UPDATE repositories), the `conn.execute_batch()` or sequential `conn.execute()` calls are used within the same mutex lock scope, providing implicit transaction isolation.

### Partial Unique Indexes for Repo Names

The `repositories` table originally had a `UNIQUE(user_id, name)` constraint. This was migrated to partial unique indexes:

```sql
CREATE UNIQUE INDEX IF NOT EXISTS idx_repos_user_name
  ON repositories(user_id, name) WHERE owner_type = 'user';

CREATE UNIQUE INDEX IF NOT EXISTS idx_repos_org_name
  ON repositories(org_id, name) WHERE org_id IS NOT NULL;
```

This allows user-owned repos and org-owned repos to have separate uniqueness scopes. A user can have a repo named `foo` without conflicting with an org that also has a repo named `foo`. The partial `WHERE` clause ensures the index only applies to relevant rows, avoiding NULL-in-unique-index issues with `org_id`.

## References

- See `_wiki: wal-mode.md` for WAL mode configuration and performance characteristics
- See `_wiki: rusqlite.md` for rusqlite patterns (params! macro, row mapping, error handling)
