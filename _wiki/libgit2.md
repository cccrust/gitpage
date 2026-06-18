# libgit2

## 概述

libgit2 是一個可連結的 C 語言函式庫，提供程式化的 Git 儲存庫操作介面。不同於命令列 `git`，libgit2 允許開發者直接在自己的行程中操作 Git 物件模型，無需產生子行程或解析文字輸出。Gitpage 使用 `git2` crate（libgit2 的 Rust 繫結）來實現所有 Git 讀取操作（tree 瀏覽、blob 讀取、commit log、README 渲染）以及進階操作（3-way merge、tree 建構）。

## 歷史與設計哲學

libgit2 由 Shawn O. Pearce（Git 核心貢獻者，JGit 作者）於 2008 年發起。其設計目標是提供一個輕量、高效、可嵌入的 Git 實作核心，讓 GUI 用戶端、IDE 外掛、網頁應用等可以程式化操控 Git，而無需依賴命令列工具。

主要設計原則：

1. **純 C 實作**：無外部依賴，易於綁定其他語言
2. **線程安全**：物件查找和讀取操作可並行
3. **可自訂 backend**：支援自訂 ODB（物件資料庫）和 refs 儲存
4. **完整的 Git 物件模型**：支援 blob、tree、commit、tag、packfile 等

## Git 物件模型（Object Model）

理解 libgit2 的前提是理解 Git 的底層物件模型：

### Blob（二進位大物件）

儲存檔案內容的二元資料。Blob 沒有檔名、時間戳或任何元資料 — 它只是內容的 SHA-1 雜湊。

### Tree（目錄樹）

儲存目錄結構的物件。Tree 包含一系列的 entry，每個 entry 包含：
- `mode`：檔案模式（100644 = 一般檔案，100755 = 可執行，040000 = 目錄，120000 = 符號連結）
- `filename`：檔名
- `oid`：指向 blob 或其他 tree 的 SHA-1

```
tree 3b18e512dba79e4c8300dd08aeb37f8e728b8dad
100644 blob a906cb2a4a904a152e80877d4088654da1ed0cf4    README.md
100644 blob 47ca634df5c0d0fb84ea2d37d9c73dd7ef7b3b22    main.rs
040000 tree 7c5f5a4b8c3d2e1f0a9b8c7d6e5f4a3b2c1d0e       src/
```

### Commit（提交）

儲存一次提交的元資料：
- `tree`：指向提交時根目錄的 tree 物件
- `parent`：父 commit 的 SHA-1（合併提交有多個父 commit）
- `author`：作者（姓名 + 電子郵件 + 時間戳）
- `committer`：提交者（與 author 可能不同）
- `message`：提交訊息

### Tag（標籤）

指向特定 commit 的命名參照（輕量標籤直接指向 commit；附註標籤為 Git 物件，包含標籤訊息、簽名等）。

## Gitpage 中的 libgit2 應用

### 1. 儲存庫初始化

使用 `git2::Repository::init_bare()` 而非命令列的 `git init --bare`：

```rust
use git2::Repository;
let repo = Repository::init_bare(path)?;
```

並設定 `http.receivepack` 為 `true` 以允許 HTTP push：

```rust
repo.config()?.set_bool("http.receivepack", true)?;
```

對應於 `src/git/mod.rs` 的 `init_bare_repo()`。

### 2. 目錄與檔案瀏覽

Gitpage 的核心功能之一是在網頁上瀏覽 Git 儲存庫的目錄結構。這需要從 Git 物件模型中重建目錄樹：

```rust
// 取得 HEAD commit 的 tree
let repo = Repository::open_bare(path)?;
let head = repo.head()?.peel_to_commit()?;
let tree = head.tree()?;

// 遍歷 tree entries
for entry in &tree {
    let name = entry.name();
    let oid = entry.id();
    let kind = entry.kind(); // Blob 或 Tree
    let mode = entry.filemode();
}
```

實作於 `list_directory()` 函數。

### 3. 檔案內容讀取

讀取特定路徑的檔案內容：

```rust
// 從 root tree 沿路徑找到物件
let entry = tree.get_path(Path::new("src/main.rs"))?;
let blob = repo.find_blob(entry.id())?;
let content = blob.content(); // &[u8]
```

對應於 `get_file_content()`。

### 4. Commit Log

使用 `revwalk` 遍歷提交歷史：

```rust
let mut revwalk = repo.revwalk()?;
revwalk.push_head()?;
revwalk.set_sorting(git2::Sort::TIME)?; // 依時間排序

for oid in revwalk {
    let commit = repo.find_commit(oid?)?;
    let message = commit.message();
    let author = commit.author();
    let time = commit.time();
}
```

對應於 `get_commit_log()`。

### 5. README 渲染

```rust
// 找 README.md
let readme_entry = tree.get_path(Path::new("README.md"));
if let Ok(entry) = readme_entry {
    let blob = repo.find_blob(entry.id())?;
    let content = str::from_utf8(blob.content())?;
    // 交給 pulldown_cmark 渲染為 HTML
}
```

### 6. Tree 建構與 Staging Commit

檔案管理器的核心：從 staging 目錄建構一個新的 Git tree，並建立 commit。

```rust
let tree_builder = repo.treebuilder(Some(&parent_tree))?;
// 覆蓋或新增 entry
tree_builder.insert("new_file.txt", blob_id, 0o100644)?;
let new_tree_id = tree_builder.write()?;
let new_tree = repo.find_tree(new_tree_id)?;
// 建立 commit
let commit_id = repo.commit(
    Some("refs/heads/main"),
    &signature, &signature,
    "commit message",
    &new_tree,
    &[&parent_commit],
)?;
```

對應於 `build_tree_from_dir()` 和 `commit_staging()`。

### 7. 3-Way Merge（Pull Request 合併）

在 PR 合併時，libgit2 的 `merge_trees()` 進行三方合併：

```rust
let merge_result = repo.merge_trees(
    &base_tree,   // base commit 的 tree
    &ours_tree,   // 目標分支的 tree
    &theirs_tree, // 來源分支的 tree
    None,         // 選項
)?;
// merge_result 包含：
// - tree_id：合併後的 tree（無衝突時）
// - conflicts：衝突列表
```

實作於 `handlers/pulls.rs` 的 `merge_pr()`。

### 8. Diff 計算

計算兩個 tree 之間的差異以顯示 PR diff：

```rust
let mut diff = repo.diff_tree_to_tree(
    Some(&base_tree),
    Some(&head_tree),
    None,
)?;
diff.foreach(
    &mut |delta, progress| { ... },  // file callback
    ...
)?;
```

## 效能特性

| 操作 | 時間複雜度 | 說明 |
|------|-----------|------|
| 開啟儲存庫 | O(1) | 讀取 HEAD 和 config |
| Tree 遍歷 | O(n) | n = entries 數量 |
| Blob 讀取 | O(1) | 直接從 ODB 查找 |
| Revwalk | O(k log n) | k = commits 數量，n = 分支數 |
| Tree 建構 | O(n) | n = entries 數量 |
| 3-Way Merge | O(m+n) | m, n = 兩棵 tree 的 entry 數 |

## 風險與限制

1. **同步阻塞**：libgit2 操作是同步的，在 async context 中會阻塞 tokio worker 線程。Gitpage 透過 `tokio::task::spawn_blocking` 包裝。

2. **大物件記憶體**：大的 blob 全部載入記憶體，對大型二進位檔案不友好。可考慮 streaming read。

3. **部分 Git 功能未實作**：libgit2 不支援 git gc、reflog（早期）、subtree merge 等。如有需要仍得呼叫命令列 git。

4. **交叉編譯**：libgit2 的 C 程式碼在交叉編譯時可能需要額外設定。

5. **子模組支援有限**：部分子模組操作需要額外處理。

## 與 git2（Rust 繫結）的關係

`git2` crate 透過 `cc` build script 在編譯時自動下載並建構 libgit2 原始碼，提供安全的 Rust 封裝：

```toml
[dependencies]
git2 = "0.18"
```

Rust 繫結保持了與 C API 的 1:1 對應，但提供：
- 記憶體安全（透過 RAII 模式）
- 型別安全（enum 而非整數常數）
- 錯誤處理（Result 型別）

## 參考資料

- [libgit2 Official Site](https://libgit2.org/)
- [libgit2 GitHub](https://github.com/libgit2/libgit2)
- [git2 crate](https://crates.io/crates/git2)
- [Git Internals - Git Objects](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects)
- `src/git/mod.rs` — Gitpage 的 libgit2 操作封裝
- `src/handlers/pulls.rs` — 3-way merge 與 diff 實作
- `src/handlers/files.rs` — staging commit 實作
