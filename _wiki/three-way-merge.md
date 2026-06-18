# 3-Way Merge（三方合併）

## 概述

3-Way Merge 是 Git 合併兩個分支的核心演算法。不同於「兩方合併」（只比較來源分支和目標分支），三方合併引入**共同祖先**（merge base）作為第三個參考點，能夠自動處理大部分的非衝突變更。Gitpage 在 Pull Request 合併功能中使用 libgit2 的 `merge_trees()` 實作三方合併。

## 演算法原理

### 為什麼需要三方？

考慮以下場景：

```
檔案 content.txt:
A: "hello"
B: "world"

雙方差異：
    來源: "hello world"
    目標: "hello"
    
二方合併結果？無法判斷是新增還是刪除！
```

加入共同祖先後：

```
共同祖先: "hello"
來源:     "hello world"    ── 新增 " world"
目標:     "hello"         ── 無變更

合併結果: "hello world"   ── 接受來源的新增
```

### 基本流程

```
          o---A---B---C  (feature branch)
         /       \
    D---E---F---G---H---?  (main branch)
        ↑           ↑
   共同祖先       合併點
```

1. 找到 merge base（共同祖先，節點 F）
2. 計算 base → source（feature）的變更（patch1）
3. 計算 base → target（main）的變更（patch2）
4. 同時應用 patch1 和 patch2
5. 若無衝突，自動產生 merge commit

## 合併策略

Git 支援多種合併策略：

| 策略 | 說明 | 使用場景 |
|------|------|---------|
| **Recursive** | 預設策略，遞迴找共同祖先 | 一般合併 |
| **Resolve** | 類似 Recursive 但只分析兩個 head | 簡單合併 |
| **Octopus** | 合併多個分支 | 一次合併多個 topic branch |
| **Ours** | 直接使用目標分支內容 | 丟棄來源分支變更 |
| **Subtree** | 自動識別子樹合併 | 子樹合併 |

libgit2 的 `merge_trees()` 使用 Recursive 策略。

## libgit2 的 3-Way Merge 實作

### 基本操作

```rust
// src/handlers/pulls.rs — merge_pr()
use git2::{Repository, MergeOptions};

pub fn perform_merge(
    repo: &Repository,
    base_branch: &str,
    head_branch: &str,
) -> Result<(), MergeError> {
    // 1. 解析分支
    let base_commit = repo.find_commit(
        repo.refname_to_id(&format!("refs/heads/{}", base_branch))?
    )?;
    let head_commit = repo.find_commit(
        repo.refname_to_id(&format!("refs/heads/{}", head_branch))?
    )?;

    // 2. 找到 merge base（共同祖先）
    let merge_base_oid = repo.merge_base(
        base_commit.id(),
        head_commit.id(),
    )?;
    let merge_base_commit = repo.find_commit(merge_base_oid)?;
    let merge_base_tree = merge_base_commit.tree()?;

    // 3. 取得兩分支的 tree
    let base_tree = base_commit.tree()?;
    let head_tree = head_commit.tree()?;

    // 4. 執行三方合併
    let merge_result = repo.merge_trees(
        &merge_base_tree,   // 共同祖先
        &base_tree,         // 目標分支 (ours)
        &head_tree,         // 來源分支 (theirs)
        Some(&MergeOptions::new()),  // 合併選項
    )?;

    // 5. 檢查衝突
    if merge_result.conflicts.is_some() && merge_result.conflicts.unwrap().count() > 0 {
        return Err(MergeError::Conflict("存在合併衝突".into()));
    }

    // 6. 建立 merge commit
    let new_tree = repo.find_tree(merge_result.tree_id)?;
    let signature = repo.signature()?;
    repo.commit(
        Some(&format!("refs/heads/{}", base_branch)),
        &signature,
        &signature,
        &format!("Merge branch '{}' into '{}'", head_branch, base_branch),
        &new_tree,
        &[&base_commit, &head_commit],  // 兩個 parent
    )?;

    Ok(())
}
```

### 衝突處理

當兩個分支修改了同一檔案同一區域時，產生衝突：

```rust
// 檢查並報告衝突
if let Some(conflicts) = merge_result.conflicts {
    let mut conflict_iter = conflicts.into_iter();
    while let Some(conflict) = conflict_iter.next() {
        if let Some(ancestor) = conflict.ancestor() {
            // 共同祖先版本
            println!("Conflict in: {}", ancestor.path().unwrap_or("?"));
        }
        if let Some(ours) = conflict.ours() {
            // 目標分支版本
            println!("  Ours: mode={:o} oid={}", ours.mode(), ours.id());
        }
        if let Some(theirs) = conflict.theirs() {
            // 來源分支版本
            println!("  Theirs: mode={:o} oid={}", theirs.mode(), theirs.id());
        }
    }
}
```

在 Gitpage 中，遇到衝突時回傳錯誤，要求使用者手動解決衝突後再合併：

```rust
// PR merge — 檢查衝突
let has_conflicts = merge_result.conflicts
    .map(|c| c.count() > 0)
    .unwrap_or(false);

if has_conflicts {
    return Err(AppError::Conflict(
        "合併存在衝突，請先解決衝突後再試".into()
    ));
}
```

### Diff 計算

PR 的 diff 顯示是透過比較 base tree 和 head tree：

```rust
pub fn get_pr_diff(
    repo: &Repository,
    base_branch: &str,
    head_branch: &str,
) -> Result<Vec<DiffEntry>, Error> {
    let base_commit = repo.find_commit(
        repo.refname_to_id(&format!("refs/heads/{}", base_branch))?
    )?;
    let head_commit = repo.find_commit(
        repo.refname_to_id(&format!("refs/heads/{}", head_branch))?
    )?;

    // 找到 merge base
    let merge_base = repo.merge_base(base_commit.id(), head_commit.id())?;
    let merge_base_tree = repo.find_commit(merge_base)?.tree()?;
    let head_tree = head_commit.tree()?;

    // 計算 diff
    let mut diff = repo.diff_tree_to_tree(
        Some(&merge_base_tree),
        Some(&head_tree),
        None,
    )?;

    // 收集差異條目
    let mut entries = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            entries.push(DiffEntry {
                status: delta.status().to_string(),  // added, modified, deleted, renamed
                old_path: delta.old_file().path().map(|p| p.to_string_lossy().to_string()),
                new_path: delta.new_file().path().map(|p| p.to_string_lossy().to_string()),
                ..Default::default()
            });
            true
        },
        None, None, None,
    )?;

    Ok(entries)
}
```

## 底層原理：merge_trees 的工作方式

`merge_trees()` 在底層執行以下步驟：

### 1. 遍歷三棵樹

同時走訪 base、ours、theirs 三棵樹的同一個路徑：

```
Base:     Ours:     Theirs:
a.txt     a.txt     a.txt
b.txt     b.txt     b.txt (modified)
c.txt     c.txt (modified)  c.txt (modified)
          d.txt (new)
e.txt     e.txt (deleted)   e.txt
```

### 2. 逐檔案判斷合併方式

對每個檔案，根據三個版本的存在情況決定：

| Base | Ours | Theirs | 結果 |
|------|------|--------|------|
| A | A | A | 無變更 |
| A | A' | A | 用 ours |
| A | A | A' | 用 theirs |
| A | A' | A' | 三方內容合併 |
| A | A' | A" | 衝突！（需人工） |
| A | (del) | A | 刪除（ours 刪除） |
| A | A | (del) | 刪除（theirs 刪除） |
| A | (del) | (del) | 刪除（雙方刪除） |
| - | new | - | 新增 |
| - | - | new | 新增 |
| - | new | new | 衝突（同名不同內容） |
| - | new | (del) | 新增（ours） |

### 3. 實際內容合併

當 Base、Ours、Theirs 都不同時，需要逐行合併：

```
Base (祖先):       Ours:            Theirs:
Line 1: hello     Line 1: hello    Line 1: hello
Line 2: world     Line 2: world    Line 2: universe
Line 3: foo       Line 3: bar      Line 3: foo
                  Line 4: baz

逐行比對結果：
Line 1: hello     ← 三方一致，輸出
Line 2: world vs → ← universe（衝突！）
Line 3: bar vs foo（衝突！）
Line 4: baz      ← 僅 ours 新增，輸出
```

## 合併衝突的標記格式

Git 在衝突檔案中插入衝突標記：

```
<<<<<<< ours
bar
=======
foo
>>>>>>> theirs
```

Gitpage 目前遇到衝突時直接拒絕合併，要求使用者解決。未來可實作「在 Web UI 解決衝突」的功能。

## 效能考量

三方合併的時間複雜度：

- 樹比對：O(min(N₁, N₂))，其中 N 為 entry 數量
- 檔案合併：O(L₁ + L₂ + L₃)，其中 L 為行數
- 總體：O(N + ΣL)

百萬行級別的儲存庫合併通常在秒級完成。

## 參考資料

- [Git Merge Strategies](https://git-scm.com/docs/merge-strategies)
- [3-Way Merge Explained](https://stackoverflow.com/questions/4129049/why-is-a-3-way-merge-advantageous-over-a-2-way-merge)
- [libgit2 merge API](https://libgit2.org/docs/group/merge/)
- `src/handlers/pulls.rs` — `merge_pr()`, `get_pr_diff()` 實作
- `src/git/mod.rs` — libgit2 操作基礎
