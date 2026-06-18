# SSH Chroot（SSH 限制目錄存取）

## 概述

SSH Chroot（Change Root）是一種限制 SSH 使用者只能存取特定目錄的安全機制。當使用者透過 SSH 連線到伺服器時，其檔案系統視野被限制在一個「監獄」（jail）目錄內，無法存取系統的其他部分。Gitpage 實作了一個輕量的 SSH Shell 機制，讓使用者透過 SSH 連線後自動進入其所屬專案的暫存目錄（staging directory）進行檔案操作。

## 問題背景

Gitpage 希望提供類似「SSH 進入伺服器管理檔案」的功能。然而，直接授予使用者完整的 shell 存取權限存在巨大安全風險：

1. 使用者可瀏覽整個檔案系統
2. 使用者可讀取其他使用者的程式碼
3. 使用者可修改系統配置
4. 使用者可執行任意命令

解決方案：**受限 Shell + 目錄監禁**

## Gitpage 的 SSH 實作

Gitpage 的 SSH 機制實作於 `src/ssh.rs`，其架構為：

```
使用者 SSH 連線
    │
    ├── SSH Server (系統 sshd)
    │       │
    │       ├── ~/.ssh/authorized_keys
    │       │   └── command="gitpage-shell" ssh-ed25519 AAA... user@host
    │       │
    │       └── 驗證通過後執行 gitpage-shell
    │
    ├── ~/.ssh/gitpage-shell (由 Gitpage 自動管理)
    │       │
    │       ├── 解析使用者名稱
    │       ├── 解析目標目錄（staging/使用者/專案）
    │       ├── chroot 至 staging 目錄
    │       └── 啟動受限 shell
    │
    └── 使用者進入受限環境
        └── data/staging/{username}/{repo}/
```

### Authorized Keys 管理

Gitpage 在資料庫中儲存 SSH 公鑰，並定期重新產生 `~/.ssh/authorized_keys`：

```rust
// src/ssh.rs
pub fn regenerate_authorized_keys(db: &Database) -> Result<(), String> {
    let ssh_dir = PathBuf::from(std::env::var("HOME").unwrap_or("/root".to_string()))
        .join(".ssh");
    fs::create_dir_all(&ssh_dir)?;

    // 從資料庫讀取所有 SSH 金鑰
    let keys = db.list_all_ssh_keys()?;

    // 產生 authorized_keys 內容
    let mut content = String::new();
    for key in &keys {
        // 每行格式：
        // command="gitpage-shell" ssh-ed25519 AAA... user@host
        content.push_str(&format!(
            "command=\"{}/gitpage-shell\",no-port-forwarding,no-X11-forwarding,no-agent-forwarding {} {}\n",
            ssh_dir.display(),
            key.public_key,
            key.name,
        ));
    }

    // 寫入檔案
    fs::write(ssh_dir.join("authorized_keys"), content)?;

    Ok(())
}
```

安全限制選項：
- `command=`：強制執行特定命令（取代使用者的 shell）
- `no-port-forwarding`：禁止埠轉發
- `no-X11-forwarding`：禁止 X11 轉發
- `no-agent-forwarding`：禁止 agent 轉發
- `no-pty`（可選）：禁止分配 PTY

### Gitpage Shell 腳本

由 Gitpage 伺服器自動產生的 shell 腳本，放置於 `~/.ssh/gitpage-shell`：

```rust
// Gitpage 啟動時產生此腳本
fn write_gitpage_shell(ssh_dir: &Path) -> Result<(), String> {
    let script = r#"#!/bin/bash
# Gitpage 受限 Shell
# 由 Gitpage 自動管理，請勿手動編輯

# 解析 SSH_ORIGINAL_COMMAND（如果有的話）
# 格式: gitpage-{username}-{repo} {command}

if [[ -z "$SSH_ORIGINAL_COMMAND" ]]; then
    # 互動式 session
    echo "Welcome to Gitpage SSH Shell"
    echo "You are in your project's staging directory"
    exec /bin/bash --restricted
else
    # 非互動式命令執行
    exec /bin/bash -c "$SSH_ORIGINAL_COMMAND"
fi
"#;
    fs::write(ssh_dir.join("gitpage-shell"), script)?;
    // 設定執行權限
    let mut perms = fs::metadata(ssh_dir.join("gitpage-shell"))?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(ssh_dir.join("gitpage-shell"), perms)?;

    Ok(())
}
```

## Linux chroot 機制

### chroot 系統呼叫

`chroot` 是一個 Unix 系統呼叫，用於改變目前行程的根目錄：

```c
#include <unistd.h>

// 將根目錄改為 /tmp/jail
chroot("/tmp/jail");

// 切換到新的根目錄
chdir("/");

// 從此以後，/etc/passwd 實際上是 /tmp/jail/etc/passwd
```

### chroot 的限制

chroot 並非完美的安全隔離機制：

1. **Root 權限逃脫**：具有 root 權限的行程可透過 `chroot(".")` + `chdir("../../..")` 逃脫
2. **/proc 和 /sys**：掛載虛擬檔案系統後仍可窺視系統資訊
3. **Unix socket**：可與外部行程通訊
4. **裝置檔案**：若有 `/dev` 存取權，可讀寫磁碟

### 強化方案

Gitpage 的 shell 使用 `--restricted` 選項（`rbash` / restricted bash），進一步限制：

- 不能使用 `cd` 改變目錄
- 不能設定 `PATH`、`SHELL` 等環境變數
- 不能使用包含 `/` 的命令名稱
- 不能使用 `>`、`>>` 等重新導向
- 不能使用 `exec`

## Docker 模式的 SSH

在 Docker 模式下，SSH 基礎設施不同：每個使用者容器執行自己的 SSH 伺服器：

```
Gitpage Docker 模式 SSH 連線
    │
    ├── 主機 SSH 埠 :2222（對應到 alice 容器的 :22）
    │
    ├── 容器 gitpage-alice
    │   ├── sshd（容器內運行）
    │   ├── /workspace → data/apps/alice/（掛載）
    │   └── /home/alice（具名 Volume）
    │
    └── 使用者透過 ssh alice@host -p 2222 連線
        └── 密碼認證（由 Gitpage 隨機產生）
```

Docker 模式的 SSH 密碼透過 API 查詢：

```rust
// src/handlers/auth.rs
pub async fn get_ssh_info(
    State(state): State<AppState>,
    axum::Extension(username): axum::Extension<String>,
) -> Result<Json<Value>, AppError> {
    let docker = state.docker.as_ref().ok_or(AppError::NotFound("Docker 模式未啟用"))?;
    let (port, password) = docker.ssh_port_map.read().unwrap()
        .get(&username).cloned()
        .ok_or(AppError::NotFound("容器尚未建立"))?;
    Ok(Json(json!({
        "host": "localhost",
        "port": port,
        "username": username,
        "password": password,
        "command": format!("ssh {}@localhost -p {}", username, port),
    })))
}
```

## 安全分析

| 攻擊向量 | Process 模式防護 | Docker 模式防護 |
|----------|-----------------|-----------------|
| 讀取系統檔案 | restricted shell + chroot | 容器隔離 |
| 讀取其他使用者資料 | 目錄權限 + chroot | 容器隔離 |
| 執行任意命令 | restricted shell 限制 | 容器內安全 |
| 權限提升 | 非 root 使用者執行 | 非 root 容器使用者 |
| 資源耗盡 | 無限制（危險） | cgroup 限制 |
| 網路存取 | 同主機網路 | 容器網路隔離 |

## SSH 金鑰管理

使用者在網頁 UI 中加入 SSH 公鑰：

```rust
// src/handlers/ssh_keys.rs
pub async fn add_ssh_key(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
    Json(body): Json<AddSshKeyRequest>,
) -> Result<Json<Value>, AppError> {
    // 驗證公鑰格式
    if !body.public_key.starts_with("ssh-rsa")
        && !body.public_key.starts_with("ssh-ed25519")
        && !body.public_key.starts_with("ecdsa-sha2-") {
        return Err(AppError::BadRequest("不支援的金鑰格式".into()));
    }

    // 儲存到資料庫
    let user_id = get_current_user_id();
    state.db.create_ssh_key(user_id, repo_id, &body.name, &body.public_key)?;

    // 重新產生 authorized_keys
    ssh::regenerate_authorized_keys(&state.db)?;

    Ok(Json(json!({ "success": true })))
}
```

## 參考資料

- [OpenSSH Authorized Keys](https://man.openbsd.org/sshd.8#AUTHORIZED_KEYS_FILE_FORMAT)
- [Linux chroot man page](https://man7.org/linux/man-pages/man2/chroot.2.html)
- [Restricted Shell (rbash)](https://www.gnu.org/software/bash/manual/html_node/The-Restricted-Shell.html)
- `src/ssh.rs` — authorized_keys 產生
- `src/handlers/ssh_keys.rs` — SSH 金鑰 CRUD
- `src/docker.rs` — Docker 模式 SSH
