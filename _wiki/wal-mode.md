# WAL Mode（Write-Ahead Logging）

## 概述

Write-Ahead Logging（WAL）是 SQLite 的一種日誌模式，用於改善並發讀取效能和寫入效能。傳統的 rollback journal 模式在寫入時鎖定整個資料庫檔案，而 WAL 模式將變更寫入獨立的日誌檔案，允許讀取操作不受寫入操作影響。Gitpage 在資料庫初始化時啟用 WAL 模式，並使用 `tokio::sync::Mutex` 確保資料庫連線的執行緒安全。

## Rollback Journal 模式的問題

在傳統的 journal 模式下，SQLite 的寫入流程如下：

1. 寫入開始
2. 將原始頁面複製到 journal 檔案
3. 直接修改資料庫頁面
4. 寫入確認
5. 刪除 journal 檔案（已提交）

### 核心問題

當一個寫入交易進行時：
- 資料庫檔案被獨佔鎖定
- 讀取操作必須等待（或是舊版本的 snapshot）
- 多個寫入操作無法並行

這對於讀取密集型應用（如網頁伺服器）是嚴重的效能瓶頸。

## WAL 模式的工作原理

WAL 模式改變了寫入策略：不直接修改資料庫檔案，而是將變更追加到一個獨立的 WAL 檔案：

```
資料庫檔案 (main.db)    ← 僅讀取（不直接寫入）
         │
         ├── 讀取路徑：從 main.db + WAL 中的新資料合併
         │
WAL 檔案 (main.db-wal)  ← 所有寫入操作追加到此
         │
         ├── checkpoint: 將 WAL 中的變更合併回 main.db
         │
SHM 檔案 (main.db-shm)  ← 共享記憶體，用於同步
```

### WAL 讀取協定

1. 讀取器搜尋 WAL 檔案，查看欲讀取頁面是否有未 checkpoint 的修改
2. 如果有，從 WAL 中讀取最新版本
3. 如果沒有，從主資料庫檔案讀取
4. 讀取器之間不互相阻塞

### WAL 寫入協定

1. 將修改追加到 WAL 檔案末尾
2. 更新 WAL index（記錄每個頁面的最新位置）
3. 返回寫入成功

## 在 Gitpage 中的應用

### 資料庫初始化

在 `src/db/mod.rs` 的 `run_migrations()` 中：

```rust
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self, AppError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL")?;
        conn.execute_batch("PRAGMA foreign_keys=ON")?;
        Self::run_migrations(&conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }
}
```

### 並發模型

在 Axum 的非同步環境中，多個請求可能同時存取資料庫。`tokio::sync::Mutex` 確保同一時間只有一個任務持有資料庫連線：

```
請求 A (READ)    請求 B (WRITE)    請求 C (READ)
    │                │                │
    ├─ lock().await ─┤                │
    │ (取得鎖)       │ (等待鎖)       │
    │ SELECT ...     │                │
    │ (讀取中)       │                │
    ├─ unlock() ─────┤                │
    │                ├─ lock().await ─┤
    │                │ (取得鎖)       ├─ lock().await (等待)
    │                │ INSERT ...     │
    │                │ (寫入中)       │
    │                ├─ unlock() ─────┤
    │                │                ├─ lock().await
    │                │                │ (取得鎖)
    │                │                │ SELECT ...
    │                │                ├─ unlock()
```

## WAL vs Journal 模式

| 特性 | Rollback Journal | WAL |
|------|-----------------|-----|
| 讀取並發 | 單一讀取器 | 多個並發讀取器 |
| 寫入阻塞讀取 | ✅ 是 | ❌ 否 |
| 讀取阻塞寫入 | ✅ 是 | ❌ 否 |
| 災難回復 | 較慢（需重做 journal） | 較快（WAL 可重放） |
| 檔案數量 | 1（+ journal 暫存） | 3（.db + .wal + .shm） |
| 效能（讀取密集） | ❌ 差 | ✅ 佳 |
| 效能（寫入密集） | ✅ 中 | ✅ 佳 |

## Checkpoint 機制

WAL 檔案會持續增長，需要定期 checkpoint：

```rust
// 自動 checkpoint：每 1000 頁（約 4MB）後自動合併
conn.execute_batch("PRAGMA wal_autocheckpoint=1000")?;

// 手動 checkpoint（TRUNCATE 模式會截斷 WAL）
conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
```

## tokio::sync::Mutex vs std::sync::Mutex

Gitpage 使用 `tokio::sync::Mutex` 而非標準 `std::sync::Mutex`：

| 特性 | tokio::sync::Mutex | std::sync::Mutex |
|------|-------------------|-----------------|
| await 支援 | ✅ `.lock().await` | ❌ 阻塞線程 |
| 適用場景 | 長時間持有鎖（含 I/O） | 短時間持有鎖 |
| 工作竊取 | ✅ 持有鎖時可讓出線程 | ❌ 阻塞整個 Worker |
| 開銷 | 略高 | 極低（~40ns） |

由於 SQLite 操作可能包含磁碟 I/O，使用 `tokio::sync::Mutex` 可避免阻塞 Worker 線程：

```rust
async fn read_user(db: &Database, user_id: i64) -> Result<User, AppError> {
    let conn = db.conn.lock().await;  // 非阻塞等待
    let user = conn.query_row("SELECT ...", params, |row| { ... })?;
    Ok(user)
    // 鎖在 guard drop 時自動釋放
}
```

## 單連線 vs 連線池

Gitpage 採用**單連線 + Mutex** 而非連線池：

| 方案 | 優點 | 缺點 |
|------|------|------|
| 單連線 + Mutex | 實作簡單，無 race condition | 所有操作序列化 |
| 連線池（r2d2） | 高並發讀取 | 實作複雜，需處理交易隔離 |
| WAL + 多連線 | 真正的並發讀取 | SQLite WAL 在多寫入器時仍會鎖 |

對於 Gitpage 的使用規模（個人或小團隊），單連線 + Mutex 的效能已足夠。

## SQLite 交易隔離層級

```rust
// Gitpage 使用預設的 SERIALIZABLE 隔離層級
// WAL 模式下允許 SNAPSHOT 隔離
conn.execute_batch("PRAGMA read_uncommitted=0")?;
// 0 = SERIALIZABLE（預設，最安全）
// 1 = READ UNCOMMITTED（髒讀取，不建議）
```

在 WAL 模式下，SERIALIZABLE 隔離層級仍允許多個讀取器並行，僅寫入器需要互斥。

## 災難恢復

WAL 模式在系統崩潰後的恢復速度優於 journal 模式：

```rust
// SQLite 在打開資料庫時自動執行恢復
// - 檢查 WAL 檔案是否存在且完整
// - 重放未 checkpoint 的交易（若最後一個 frame 完整）
// - 捨棄未完成的交易（若最後一個 frame 不完整）
// 所有操作自動完成，無需手動干預
```

## 參考資料

- [SQLite WAL Documentation](https://www.sqlite.org/wal.html)
- [SQLite Journal Modes](https://www.sqlite.org/pragma.html#pragma_journal_mode)
- [tokio::sync::Mutex](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html)
- `src/db/mod.rs` — Database 連線初始化
