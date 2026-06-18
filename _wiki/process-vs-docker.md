# Process vs Docker Runtime（兩種應用執行模式）

## 概述

Gitpage 支援兩種應用執行模式：**Process 模式**（預設）和 **Docker 模式**。兩種模式共用相同的 API 和前端介面，但底層的應用部署、執行、隔離機制完全不同。使用者在 `config.toml` 中設定 `[runtime] mode` 來選擇。

## 比較總表

| 面向 | Process 模式 | Docker 模式 |
|------|-------------|-------------|
| **設定** | `mode = "process"` | `mode = "docker"` |
| **依賴** | 無特殊依賴 | Docker Engine |
| **隔離性** | ❌ 共用主機環境 | ✅ 獨立容器 |
| **重複性** | ❌ 取決於主機軟體版本 | ✅ 映像固定環境 |
| **SSH** | ❌ 不支援 | ✅ 每使用者 SSH |
| **資源控制** | ❌ 無限制 | ✅ cgroup |
| **啟動速度** | ✅ 即時 | ❌ 需先啟容器 |
| **重啟復原** | ❌ 行程遺失 | ✅ 容器持續運行 |
| **安全性** | ❌ 共用使用者 | ✅ 使用者 namespace |
| **單一 vs 多應用** | 每應用獨立行程 | 每容器多應用 exec |
| **Build 環境** | 主機工具鏈 | 容器內工具鏈 |
| **Port 管理** | 相同（port range） | 相同 |
| **Proxy** | `127.0.0.1:port` | `container_ip:port` |
| **檔案系統** | 直接存取 | 綁定掛載 + 具名 Volume |

## Process 模式（預設）

### 運作方式

在 Process 模式下，每個使用者的應用程式作為 Gitpage 伺服器的子行程（child process）執行：

```
Gitpage Server (PID 1)
    │
    ├── App: alice/myapp (PID 1234, port 4001)
    │   ├── stdin/stdout/stderr (piped to deploy logs)
    │   └── 環境變數: PORT=4001, HOST=0.0.0.0
    │
    ├── App: bob/blog (PID 5678, port 4002)
    │   └── ...
    │
    └── App: charles/api (PID 9012, port 4003)
        └── ...
```

### 建置流程

```rust
// src/deploy.rs — Process 模式建置
pub fn run_build_process(
    workspace_dir: &Path,
    build_cmd: &str,
    log_path: &Path,
) -> Result<String, String> {
    let output = Command::new("sh")
        .args(["-c", build_cmd])
        .current_dir(workspace_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("建置錯誤: {}", e))?;

    let log = String::from_utf8_lossy(&output.stdout).to_string();
    fs::write(log_path, &log).ok();

    if !output.status.success() {
        return Err("建置失敗".into());
    }

    Ok(log)
}
```

### 啟動流程

```rust
// src/deploy.rs — Process 模式啟動
pub fn start_app_process(
    start_cmd: &str,
    workspace_dir: &Path,
    port: u16,
    manager: &AppProcessManager,
) -> Result<u32, String> {
    let mut child = Command::new("sh")
        .args(["-c", start_cmd])
        .env("PORT", port.to_string())
        .env("HOST", "0.0.0.0")
        .current_dir(workspace_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("啟動錯誤: {}", e))?;

    let pid = child.id();

    // 註冊到管理器
    manager.register(ProcessInfo {
        repo_id: ...,
        pid,
        port,
        status: "running".to_string(),
        started_at: chrono::Utc::now(),
    });

    // 背景監控行程
    tokio::spawn(async move {
        let status = child.wait().await;
        manager.update_status(repo_id, "stopped");
    });

    Ok(pid)
}
```

### 優缺點

**優點：**
- 零依賴，無需 Docker
- 啟動快，無容器開銷
- 記憶體使用低
- 簡單易懂

**缺點：**
- 行程共用主機環境，版本衝突風險高
- 重啟後所有應用消失（需手動重新部署）
- 無法限制 CPU/記憶體
- 無 SSH 存取
- 應用崩潰可能影響主機穩定性

## Docker 模式

### 運作方式

在 Docker 模式下，每個使用者有一個專屬容器，應用在容器內透過 `docker exec` 執行：

```
Gitpage Server
    │
    ├── Docker Engine
    │   │
    │   ├── Container: gitpage-alice
    │   │   ├── PID 1: sleep infinity (基礎行程)
    │   │   ├── sshd (port 22 → host port 2222)
    │   │   ├── exec: npm start (alice/myapp, port 4001)
    │   │   └── exec: cargo run (alice/another-app, port 4002)
    │   │
    │   └── Container: gitpage-bob
    │       ├── PID 1: sleep infinity
    │       ├── sshd (port 22 → host port 2223)
    │       └── exec: python app.py (bob/blog, port 4003)
    │
    └── Named Volumes
        ├── gitpage-home-alice → /home/alice
        └── gitpage-home-bob   → /home/bob
```

### 容器建立（註冊時）

```rust
// src/docker.rs
pub async fn ensure_user_container(&self, username: &str) -> Result<(), DockerError> {
    let container_name = format!("gitpage-{}", username);

    if self.container_exists(&container_name).await? {
        return Ok(()); // 已存在
    }

    let ssh_port = self.find_free_ssh_port().await?;
    let password = generate_random_password(12);

    let config = Config {
        image: Some(self.base_image.clone()),
        cmd: Some(vec!["sleep", "infinity"]),
        host_config: Some(HostConfig {
            port_bindings: Some({
                let mut map = HashMap::new();
                map.insert("22/tcp".to_string(), Some(vec![
                    PortBinding { host_port: Some(ssh_port.to_string()), ..Default::default() }
                ]));
                map
            }),
            binds: Some(vec![
                format!("{}:/workspace", workspace_dir.display()),
            ]),
            memory: Some(self.memory_limit),
            cpu_shares: Some(self.cpu_shares),
            ..Default::default()
        }),
        ..Default::default()
    };

    // 建立並啟動
    let container = self.docker.create_container(Some(options), config).await?;
    self.docker.start_container(&container.id, None).await?;

    // 記錄 SSH 資訊
    self.ssh_port_map.write().unwrap()
        .insert(username.to_string(), (ssh_port, password));

    Ok(())
}
```

### 建置與啟動（Docker exec）

```rust
// Docker 模式 — 在容器內建置
pub fn run_build_docker(
    docker: &DockerManager,
    username: &str,
    repo_name: &str,
    build_cmd: &str,
) -> Result<String, String> {
    let output = docker.exec_command(username, &[
        "sh", "-c",
        &format!("cd /workspace/{} && {}", repo_name, build_cmd),
    ])?;
    Ok(output)
}

// Docker 模式 — 在容器內啟動應用
pub fn start_app_docker(
    docker: &DockerManager,
    username: &str,
    repo_name: &str,
    start_cmd: &str,
    port: u16,
) -> Result<(), String> {
    docker.exec_start_detached(username, &[
        "sh", "-c",
        &format!("cd /workspace/{} && PORT={} HOST=0.0.0.0 {}", repo_name, port, start_cmd),
    ])?;

    // 健康檢查
    docker.exec_check_status(username, port)?;

    Ok(())
}
```

### 優缺點

**優點：**
- 完整隔離，應用崩潰不影響主機
- 一致環境（映像固定 Node/Rust/Python 版本）
- 支援 SSH 連線
- 資源控制（cgroup memory/cpu）
- 重啟後容器自動恢復，應用可重新部署

**缺點：**
- 需 Docker Engine，增加部署複雜度
- 每個容器數十 MB 記憶體開銷
- 容器建立需數秒
- 基礎映像需定期更新

## 路由代理差異

兩種模式在反向代理時的路由目標不同：

```rust
// src/app.rs
fn get_app_target(state: &AppState, owner: &str, repo: &str) -> Result<(String, u16), AppError> {
    let info = state.app_manager.get_by_owner_repo(owner, repo)?
        .ok_or(AppError::NotFound("應用未執行"))?;

    match &state.docker {
        Some(docker) => {
            // Docker 模式：代理到容器內部 IP
            let container_ip = docker.get_container_ip(owner)?;
            Ok((container_ip, info.port))
        }
        None => {
            // Process 模式：代理到本機
            Ok(("127.0.0.1".to_string(), info.port))
        }
    }
}
```

## 何時選擇哪種模式？

### 選擇 Process 模式
- 個人開發或小型團隊
- 無 Docker 環境（如最小化 VPS）
- 只需要快速部署簡單應用
- 不關心隔離性

### 選擇 Docker 模式
- 多人使用，需使用者隔離
- 需要 SSH 除錯
- 應用使用不同語言/版本
- 需要資源限制防止濫用
- 生產環境部署

## 參考資料

- `src/deploy.rs` — 兩種模式的建置/啟動/停止實作
- `src/docker.rs` — Docker 容器管理
- `src/app.rs` — 反向代理路由
- `src/main.rs` — 啟動時模式選擇與容器復原
- `config.toml` — `[runtime] mode` 設定
