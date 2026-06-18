# Revwalk（Git Commit 拓樸遍歷）

## 概述

Revwalk（Revision Walk）是 libgit2 提供的 commit 歷史遍歷機制，用於從一個或多個起始 commit 出發，遍歷 Git 的 commit DAG（Directed Acyclic Graph，有向無環圖）。Gitpage 使用 Revwalk 在 commits 頁面中顯示分支的提交歷史，以反向時間順序列出 commit SHA、訊息、作者和時間。

## Git Commit DAG 結構

### Commit 物件之間的關係

Git 的 commit 歷史形成一個 DAG，每個 commit 節點指向零個或多個父 commit：

```
    A (初始 commit，無父節點)
     │
     ▼
    B (A 的 child)
     │
     ├──── C (B 的 child)
     │      │
     ▼      ▼
     D     E (合併 commit，有兩個父節點)
     │      │
     ▼      ▼
     F     G
           │
           ▼
          H (HEAD)
```

D 的父節點是 C，E 的父節點是 B 和 C（合併），F 的父節點是 D，H 的父節點是 G。

### 每個 Commit 的儲存結構

```rust
// Commit 在 Git 中的物件結構（由 libgit2 的 git2::Commit 表示）
struct Commit {
    tree:  TreeOID,           // 指向 commit 時的目錄快照
    parents: Vec<CommitOID>,  // 父 commit 的 OID 列表
    author: Signature,        // { name, email, time }
    committer: Signature,
    message: String,          // commit message
}
```

### DAG 的特色

與線性歷史不同，Git 的 DAG 具有：
1. **多個起點**：多個 root commit（無父節點的 commit）
2. **分岔**：branch divergence（不同 commit 有不同的後代）
3. **合併**：merge commits（有多個父節點的 commit）
4. **無環**：不存在從一個 commit 出發能走回自身的路徑

## Revwalk 的運作原理

### 遍歷演算法

Revwalk 使用**優先權佇列**（priority queue）進行遍歷：

```
Algorithm Revwalk(start_oids, sorting_mode):
    queue = empty priority queue
    visited = set of "known" commits (用於避免重複)
    
    for oid in start_oids:
        push oid into queue
    
    while queue is not empty:
        oid = pop highest priority from queue
        if oid in visited:
            continue
        add oid to visited
        yield oid
        
        commit = load commit object for oid
        for parent in commit.parents:
            push parent into queue
```

### libgit2 的 Revwalk API

```rust
// src/git/mod.rs — Gitpage 的 Revwalk 使用
pub fn get_commit_log(
    repo_path: &str,
    branch: &str,
    limit: usize,
) -> Result<Vec<(String, String, String, String)>, AppError> {
    let repo = git2::Repository::open_bare(repo_path)?;

    // 1. 將分支名稱轉換為 ref
    let branch_ref = format!("refs/heads/{}", branch);
    let oid = match repo.refname_to_id(&branch_ref) {
        Ok(oid) => oid,
        Err(_) => return Ok(Vec::new()),
    };

    // 2. 建立 Revwalk 實例
    let mut revwalk = repo.revwalk()?;

    // 3. 將分支 head 的 OID 加入遍歷佇列
    revwalk.push(oid)?;

    // 4. 設定排序模式
    revwalk.set_sorting(git2::Sort::TIME)?;

    // 5. 迭代 commit
    let mut commits = Vec::new();
    for (i, oid) in revwalk.enumerate() {
        if i >= limit { break; }
        if let Ok(oid) = oid {
            if let Ok(commit) = repo.find_commit(oid) {
                let sha = oid.to_string();
                let short_sha = sha[..8].to_string();
                let message = commit.message().unwrap_or("").to_string();
                let author = commit.author().name().unwrap_or("unknown").to_string();
                let time = commit.time().seconds();
                let datetime = chrono::DateTime::from_timestamp(time, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default();
                commits.push((short_sha, message, author, datetime));
            }
        }
    }
    Ok(commits)
}
```

### Revwalk 的隱含隱含去重

libgit2 的 Revwalk 內部維護了一個 `seen` 集合，確保同一個 commit 不會被處理兩次。這對於處理合併 commit 特別重要—一個 commit 可以是多個分支的共同祖先，但它只會在輸出中出現一次。

## 排序模式

### git2::Sort::TIME（時間排序）

```
H (HEAD)     - 現在
G            - 1 小時前
F            - 2 小時前
E            - 3 小時前 (合併)
D            - 4 小時前
C            - 5 小時前
B            - 6 小時前
A            - 7 小時前
```

按 commit 的 **committer timestamp** 降序列出。這是 git log 的預設行為。

時間排序的優點是直觀，但對於合併歷史，時間排序可能會產生違反直覺的結果：

```
    A (10:00) ←── B (10:05) ←── C (10:15)
     │                            │
     └──── D (10:03) ←───────────┘
```

時間排序輸出：C → B → D → A（D 雖然在 B 的拓樸之前，但時間較早）

### git2::Sort::TOPO（拓樸排序）

在拓樸排序中，一個 commit 只有在它的所有子節點都列印之後才會被輸出：

```
    A ←── B ←── C ←── D
                │
                └─── E ←── F
```

拓樸輸出：D → F → C → E → B → A

拓樸排序保證：
- 如果 B 是 A 的祖先，則 A 在 B 之前輸出
- 適合檢視合併歷史的真實結構

### git2::Sort::REVERSE（反轉）

與其他排序模式結合，將輸出反轉：

```rust
// 反向時間排序（最早的 commit 先輸出）
revwalk.set_sorting(git2::Sort::TIME | git2::Sort::REVERSE)?;
```

Gitpage 未使用 REVERSE，因為 commit log 頁面需要最新的 commit 在最上面。

### git2::Sort::NONE（不排序）

按照 Revwalk 內部遍歷的順序輸出（取決於 push 順序和底層 commit 圖的結構）。通常不建議使用。

## Gitpage 中的使用

### 完整的資料流

```
使用者請求 commit 列表
     │
     ├── GET /api/:username/:repo/commits/:branch
     │
     ▼
src/handlers/content.rs:list_commits()
     │
     ├── 1. resolve_repo() — 解析使用者/組織
     ├── 2. 計算 Git repo 路徑
     │
     ▼
src/git/mod.rs:get_commit_log()
     │
     ├── 1. git2::Repository::open_bare() — 開啟 bare repo
     ├── 2. repo.refname_to_id() — 取得分支的 OID
     ├── 3. repo.revwalk() — 建立 Revwalk
     ├── 4. revwalk.push() — 加入起始 OID
     ├── 5. revwalk.set_sorting(Sort::TIME) — 設定排序
     ├── 6. revwalk.enumerate() — 迭代
     │       ├── 取 OID → find_commit → 提取 sha, message, author, time
     │       └── 直到 limit 或遍歷結束
     │
     ▼
JSON 回傳
     │
     ▼
前端 CommitsPage.tsx
     │
     └── 顯示 commit 列表（每行顯示 avatar, sha, message, author, time）
```

### 限制提交數量

```rust
for (i, oid) in revwalk.enumerate() {
    if i >= limit { break; }  // 只取前 limit 個 commit
    // ...
}
```

`limit` 由前端請求參數或後端預設值決定。Gitpage 預設至少回傳 20 個 commits。

### 錯誤處理

```rust
let oid = match repo.refname_to_id(&branch_ref) {
    Ok(oid) => oid,
    Err(_) => return Ok(Vec::new()),  // 分支不存在，回傳空列表
};
```

如果分支不存在（例如剛初始化的空儲存庫），不會出錯，而是回傳空 commit 列表。

## 效能分析

### 時間複雜度

Revwalk 的時間複雜度為：

```
O(k * log k + n)
```

其中：
- k = 遍歷範圍內的 commit 數量
- n = push 進來的 ref 數量
- log k 來自優先權佇列的操作

對於 Gitpage 的典型使用場景（單個分支，幾百個 commit），實際執行時間在微秒到毫秒級別。

### 與 `git log` 命令的比較

| 方面 | libgit2 Revwalk | `git log` 命令 |
|------|----------------|---------------|
| 程序啟動 | 無（in-process） | 需要 fork 新程序 |
| IPC 開銷 | 無 | stdout pipe 讀取 |
| 解析成本 | 直接取得結構化資料 | 解析文字輸出 |
| 分頁控制 | 直接 limit 控制 | 需 `--max-count` |
| 非同步支援 | 可在 async context 使用 | 需 `tokio::process::Command` |

在 Gitpage 中，如果使用 `git log` 命令，每個 commit 請求都需要：
1. 建立子程序（fork + exec）
2. 透過 pipe 讀取輸出
3. 解析文字格式（分隔符、時間格式等）

使用 libgit2 Revwalk，這些都在同一個程序內完成，沒有 IPC 開銷。

### 記憶體開銷

Revwalk 的記憶體使用主要來自：
1. **優先權佇列**：最多儲存 commit 數量級的元素，可忽略
2. **seen 集合**：位元集合，每個 commit 佔 1 bit
3. **Commit 物件**：只在遍歷到時載入和解構

相比於 `git log` 需要將所有輸出保留在記憶體中直到讀取完畢，Revwalk 是串流的—邊遍歷邊輸出。

## 進階：Commit Graph 檔案

### 什麼是 Commit Graph

Git 2.18+ 支援 `.git/objects/info/commit-graph` 檔案，這是一種二進位格式，儲存了 commit DAG 的結構化資訊：

```
commit-graph 檔案結構：
┌─────────────────────────┐
│   Header (signature)     │  "CGPH" + version
├─────────────────────────┤
│   OID Fanout Table       │  256 entries, SHA1 分桶
├─────────────────────────┤
│   OID Lookup Table       │  每個 commit 的 OID
├─────────────────────────┤
│   Commit Data            │  Tree OID, parents, time, generation
├─────────────────────────┤
│   Extra Edge List        │  超過 2 個 parent 的 edge
├─────────────────────────┤
│   Bloom Filter (可選)    │  Changed path filter
└─────────────────────────┘
```

### Commit Graph 對 Revwalk 的加速

1. **生成數（generation number）**：每個 commit 被賦予一個 generation number，代表從 root commit 到該節點的最長路徑長度。Revwalk 可以使用 generation number 進行拓樸排序的優化。
2. **快速的父節點查詢**：commit graph 直接編碼了父子關係，不需要載入和解析 commit 物件。
3. **減少 I/O**：傳統 git 需要從 `.git/objects/` 目錄讀取大量小檔案，commit graph 將資料合併為一個連續檔案。

### Gitpage 的 Commit Graph 支援

Gitpage 的 bare repo 可選擇性地建立 commit-graph：

```bash
# 在 bare repo 中建立 commit-graph
git -C data/repos/alice/project.git commit-graph write --reachable
```

建立後，libgit2 會自動使用它來加速 Revwalk。不過 Gitpage 本身不管理 commit-graph 檔案的生命週期。

## Revwalk 的進階功能

### 多個起始點

```rust
revwalk.push(oid1)?;  // branch main
revwalk.push(oid2)?;  // branch feature
```

這將遍歷 main 和 feature 兩個分支的所有 commit。對於顯示網路圖或比較分支很有用。

### 排除特定範圍

```rust
revwalk.push(oid_main)?;     // 包含所有 main 的祖先
revwalk.hide(oid_base)?;     // 排除 base 及其祖先（顯示從 base 到 main 的差異）
```

這相當於 `git log main ^base`。

### Revwalk 的重置

```rust
revwalk.reset();   // 清空 push/hide 列表
revwalk.push(new_oid)?;  // 重新設定
```

## 參考資料

- [libgit2 Revwalk API](https://libgit2.org/docs/classes/git_revwalk.html) — 官方 API 文檔
- [Git Commit Graph Design](https://git-scm.com/docs/commit-graph) — Git commit-graph 檔案格式定義
- [Git Internals - Git Objects](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects) — commit 物件的底層結構
- `src/git/mod.rs:408-439` — `get_commit_log()` 實作
- `src/handlers/content.rs` — `list_commits()` handler
- `frontend/src/pages/CommitsPage.tsx` — 前端的 commit 列表頁面
