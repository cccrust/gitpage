# Staging Area（暫存區）

## 概述

Staging Area（暫存區）是 Git 的核心概念之一，位於工作目錄（Working Directory）與 Git 物件資料庫（Object Database）之間的中間層。Gitpage 將此概念延伸應用於其**檔案管理器**（File Manager）功能中，建立了一個位於 `data/staging/{owner}/{repo}/` 的實體暫存目錄，讓使用者可以像使用 Google Drive 或 Dropbox 一樣管理檔案，最後再一次性提交（commit）到 Git 倉庫。

## Git 原始暫存區概念

### 三棵樹架構

Git 的內部狀態管理基於「三棵樹」（Three Trees）模型：

```
Working Directory    ←    Staging Area (Index)    ←    HEAD Commit
(實際檔案系統)            (index，即將提交的內容)        (最後一次提交)
```

1. **HEAD**：指向當前分支最新 commit 的指標，代表倉庫的已知狀態
2. **Staging Area（Index）**：一個二進位檔案 `.git/index`，記錄了下一次 commit 要包含的檔案清單、模式、SHA-1
3. **Working Directory**：實際的檔案系統，供編輯器操作

### 典型工作流程

```bash
# 修改檔案
echo "hello" > file.txt

# 加入暫存區
git add file.txt       # 將 file.txt 的快照加入 index

# 提交
git commit -m "msg"    # 從 index 建構 tree，建立 commit 物件
```

### Index 內部結構

`.git/index` 是一個二進位檔案，其結構為：

```
Header:
  signature: "DIRC" (4 bytes)
  version: 2/3/4 (4 bytes)
  entry_count: N (4 bytes)

Entries (each 62+ bytes):
  ctime_s, ctime_ns, mtime_s, mtime_ns (各 4 bytes)
  dev, ino, mode, uid, gid, file_size (各 4 bytes)
  sha1 (20 bytes)
  flags (2 bytes)
  path_name (variable, null-terminated)

Extensions (optional):
  cache_tree, resolve_undo, untracked, fsmonitor

Trailer:
  SHA-1 checksum (20 bytes)
```

## Gitpage 的 Staging Area 實現

### 設計動機

傳統 Git 工作流程需要使用者熟悉命令列操作，對非技術使用者門檻較高。Gitpage 的檔案管理器提供了一個**視覺化、即時的檔案操作介面**，類似 Google Drive 的體驗：

1. 上傳/建立/刪除/重新命名檔案立即反映
2. 檔案預覽與編輯
3. 批量提交至 Git 倉庫

### 實體儲存結構

不同於 Git 的 `.git/index`（二進位索引檔案），Gitpage 使用實際的檔案系統目錄作為暫存區：

```
data/staging/{owner}/{repo}/
├── README.md          ← 使用者建立的檔案
├── src/
│   └── main.rs
├── new_file.txt
└── ... (上傳/建立的檔案)
```

此目錄位於 `{storage.base_path}/staging/{owner_name}/{repo_name}`，透過 `Config::staging_path()` 方法計算路徑：

```rust
pub fn staging_path(&self, username: &str, repo: &str) -> PathBuf {
    PathBuf::from(&self.storage.base_path)
        .join("staging")
        .join(username)
        .join(repo)
}
```

### 程式實作

Gitpage 的檔案管理器實作於 `src/handlers/files.rs`，提供完整的檔案 CRUD API：

#### 列出暫存區內容

`GET /api/repos/:id/tree?path=<path>` — 遍歷暫存區目錄：

```rust
pub async fn list_working_tree(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
    Query(params): Query<TreeParams>,
) -> Result<Json<Value>, AppError> {
    // 1. 取得儲存庫資訊以解析擁有者
    let repo = state.db.get_repo(repo_id)?;
    let owner = resolve_owner(&state.db, &repo)?;
    let staging_path = state.config.staging_path(&owner, &repo.name);

    // 2. 遍歷目錄，收集檔案資訊
    let mut entries = vec![];
    let full_path = staging_path.join(&params.path);
    for entry in fs::read_dir(full_path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = entry.file_type()?;
        entries.push(FileEntry {
            name,
            is_dir: file_type.is_dir(),
            size: if file_type.is_file() { Some(entry.metadata()?.len()) } else { None },
            modified: entry.metadata()?.modified()?.into(),
        });
    }

    Ok(Json(json!({ "entries": entries, "path": params.path })))
}
```

#### 寫入檔案

`PUT /api/repos/:id/files?path=<path>` — 建立或覆蓋檔案：

```rust
pub async fn write_file(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
    Query(params): Query<FilePathParams>,
    axum::Extension(user_id): axum::Extension<i64>,
    body: Bytes,
) -> Result<Json<Value>, AppError> {
    // 1. 路徑安全檢查（防止目錄穿越）
    let safe = safe_path(&params.path)?;

    // 2. 計算目標路徑
    let staging_path = compute_staging_path(&state, repo_id)?;
    let file_path = staging_path.join(&params.path);

    // 3. 確保父目錄存在
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // 4. 寫入檔案
    fs::write(&file_path, &body)?;

    Ok(Json(json!({ "success": true })))
}
```

#### 建立目錄

`POST /api/repos/:id/mkdir?path=<path>`：

```rust
pub async fn mkdir(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
    Query(params): Query<FilePathParams>,
) -> Result<Json<Value>, AppError> {
    let staging_path = compute_staging_path(&state, repo_id)?;
    let dir_path = staging_path.join(&params.path);
    fs::create_dir_all(&dir_path)?;
    Ok(Json(json!({ "success": true })))
}
```

#### 移動/重新命名

`POST /api/repos/:id/move?from=<from>&to=<to>`：

```rust
pub async fn move_file(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
    Query(params): Query<MoveParams>,
) -> Result<Json<Value>, AppError> {
    let staging_path = compute_staging_path(&state, repo_id)?;
    let from = staging_path.join(&params.from);
    let to = staging_path.join(&params.to);
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&from, &to)?;
    Ok(Json(json!({ "success": true })))
}
```

#### 提交暫存區至 Git

`POST /api/repos/:id/commit` — 將整個暫存區提交到 Git 倉庫：

```
staging/{owner}/{repo}/          libgit2 TreeBuilder
├── README.md       ──────────►  tree_builder.upsert("README.md", blob)
├── src/                         tree_builder.upsert("src", subtree_oid)
│   └── main.rs    ──────────►    tree_builder.upsert("src/main.rs", blob)
└── img/
    └── logo.png                 這層遍歷所有檔案，建構完整的 tree
```

內部流程（`commit_staging()` in `src/git/mod.rs`）：

1. 讀取 parent tree（目前 HEAD commit 的 tree 物件）
2. 初始化 `TreeBuilder`，載入 parent tree 的所有 entries
3. 遞迴走訪 staging 目錄，對每個檔案：建立或查找 blob → 插入/更新 tree
4. 對 staging 中不存在的 parent entry → 從 tree 移除
5. 寫出新的 tree 物件
6. 建立新的 commit 物件，指向此 tree
7. 更新 branch ref 指向新 commit
8. 保留暫存區內容不刪除（使用者可繼續編輯）

```rust
pub fn commit_staging(repo: &Repository, staging_path: &Path, branch: &str, message: &str) -> Result<Oid, Error> {
    // 1. 取得目前 HEAD commit 和 tree
    let head = repo.head()?.peel_to_commit()?;
    let parent_tree = head.tree()?;

    // 2. 從 staging 建構新 tree
    let tree_oid = build_tree_from_dir(repo, &parent_tree, staging_path, "")?;
    let new_tree = repo.find_tree(tree_oid)?;

    // 3. 建立簽章和 commit
    let sig = repo.signature()?;
    let commit_oid = repo.commit(
        Some(&format!("refs/heads/{}", branch)),
        &sig, &sig, message,
        &new_tree, &[&head],
    )?;

    // 4. 檢查 refs/heads/branch 是否存在，更新之
    Ok(commit_oid)
}
```

`build_tree_from_dir()` 是核心遞迴函數：

```rust
fn build_tree_from_dir(repo: &Repository, parent_tree: &Tree, dir: &Path, prefix: &str) -> Result<Oid, Error> {
    let mut tb = repo.treebuilder(Some(parent_tree))?;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let relative = if prefix.is_empty() { name.clone() } else { format!("{}/{}", prefix, name) };

        if entry.file_type()?.is_dir() {
            // 遞迴處理子目錄
            let subtree_oid = build_tree_from_dir(repo, parent_tree, &entry.path(), &relative)?;
            let subtree = repo.find_tree(subtree_oid)?;
            tb.insert(&name, &subtree, 0o40000)?;
        } else {
            // 建立 blob 並插入 tree
            let content = fs::read(entry.path())?;
            let blob_oid = repo.blob(&content)?;
            let mode = if is_executable(&entry.path()) { 0o100755 } else { 0o100644 };
            tb.insert(&name, blob_oid, mode)?;
        }
    }

    // 找出存在 parent tree 但不在 staging 中的 entry，將其移除
    for parent_entry in parent_tree.iter() {
        let name = parent_entry.name().unwrap_or("");
        if !staging_has_entry(dir, name) {
            tb.remove(name)?;
        }
    }

    Ok(tb.write()?)
}
```

#### 檢視狀態

`GET /api/repos/:id/status` — 比較暫存區與 HEAD 的差異：

```rust
pub async fn get_status(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    // 走訪 staging 目錄，與 HEAD tree 比較
    // 回傳新增、修改、刪除的檔案清單
    let changes = compute_changes(repo, &staging_path)?;
    Ok(Json(json!({
        "pending": changes.len() > 0,
        "changes": changes
    })))
}
```

## 安全性：路徑穿越防護

檔案管理器的核心安全挑戰是防止路徑穿越攻擊（Path Traversal Attack）。使用者可能嘗試透過 `../../etc/passwd` 存取系統檔案。Gitpage 使用 `safe_path()` 函數進行防護：

```rust
fn safe_path(path: &str) -> Result<String, AppError> {
    let path = path.trim_start_matches('/');
    if path.contains("..") {
        return Err(AppError::BadRequest("路徑不合法".into()));
    }
    Ok(path.to_string())
}
```

## 與傳統 Git Staging 的對比

| 特性 | 傳統 Git (Index) | Gitpage Staging |
|------|-----------------|-----------------|
| 儲存形式 | `.git/index` 二進位檔 | 實體檔案系統目錄 |
| 修改操作 | `git add` / `git rm` | Web UI 上傳/編輯/刪除 |
| 提交方式 | `git commit` | API 觸發 commit |
| 差異檢視 | `git diff --cached` | API 回傳變更清單 |
| 多人協作 | 直接操作同一 index | 單一使用者編輯 |
| 中間狀態 | 可部分暫存（部分 add） | 全部或無（全部提交） |

Gitpage 的設計犧牲了「部分暫存」的靈活性，但換來了更直觀的使用者體驗——所有對 staging 目錄的修改都是「已暫存」狀態，ready to commit。

## 自動部署觸發

Staging commit 完成後，會像 push 一樣觸發自動部署：

```rust
// commit 成功後
tokio::spawn(auto_deploy_pages(state.clone(), repo_id));
tokio::spawn(auto_deploy_app(state.clone(), repo_id));
```

這使得使用者可以在 Web UI 中編輯檔案後，一鍵 commit 並自動部署到 Pages 或 App 平台，實現類似 Vercel/Netlify 的體驗。

## 參考資料

- [Git Internals - The Index](https://git-scm.com/book/en/v2/Git-Internals-Git-Index)
- [Pro Git - 3.2 Git Basics - Recording Changes](https://git-scm.com/book/en/v2/Git-Basics-Recording-Changes)
- `src/handlers/files.rs` — Gitpage 檔案管理器 API 實作
- `src/git/mod.rs` — `commit_staging()` 和 `build_tree_from_dir()` 實作
- `src/config.rs` — `staging_path()` 路徑計算

## 圖表

```mermaid
flowchart LR
    subgraph Staging["Staging Dir"]
        F1[README.md]
        F2[src/main.rs]
        F3[img/logo.png]
    end
    subgraph Git["Bare Git Repo"]
        OBJ[(Objects)]
        REF[refs/heads/main]
    end
    subgraph TreeBuild["TreeBuilder"]
        TB[TreeBuilder]
        B1[blob: README.md]
        B2[blob: main.rs]
        B3[blob: logo.png]
        T1[tree: src]
        T2[tree: img]
    end
    Staging -->|fs::read_dir| TB
    TB -->|write()| OBJ
    TB -->|commit()| REF
    OBJ -->|deploy_pages| PAGES[(Pages Dir)]
    OBJ -->|deploy_app| APP[App Workspace]
```
