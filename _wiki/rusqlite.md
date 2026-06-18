# rusqlite（Rust SQLite 繫結）

## 概述

rusqlite 是 SQLite 資料庫引擎的 Rust 語言繫結（binding），提供安全、符合 Rust 慣用法的 SQLite 操作介面。不同於透過 ODBC/JDBC 等中間層，rusqlite 直接嵌入 SQLite C 函式庫，無需獨立安裝或管理資料庫伺服器。Gitpage 使用 rusqlite 搭配 `tokio::sync::Mutex` 實現非同步安全的資料持久化。

## SQLite 的應用場景

### SQLite vs 客戶端-伺服器資料庫

| 特性 | SQLite | PostgreSQL / MySQL |
|------|--------|-------------------|
| 部署 | 無需伺服器行程 | 需獨立資料庫伺服器 |
| 設定 | 零設定 | 使用者、密碼、權限、連線池 |
| 大小 | < 1MB（函式庫） | 數十至數百 MB |
| 並發寫入 | 單寫入器（檔案鎖） | 多重寫入器 |
| 並發讀取 | 多重讀取器（WAL 模式） | 多重讀取器 |
| 備份 | 複製檔案即可 | pg_dump / mysqldump |
| 適合規模 | 單機、小團隊 | 企業級、大規模 |

對於 Gitpage 的目標（自託管、個人/小團隊、無需獨立資料庫服務），SQLite 是理想的選擇。

## rusqlite 使用方法

### 建立連線

```rust
use rusqlite::Connection;

// 建立新資料庫（如檔案不存在會自動建立）
let conn = Connection::open("data/gitpage.db")?;

// 記憶體資料庫（測試用）
let conn = Connection::open_in_memory()?;
```

### 執行語句

```rust
// execute：用於 INSERT, UPDATE, DELETE, CREATE 等（不回傳資料）
conn.execute(
    "INSERT INTO users (username, email, password_hash) VALUES (?1, ?2, ?3)",
    params![username, email, password_hash],
)?;

// query_row：用於查詢單一資料列
let user: User = conn.query_row(
    "SELECT id, username, email, password_hash, bio, avatar_url, created_at FROM users WHERE id = ?1",
    params![user_id],
    |row| {
        Ok(User {
            id: row.get(0)?,
            username: row.get(1)?,
            email: row.get(2)?,
            password_hash: row.get(3)?,
            bio: row.get(4)?,
            avatar_url: row.get(5)?,
            created_at: row.get(6)?,
        })
    },
)?;

// prepare + query：用於查詢多列
let mut stmt = conn.prepare(
    "SELECT id, name, is_private FROM repositories WHERE owner_id = ?1 AND owner_type = 'user'"
)?;
let repos: Vec<Repository> = stmt.query_map(params![user_id], |row| {
    Ok(Repository {
        id: row.get(0)?,
        name: row.get(1)?,
        is_private: row.get(2)?,
        ..Default::default()
    })
})?.collect::<Result<Vec<_>, _>>()?;
```

## Gitpage 中的資料庫操作

### Database 結構

```rust
// src/db/mod.rs
pub struct Database {
    conn: Arc<tokio::sync::Mutex<Connection>>,
}
```

`Arc` 用於多所有權共享，`Mutex` 確保一次只有一個 tokio 任務存取連線。

### 交易操作

```rust
pub fn create_repo(&self, owner_id: i64, owner_type: &str, name: &str, ...) -> Result<Repository, AppError> {
    let conn = self.conn.lock().unwrap();

    // 使用交易確保原子性
    conn.execute_batch("BEGIN TRANSACTION")?;

    let result = (|| -> Result<Repository, AppError> {
        // 1. 建立資料庫記錄
        conn.execute(
            "INSERT INTO repositories (owner_id, owner_type, name, description, is_private, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))",
            params![owner_id, owner_type, name, description, is_private],
        )?;
        let repo_id = conn.last_insert_rowid();

        // 2. 查詢剛建立的記錄
        let repo = conn.query_row(
            "SELECT * FROM repositories WHERE id = ?1",
            params![repo_id],
            |row| { /* 反序列化 */ },
        )?;

        // 3. 交易成功
        conn.execute("COMMIT", [])?;
        Ok(repo)
    })();

    match result {
        Ok(repo) => Ok(repo),
        Err(e) => {
            conn.execute("ROLLBACK", [])?;
            Err(e)
        }
    }
}
```

### 參數繫結

rusqlite 支援多種參數繫結方式：

```rust
// 位置參數 (?1, ?2, ...) — 建議使用
conn.execute("INSERT INTO t VALUES (?1, ?2, ?3)", params![a, b, c])?;

// 命名參數 (@name, :name)
conn.execute(
    "INSERT INTO t VALUES (@a, @b, @c)",
    named_params!{"@a": a, "@b": b, "@c": c},
)?;

// 匿名參數 (?)
conn.execute("INSERT INTO t VALUES (?, ?, ?)", params![a, b, c])?;
```

## 資料庫遷移

Gitpage 在 `run_migrations()` 中執行所有資料庫結構建立與變更：

```rust
pub fn run_migrations(conn: &Connection) -> Result<(), AppError> {
    // 每個 CREATE TABLE 使用 IF NOT EXISTS 確保冪等性
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            bio TEXT DEFAULT '',
            avatar_url TEXT DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS repositories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            owner_id INTEGER NOT NULL,
            owner_type TEXT NOT NULL DEFAULT 'user',
            org_id INTEGER,
            name TEXT NOT NULL,
            description TEXT DEFAULT '',
            is_private INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (org_id) REFERENCES organizations(id) ON DELETE CASCADE
        );

        -- 使用部分索引支援使用者/組織的獨特名稱
        CREATE UNIQUE INDEX IF NOT EXISTS idx_repos_user_unique
            ON repositories(owner_id, name) WHERE owner_type = 'user';
        CREATE UNIQUE INDEX IF NOT EXISTS idx_repos_org_unique
            ON repositories(owner_id, name) WHERE owner_type = 'org';
        "
    )?;

    // ALTER TABLE 遷移（用於新增欄位）
    let has_owner_type = conn.prepare(
        "SELECT owner_type FROM repositories LIMIT 1"
    ).is_ok();

    if !has_owner_type {
        conn.execute_batch(
            "ALTER TABLE repositories ADD COLUMN owner_type TEXT NOT NULL DEFAULT 'user';"
        )?;
    }

    // 更多遷移...（issues, PRs, stars 等表格）

    Ok(())
}
```

## 型別轉換

rusqlite 透過 `FromSql` 和 `ToSql` trait 處理 Rust 和 SQLite 型別的轉換：

```rust
// SQLite → Rust
let id: i64 = row.get(0)?;      // INTEGER → i64
let name: String = row.get(1)?;  // TEXT → String
let is_private: bool = row.get(2)?;  // INTEGER(0/1) → bool

// 自訂型別
#[derive(Debug)]
pub struct CustomType(i64);

impl rusqlite::types::FromSql for CustomType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        i64::column_result(value).map(CustomType)
    }
}
```

## 效能最佳化

```rust
// WAL 模式（改善並發）
conn.execute_batch("PRAGMA journal_mode=WAL")?;

// 外鍵約束
conn.execute_batch("PRAGMA foreign_keys=ON")?;

// 同步模式（NORMAL 比 FULL 快，但安全性略低）
conn.execute_batch("PRAGMA synchronous=NORMAL")?;

// 快取大小（提升大量查詢效能）
conn.execute_batch("PRAGMA cache_size=-8000")?; // 8MB

// 暫存儲存引擎
conn.execute_batch("PRAGMA temp_store=MEMORY")?;

// Busy timeout（等待而非立刻回傳鎖定錯誤）
conn.execute_batch("PRAGMA busy_timeout=5000")?;
```

## 參考資料

- [rusqlite crate](https://crates.io/crates/rusqlite)
- [SQLite 文件](https://www.sqlite.org/docs.html)
- [rusqlite GitHub](https://github.com/rusqlite/rusqlite)
- `src/db/mod.rs` — Gitpage 資料庫操作
- `src/db/models.rs` — 資料模型
