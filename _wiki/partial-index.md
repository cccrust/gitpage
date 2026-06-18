# Partial Index（SQLite 條件式索引）

## 概述

Partial Index（部分索引/條件式索引）是 SQLite 3.8.0+ 引入的功能，允許在建立索引時指定 `WHERE` 條件，只有滿足條件的資料行才會被納入索引。Gitpage 使用 partial unique index 解決了使用者與組織雙重擁有權模型下的唯一性約束問題—同一個倉庫名稱可以在不同使用者/組織之間重複，但同一擁有者下不能重複。

## 問題描述

### 雙重擁有權的唯一性要求

Gitpage 的 `repositories` 表：

```sql
CREATE TABLE repositories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,      -- 使用者 ID
    org_id INTEGER,                 -- 組織 ID（可為 NULL）
    owner_type TEXT NOT NULL,       -- 'user' 或 'org'
    name TEXT NOT NULL,              -- 倉庫名稱
    -- ... 其他欄位
);
```

業務規則要求：
1. **使用者不能在同名下建立兩個同名倉庫**：`alice/foo` 只能存在一個
2. **組織不能在同名下建立兩個同名倉庫**：`myteam/foo` 只能存在一個
3. **不同使用者之間允許同名**：`alice/foo` 和 `bob/foo` 可以並存
4. **使用者與組織之間允許同名**：`alice/foo` 和 `alice-org/foo` 可以並存

### 傳統方案的問題

#### 方案一：UNIQUE(user_id, name)

```sql
CREATE UNIQUE INDEX idx_repos_name ON repositories(user_id, name);
```

問題：
- `org_id` != `user_id`，如果使用者 `alice` 的帳號 ID = 1，組織 `alice-org` 的 ID = 5，則 `alice/foo` 和 `alice-org/foo` 會分別以 `(1, foo)` 和 `(5, foo)` 儲存
- 但當一個組織的 `org_id` 跟一個使用者的 `user_id` 相同時（不可能，但資料庫不保證），可能有碰撞問題
- 無法表達 `org_id` 維度的唯一性

#### 方案二：UNIQUE(user_id, org_id, name)

```sql
CREATE UNIQUE INDEX idx_repos_name ON repositories(user_id, org_id, name);
```

問題：
- 對於使用者擁有的倉庫，`org_id = NULL`。在 SQL 中，NULL 在唯一索引中被視為不同的值，所以 `(1, NULL, foo)` 不會與 `(1, NULL, foo2)` 衝突
- 但 `null` 的處理在不同資料庫之間不一致
- 索引較大（包含三個欄位）

#### 方案三：使用單一唯一索引 + 應用層檢查

先查詢再插入，競態條件可能導致重複資料。

## Partial Index 的解決方案

### 語法

```sql
CREATE UNIQUE INDEX IF NOT EXISTS idx_repos_user_name
    ON repositories(user_id, name)
    WHERE owner_type = 'user';

CREATE UNIQUE INDEX IF NOT EXISTS idx_repos_org_name
    ON repositories(org_id, name)
    WHERE org_id IS NOT NULL;
```

### 如何運作

第一個索引 `idx_repos_user_name`：
- 只對 `owner_type = 'user'` 的行建立索引
- 保證同一個 `user_id` + `name` 在使用者倉庫中唯一

第二個索引 `idx_repos_org_name`：
- 只對 `org_id IS NOT NULL` 的行建立索引
- 保證同一個 `org_id` + `name` 在組織倉庫中唯一

### Gitpage 中的實作

```rust
// src/db/mod.rs — 資料庫遷移
// 第 147-151 行
// Partial unique indexes for user and org repo names
conn.execute_batch(
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_repos_user_name
        ON repositories(user_id, name) WHERE owner_type = 'user';
     CREATE UNIQUE INDEX IF NOT EXISTS idx_repos_org_name
        ON repositories(org_id, name) WHERE org_id IS NOT NULL;"
)?;
```

### 從舊約束遷移

在加入 org 支援之前，Gitpage 使用的約束是：

```sql
UNIQUE(user_id, name)
```

遷移過程（`src/db/mod.rs:100-145`）：

```rust
// 1. 檢查舊約束是否存在
let has_old_unique = conn
    .prepare("SELECT COUNT(*) FROM sqlite_master
              WHERE sql LIKE '%UNIQUE(user_id, name)%'")?
    .query_row([], |row| row.get::<_, i64>(0))?;

if has_old_unique > 0 {
    // 2. 建立新表（不含舊約束）
    conn.execute_batch("
        CREATE TABLE repositories_migrated (...);

        INSERT INTO repositories_migrated SELECT * FROM repositories;

        DROP TABLE repositories;
        ALTER TABLE repositories_migrated RENAME TO repositories;
    ")?;
}

// 3. 建立 partial index
conn.execute_batch("
    CREATE UNIQUE INDEX IF NOT EXISTS idx_repos_user_name
        ON repositories(user_id, name) WHERE owner_type = 'user';
    CREATE UNIQUE INDEX IF NOT EXISTS idx_repos_org_name
        ON repositories(org_id, name) WHERE org_id IS NOT NULL;
")?;
```

## Partial Index 的優點

### 更小的索引大小

傳統的複合索引 `(user_id, org_id, name)` 會為每一行建立索引，包括 `org_id IS NULL` 的行。Partial index 只對需要的行建立索引：

| 索引類型 | 索引行數 | 索引大小 |
|---------|---------|---------|
| UNIQUE(user_id, name) | 所有行 | 較小 |
| UNIQUE(user_id, org_id, name) | 所有行 | 最大 |
| Partial (user) + Partial (org) | 僅使用者行 + 僅組織行 | 最小 |

假設資料庫有 1000 個 repos，其中 800 個是使用者擁有，200 個是組織擁有：

| 索引 | 條目數 | 約略大小 |
|-----|-------|---------|
| `UNIQUE(user_id, org_id, name)` | 1000 | ~120KB |
| Partial user index | 800 | ~96KB |
| Partial org index | 200 | ~24KB |
| Partial total | 1000 | ~120KB（但查詢更快） |

雖然總大小相近，但**查詢效能更好**，因為每個索引的 B-tree 更淺、更集中。

### 更好的查詢效能

```sql
-- 查詢使用者倉庫 — 只需要掃描 idx_repos_user_name
SELECT * FROM repositories
WHERE user_id = ? AND owner_type = 'user' AND name = ?;

-- 查詢組織倉庫 — 只需要掃描 idx_repos_org_name
SELECT * FROM repositories
WHERE org_id = ? AND owner_type = 'org' AND name = ?;
```

Partial index 允許 SQLite 查詢規劃器做出更精確的選擇：
1. 查詢規劃器看到 `WHERE owner_type = 'user'`，立刻知道只有 `idx_repos_user_name` 是相關的
2. Partial index 的 B-tree 比全量索引更小，因此查詢路徑更短
3. 索引的選擇性（selectivity）更高，因為資料分群集中

### 語意清晰的唯一性保證

Partial unique index 直接編碼了業務邏輯：
- `WHERE owner_type = 'user'`：使用者下的倉庫名稱唯一
- `WHERE org_id IS NOT NULL`：組織下的倉庫名稱唯一

這比應用層檢查更可靠（資料庫層級的保證，不受應用 bug 影響）。

## Partial Index 的限制

### WHERE 條件限制

SQLite 對 partial index 的 WHERE 子句有限制：
1. **只能引用表的欄位**：不能使用子查詢或表達式
2. **必須是確定性的**：不能使用隨機函數或日期函數
3. **用於唯一索引時**：條件必須精確匹配插入/更新的值

### 維護成本

1. **確保條件匹配查詢**：如果查詢的 WHERE 條件與 partial index 的條件不匹配，規劃器可能不使用該索引
2. **更新約束**：當一行從使用者倉庫變更為組織倉庫（或反之），需要確保索引正確維護

## Partial Index 在 Gitpage 查詢中的應用

### 使用者倉庫查詢

```rust
// src/db/mod.rs:438
fn get_repo_by_user_and_name(&self, user_id: i64, name: &str) -> Result<Option<Repository>> {
    self.conn.prepare(
        "SELECT * FROM repositories
         WHERE user_id = ?1 AND name = ?2 AND owner_type = 'user'"
    )?.query_row(params![user_id, name], |row| { ... })
      .optional()
}
```

SQLite 查詢規劃器會選擇 `idx_repos_user_name` 索引進行精確查找。

### 組織倉庫查詢

```rust
// src/db/mod.rs:461
fn get_repo_by_org_and_name(&self, org_id: i64, name: &str) -> Result<Option<Repository>> {
    self.conn.prepare(
        "SELECT * FROM repositories
         WHERE org_id = ?1 AND name = ?2 AND owner_type = 'org'"
    )?.query_row(params![org_id, name], |row| { ... })
      .optional()
}
```

SQLite 查詢規劃器會選擇 `idx_repos_org_name` 索引。

### 列表查詢

```rust
// 使用者倉庫列表
"SELECT * FROM repositories WHERE user_id = ?1 AND owner_type = 'user' ORDER BY updated_at DESC"

// 組織倉庫列表
"SELECT * FROM repositories WHERE org_id = ?1 AND owner_type = 'org' ORDER BY updated_at DESC"
```

在這些範圍查詢中，partial index 同樣可以減少掃描的行數。

## EXPLAIN 查詢計劃

驗證 partial index 是否被使用的典型方法：

```sql
EXPLAIN QUERY PLAN
SELECT * FROM repositories
WHERE user_id = 1 AND owner_type = 'user' AND name = 'myproject';

-- 輸出示例：
-- |--SEARCH repositories USING INDEX idx_repos_user_name (user_id=? AND name=?)
```

如果沒有 partial index，查詢計劃可能是：

```sql
-- |--SEARCH repositories USING INDEX old_unique_index (user_id=? AND name=?)
-- |--USE WHERE owner_type = 'user'  (需要額外的過濾)
```

多了一步額外的 WHERE 過濾。

## Partial Unique Index 與傳統 UNIQUE 約束的比較

| 特性 | Partial Unique Index | UNIQUE 約束 |
|------|---------------------|-------------|
| 支援 WHERE 條件 | ✅ | ❌ |
| 可為部分資料建立 | ✅ | ❌（整張表） |
| NULL 處理 | 可由條件控制 | NULL 不被視為相等 |
| 索引大小 | 較小 | 較大 |
| DDL 相容性 | SQLite 3.8+ | 所有版本 |
| 遷移成本 | 低（CREATE INDEX） | 高（需重建表） |

## 與一般索引的互補

除了 partial unique index，Gitpage 還使用一般索引加速常見查詢：

```sql
-- 使用者名稱查詢（一般唯一索引）
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- 組織名稱查詢（一般唯一索引）
CREATE UNIQUE INDEX IF NOT EXISTS idx_orgs_name ON organizations(name);
```

Partial index 和一般索引共同構成了 Gitpage 的索引策略：
- **唯一性保證**：Partial unique index（repos）+ 一般 unique index（users, orgs）
- **查詢加速**：Partial unique index 同時承擔查詢加速的任務

## 參考資料

- [SQLite Partial Index](https://sqlite.org/partialindex.html) — SQLite 官方文檔
- [SQLite CREATE INDEX 文檔](https://sqlite.org/lang_createindex.html) — CREATE INDEX 語法
- [Indexing NULLs in SQLite](https://sqlite.org/nulls.html) — NULL 值在索引中的行為
- `src/db/mod.rs:100-151` — 資料庫遷移（從 UNIQUE 到 partial index）
- `src/db/mod.rs:438-465` — 使用者/組織倉庫查詢
- `_wiki/owner-resolution.md` — 擁有者解析模式
