# Gitpage 2.x 開發藍圖

```
v2.0 ─────── v2.1 ─────── v2.2 ─────── v2.3
Issue/PR     後台設定      Star/Watch   通知整合
基礎協作     管理         社交機制      即時反饋
```

核心思路：讓 gitpage 不只是 Git 託管 + Pages/App 部署，而是具備完整協作功能的平台，在 process 與 docker 模式皆可使用。

---

## 來回對齊

所有 v2.x 功能須同時滿足：

- **Docker 模式**：API + 資料表操作透過容器 exec 或直接 DB 讀寫
- **Process 模式**：完全相同的 API，無容器依賴
- **libgit2**：Fork 用 git clone (bare)，Merge 用 git merge-tree / commit tree 操作
- **無 git 後端指令**：所有 git 操作透過 libgit2 或 git2 crate 完成，不 spawn git CLI
- **前端**：React 19、SPA 路由、中文介面

---

## v2.0 — Issue 與 Pull Request

### Issue 系統

目標：類似 GitHub Issues，支援 markdown 內容、標籤、指派、留言。

#### 資料表

```sql
-- issues
CREATE TABLE issues (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    number INTEGER NOT NULL,                          -- repo 內序號 (自然鍵)
    title TEXT NOT NULL,
    body TEXT NOT NULL DEFAULT '',
    state TEXT NOT NULL DEFAULT 'open',                -- open / closed
    author_id INTEGER NOT NULL REFERENCES users(id),
    assignee_id INTEGER REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    closed_at TEXT,
    UNIQUE(repo_id, number)
);

-- issue_labels (repo-level label definition)
CREATE TABLE issue_labels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '#0366d6',             -- hex color
    UNIQUE(repo_id, name)
);

-- issue_label_map
CREATE TABLE issue_label_map (
    issue_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    label_id INTEGER NOT NULL REFERENCES issue_labels(id) ON DELETE CASCADE,
    PRIMARY KEY(issue_id, label_id)
);

-- issue_comments
CREATE TABLE issue_comments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    issue_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    author_id INTEGER NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

#### API (後綴 `/api/repos/:repo_id`)

| Method | Path | Handler | 功能 |
|--------|------|---------|------|
| GET | `/issues` | `list_issues` | 列出 issue（?state=open&label=bug&assignee=1） |
| POST | `/issues` | `create_issue` | 開 issue |
| GET | `/issues/:number` | `get_issue` | 檢視 issue（含 labels + comments） |
| PUT | `/issues/:number` | `update_issue` | 修改 title/body/state/assignee/labels |
| DELETE | `/issues/:number` | `delete_issue` | 刪除 issue |
| POST | `/issues/:number/comments` | `add_comment` | 留言 |
| PUT | `/issues/:number/comments/:id` | `update_comment` | 編輯留言 |
| DELETE | `/issues/:number/comments/:id` | `delete_comment` | 刪除留言 |
| GET | `/labels` | `list_labels` | 列出 repo 標籤 |
| POST | `/labels` | `create_label` | 新增標籤 |
| PUT | `/labels/:id` | `update_label` | 修改標籤 |
| DELETE | `/labels/:id` | `delete_label` | 刪除標籤 |

#### Issue Number 自動遞增

```
SELECT COALESCE(MAX(number), 0) + 1 FROM issues WHERE repo_id = ?
```

用 `INSERT ... SELECT` 或獨立 query+insert 避免 race（repo 層級，非 global）。

#### 前端頁面

| 路徑 | 元件 | 說明 |
|------|------|------|
| `/repo/:id/issues` | `IssueList` | 篩選/搜尋/分頁 issue 列表 |
| `/repo/:id/issues/new` | `IssueNew` | 開 issue (title + body editor + label picker) |
| `/repo/:id/issues/:number` | `IssueDetail` | 主體 + 留言串 + 側欄標籤/指派 |

### Pull Request 系統

目標：Fork → branch → compare → PR → merge，完整協作流程。

#### 協作模型

```
Fork 示意
原始 repo (upstream): A/myapp  ─── main
                          │
Fork (origin):            B/myapp  ─── main
                                         │
                                        fix-bug  ← 修改後發 PR
                                         │
                                     A/myapp 比較 main..B:fix-bug
```

#### 資料表

```sql
-- pull_requests
CREATE TABLE pull_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    number INTEGER NOT NULL,                           -- repo 內序號
    title TEXT NOT NULL,
    body TEXT NOT NULL DEFAULT '',
    state TEXT NOT NULL DEFAULT 'open',                -- open / closed / merged
    author_id INTEGER NOT NULL REFERENCES users(id),
    head_repo_id INTEGER NOT NULL REFERENCES repositories(id),
    head_ref TEXT NOT NULL,                            -- "main" or "user:branch"
    base_ref TEXT NOT NULL,                            -- 目標分支
    merge_commit_sha TEXT,                             -- merge 後產生的 commit SHA
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    closed_at TEXT,
    merged_at TEXT,
    UNIQUE(repo_id, number)
);

-- pr_reviews (optional v2.0.1)
CREATE TABLE pr_reviews (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pr_id INTEGER NOT NULL REFERENCES pull_requests(id) ON DELETE CASCADE,
    reviewer_id INTEGER NOT NULL REFERENCES users(id),
    state TEXT NOT NULL DEFAULT 'pending',             -- pending / approved / changes_requested
    body TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

#### Fork API

| Method | Path | Handler | 功能 |
|--------|------|---------|------|
| POST | `/api/repos/:id/forks` | `fork_repo` | Fork 到登入使用者名下 |

**fork_repo 實作細節**：

1. 驗證原始 repo 存在且可讀（public 或 owner）
2. 用 `libgit2` 的 `git_clone_bare()` 複製 bare repo 到 `data/repos/{user}/{repo}.git`
3. 新增 `data/staging/{user}/{repo}/` 目錄
4. 寫入 DB：`repositories` 表，`forked_from` 指向原始 repo_id
5. 回傳新的 Repo 物件

```rust
// src/handlers/repos.rs
pub async fn fork_repo(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let original = state.db.get_repo_by_id(repo_id).await?
        .ok_or(AppError::NotFound("Repository not found".into()))?;
    // check permissions...
    let user = state.db.find_user_by_id(user_id).await?.unwrap();
    let new_path = state.config.repo_path(&user.username, &original.name);
    git2::Repository::clone_bare(&original.path, &new_path)?;
    // insert into DB with forked_from
}
```

#### Pull Request API

| Method | Path | Handler |
|--------|------|---------|
| POST | `/api/repos/:repo_id/pulls` | `create_pr` |
| GET | `/api/repos/:repo_id/pulls` | `list_prs` |
| GET | `/api/repos/:repo_id/pulls/:number` | `get_pr` |
| PUT | `/api/repos/:repo_id/pulls/:number` | `update_pr` |
| POST | `/api/repos/:repo_id/pulls/:number/merge` | `merge_pr` |
| POST | `/api/repos/:repo_id/pulls/:number/comments` | `pr_comment` |

**create_pr 流程**：

1. 接收 `{ title, body, head_repo_id, head_ref, base_ref }`
2. 驗證 head_repo 存在、base_ref 在目標 repo 存在
3. 產生 PR number（同 issue 機制）
4. 寫入 DB，回傳 PR 資訊

**merge_pr 流程**（git merge 實作，無 git CLI）：

```rust
use git2::{Repository, MergeOptions, MergePreference};

fn merge_bare(repo: &Repository, head_oid: Oid, base_ref: &str) -> Result<Oid, Error> {
    // 1. 將 head_repo 加入 remote
    // 2. git fetch head_ref
    // 3. 建立 merge base (merge-base)
    // 4. git merge-tree (index 層級)
    // 5. 若無 conflict → 建立 merge commit
    // 6. 更新 base_ref
}
```

libgit2 的 merge 支援：

```rust
let merge_base = repo.merge_base(base_oid, head_oid)?;
let merge_tree = repo.merge_trees(&base_tree, &head_tree, &base_tree, None)?;
// 檢查 conflict
let index = merge_tree.index();
if index.has_conflicts() { return Err("conflict".into()); }
// 建立 merge commit
let tree_oid = index.write_tree_to(&repo)?;
let merge_commit = repo.commit(...)?;
```

#### PR Compare / Diff

| Method | Path | Handler |
|--------|------|---------|
| GET | `/api/repos/:repo_id/compare/:base...:head` | `compare_commits` |

libgit2 diff 實作：

```rust
fn compare(repo: &Repository, base: &str, head: &str) -> Result<Vec<DiffEntry>, Error> {
    let base_tree = repo.find_tree(repo.revparse_single(base)?.peel_to_tree()?)?;
    let head_tree = repo.find_tree(repo.revparse_single(head)?.peel_to_tree()?)?;
    let mut diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
    let mut entries = vec![];
    diff.foreach(&mut |delta, _| {
        entries.push(DiffEntry {
            status: delta.status(),
            old_path: delta.old_file().path().map(|p| p.to_string_lossy().to_string()),
            new_path: delta.new_file().path().map(|p| p.to_string_lossy().to_string()),
        });
        true
    }, None, None, None)?;
    Ok(entries)
}
```

#### 前端 PR 頁面

| 路徑 | 元件 | 說明 |
|------|------|------|
| `/repo/:id/pulls` | `PRList` | PR 列表（可切換 open/closed/merged） |
| `/repo/:id/pulls/new` | `PRNew` | 選 head repo + branch、base branch、title/body |
| `/repo/:id/pulls/:number` | `PRDetail` | Diff 檢視、留言、Merge 按鈕 |
| `/repo/:id/pulls/:number/files` | `PRFiles` | 逐檔案 diff |
| `/repo/:id/compare/:base...:head` | `PRCompare` | 比較頁面，可導向開 PR |

#### Repository fork 相關欄位

```sql
ALTER TABLE repositories ADD COLUMN forked_from INTEGER REFERENCES repositories(id);
```

前端 Fork 按鈕顯示在 repo header，點擊直接執行 fork API。

---

## v2.1 — 後台設定管理

目標：統一管理 account + project 設定，不需要直接編輯 config.toml。

### Account Settings（使用者層級）

| 頁面 | 功能 |
|------|------|
| `/settings/profile` | Avatar、Bio、顯示名稱（現有） |
| `/settings/account` | Email、密碼（現有）、刪除帳號 |
| `/settings/ssh-keys` | SSH public keys 管理（現有） |
| `/settings/notifications` | Email/webhook 通知偏好 |
| `/settings/tokens` | Personal Access Token 管理 |

#### Personal Access Token

```sql
CREATE TABLE access_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,                                -- e.g. "my-laptop"
    token_hash TEXT NOT NULL,                          -- SHA-256 of token
    scopes TEXT NOT NULL DEFAULT 'repo',               -- "repo", "repo:admin", "all"
    last_used_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT
);
```

Token 格式：`gpt_<random-hex>`（prefix 方便識別），hash 存 DB，明碼只顯示一次。

### Project Settings（Repo 層級）

| 路徑 | 功能 |
|------|------|
| `/repo/:id/settings` | 基本設定（name、description、visibility）— 現有 |
| `/repo/:id/settings/pages` | Pages 部署設定 — 現有 |
| `/repo/:id/settings/app` | App 部署設定 — 現有 |
| `/repo/:id/settings/deploy-keys` | Deploy SSH keys（唯讀 repo 存取） |
| `/repo/:id/settings/secrets` | CI/CD 環境變數 secrets（加密儲存） |
| `/repo/:id/settings/collaborators` | 協作者管理（非 owner 寫入權限） |
| `/repo/:id/settings/branches` | Branch protection rules |

#### Collaborators

```sql
CREATE TABLE repo_collaborators (
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permission TEXT NOT NULL DEFAULT 'write',           -- read / write / admin
    PRIMARY KEY (repo_id, user_id)
);
```

所有需要寫入的 API 檢查：owner 或 collaborator with `write`/`admin`。

#### Secrets（加密）

```sql
CREATE TABLE repo_secrets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    encrypted_value BLOB NOT NULL,                     -- AES-256-GCM
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(repo_id, name)
);
```

#### Branch Protection

```sql
CREATE TABLE branch_protection (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    pattern TEXT NOT NULL,                             -- "main", "release/*"
    require_pr BOOLEAN NOT NULL DEFAULT 1,
    require_approvals INTEGER NOT NULL DEFAULT 1,
    dismiss_stale_reviews BOOLEAN NOT NULL DEFAULT 1,
    UNIQUE(repo_id, pattern)
);
```

---

## v2.2 — Star / Watch / Fork 社交機制

目標：類似 GitHub 的 star（書籤/指標）、watch（通知）、fork 計數。

### 資料表

```sql
-- stars (user repo bookmark)
CREATE TABLE stars (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, repo_id)
);

-- watches (notification subscription)
CREATE TABLE watches (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    watch_type TEXT NOT NULL DEFAULT 'participating',  -- all / participating / ignore
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, repo_id)
);
```

### API

| Method | Path | Handler |
|--------|------|---------|
| PUT | `/api/repos/:id/star` | `star_repo` |
| DELETE | `/api/repos/:id/star` | `unstar_repo` |
| GET | `/api/repos/:id/stars` | `list_stargazers` |
| PUT | `/api/repos/:id/watch` | `watch_repo` |
| DELETE | `/api/repos/:id/watch` | `unwatch_repo` |
| GET | `/api/users/:username/stars` | `list_user_stars` |

### Repo 快取計數

```sql
ALTER TABLE repositories ADD COLUMN stars_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE repositories ADD COLUMN forks_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE repositories ADD COLUMN watch_count INTEGER NOT NULL DEFAULT 0;
```

透過 trigger 或 application-level 更新（INSERT star → UPDATE repos SET stars_count = stars_count + 1）。

### 前端

- Repo header 顯示 Star / Fork / Watch 計數 + 按鈕
- Dashboard「Starred Repos」分頁
- User profile 顯示星數

### Fork 完成項目

v2.0 的 fork API + v2.2 的 fork 計數合併：
- Fork 按鈕（repo header）
- Fork 計數顯示
- `forked_from` 欄位查詢

---

## v2.3 — 通知整合

目標：Issue/PR 異動時通知相關使用者（email / webhook / 站內通知）。

### 資料表

```sql
CREATE TABLE notifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type TEXT NOT NULL,                                -- issue_created / pr_created / mention / ...
    source_type TEXT NOT NULL,                         -- issue / pull_request / ...
    source_id INTEGER NOT NULL,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    body TEXT,
    is_read BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### API

| Method | Path | Handler |
|--------|------|---------|
| GET | `/api/notifications` | `list_notifications` |
| PUT | `/api/notifications/:id/read` | `mark_read` |
| PUT | `/api/notifications/read-all` | `mark_all_read` |
| GET | `/api/notifications/count` | `unread_count` |

### 前端

- 導航列顯示未讀數徽章
- `/notifications` 頁面

---

## DB Migration 策略

每個 .0 版本增加一組 migration（`src/db/mod.rs`）：

```rust
// v2.0.0
if version < 20250601 {
    tx.execute_batch("CREATE TABLE issues (...); CREATE TABLE issue_labels (...) ...")?;
    tx.execute("ALTER TABLE repositories ADD COLUMN forked_from ...", [])?;
}

// v2.1.0
if version < 20250701 {
    tx.execute_batch("CREATE TABLE access_tokens (...); CREATE TABLE repo_collaborators (...) ...")?;
}

// v2.2.0
if version < 20250801 {
    tx.execute_batch("CREATE TABLE stars (...); CREATE TABLE watches (...) ...")?;
    tx.execute("ALTER TABLE repositories ADD COLUMN stars_count ...", [])?;
}

// v2.3.0
if version < 20250901 {
    tx.execute_batch("CREATE TABLE notifications (...)")?;
}
```

當前 migration version: `20250101` (v1.x)

---

## 測試策略

每個功能模組新增 test script：

```bash
./test_issues.sh          # issue CRUD + labels + comments
./test_pr.sh              # fork → branch → PR → merge
./test_star.sh            # star/unstar/list
./test_collab.sh          # collaborator permission check
```

現有測試維持不變：

- `test.sh`（無 Docker）
- `test_docker.sh`（Docker 共用容器）
- `test_docker_mode.sh`（每人獨立容器 + app deploy）

新的 API 測試原則：

1. 先寫 `test_{feature}.sh`（純 API 驗證）
2. 確保 process mode 和 docker mode 共用同一組 handler（只在 deploy/exec 層有分歧）
3. Docker 端只需驗證 API 相容性，不需要為每項功能重跑容器測試

---

## 檔案變更清單（估算）

### 後端

| 檔案 | 異動 |
|------|------|
| `Cargo.toml` | 可能需要加密 crate（aes-gcm） |
| `src/db/mod.rs` | 4 組 migration、8+ 新查詢方法 |
| `src/db/models.rs` | Issue、PR、Star、Watch、Notification 等 struct |
| `src/handlers/issues.rs` | 新檔案：issue CRUD |
| `src/handlers/pulls.rs` | 新檔案：PR CRUD + merge |
| `src/handlers/repos.rs` | 新增 `fork_repo` |
| `src/handlers/settings.rs` | 新檔案：collaborator、secret、token CRUD |
| `src/handlers/stars.rs` | 新檔案：star/watch API |
| `src/handlers/notifications.rs` | 新檔案：通知 API |
| `src/app.rs` | 35+ 新路由 |
| `src/utils/encrypt.rs` | 新檔案：AES-256-GCM secrets 加密 |

### 前端

| 路徑 | 說明 |
|------|------|
| `src/pages/IssueList.tsx` | Issue 列表 |
| `src/pages/IssueNew.tsx` | 開 issue |
| `src/pages/IssueDetail.tsx` | Issue 檢視 + 留言 |
| `src/pages/PRList.tsx` | PR 列表 |
| `src/pages/PRNew.tsx` | 開 PR |
| `src/pages/PRDetail.tsx` | PR 檢視 + diff |
| `src/pages/PRCompare.tsx` | Branch compare |
| `src/pages/SettingsTokens.tsx` | Personal access tokens |
| `src/pages/SettingsCollaborators.tsx` | 協作者管理 |
| `src/pages/SettingsSecrets.tsx` | Secrets 管理 |
| `src/pages/SettingsNotificationsPrefs.tsx` | 通知偏好 |
| `src/pages/SettingsBranches.tsx` | Branch protection |
| `src/pages/Notifications.tsx` | 通知列表 |
| `src/api.ts` | 35+ 新 API 函數 |
| `src/App.tsx` | 25+ 新路由 |

---

## 優先順序建議

```
Phase 1 (v2.0)
  ├── Issue CRUD 資料表 + API + 前端   (2-3 天)
  ├── Fork API + 頁面按鈕                (1 天)
  ├── PR 資料表 + API                   (2 天)
  ├── git merge (libgit2)               (2 天)
  ├── PR Compare / Diff                  (1 天)
  └── PR 前端頁面                        (2 天)

Phase 2 (v2.1)
  ├── Access Token                       (1 天)
  ├── Collaborators                      (1 天)
  ├── Secrets (加密)                     (1 天)
  └── Branch Protection                  (1 天)

Phase 3 (v2.2)
  ├── Star / Watch 資料表 + API          (1 天)
  ├── 前端 Star/Watch 按鈕 + 計數        (1 天)
  └── User stars 頁面                    (0.5 天)

Phase 4 (v2.3)
  ├── 通知資料表 + API                   (1 天)
  ├── 站內通知頁面 + 未讀徽章            (1 天)
  └── Email 通知（可選）                  (2 天)
```

---

## 技術債務

- `src/handlers/` 檔案會變多，考慮模組化（每個功能一個目錄或多檔案模組）
- Issue/PR number 在同一 repo 共用序列？GitHub 是分開的（issue 和 PR 各自編號，不會衝突但問題列表混合）。建議：獨立編號，不混合。
- Merge conflict 處理：第一版先拒絕有 conflict 的 merge（回傳 conflict file list），不提供線上 resolver
- Secrets 加密金鑰放在 config.toml `[secrets] encryption_key`，生產環境應透過環境變數 `SECRETS_ENCRYPTION_KEY`
- 通知可先做站內通知，email 之後再補

---

## 未納入 v2.x

| 功能 | 說明 | 原因 |
|------|------|------|
| OAuth / SSO | GitHub/Google 登入 | 需外部服務 |
| Serverless Functions | 輕量函數託管 | 需 runtime 支援 |
| WebSocket Proxy | App proxy 支援 WebSocket | 需 axum upgrade |
| Multi-node | 叢集 | 需分散式儲存 |
| CI/CD Pipeline YAML | 自訂 pipeline | 先以 webhook 為基礎 |
| Container Registry | Docker image 儲存 | 需大量儲存空間 |
