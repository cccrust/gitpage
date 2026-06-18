# Docker Runtime Mode（Docker 執行模式）

## 概述

Gitpage 支援兩種應用執行模式：**Process 模式**（預設）和 **Docker 模式**。在 Docker 模式下，每個註冊的使用者會獲得一個獨立的 Docker 容器（`gitpage-{username}`），使用者的應用程式在該容器內透過 `docker exec` 建置和執行。此模式提供更強的隔離性、一致的執行環境，以及 SSH 連線支援。

## 設計目標

1. **行程隔離**：使用者的應用程式在容器內執行，不影響主機
2. **環境一致性**：開發與執行環境一致，消除「在我機器上可以跑」的問題
3. **SSH 存取**：每個容器暴露 SSH 埠，使用者可 SSH 進入容器查看應用
4. **資源限制**：可對每個使用者的 CPU、記憶體用量進行限制

## 架構概覽

```
主機 (Host)
├── Gitpage 伺服器 (:8080)
│   ├── bollard (Docker Engine API 客戶端)
│   └── AppProcessManager (應用狀態管理)
│
├── Docker Engine
│   ├── Container: gitpage-alice (SSH: :2222)
│   │   ├── sleep infinity (基礎行程)
│   │   ├── /workspace → data/apps/alice/
│   │   ├── /home/alice → volume
│   │   └── exec: npm start (應用程式)
│   │
│   ├── Container: gitpage-bob (SSH: :2223)
│   │   ├── sleep infinity
│   │   ├── /workspace → data/apps/bob/
│   │   └── exec: cargo run
│   │
│   └── Named Volumes:
│       ├── gitpage-home-alice
│       └── gitpage-home-bob
```

## 實作詳解

### DockerManager 結構

實作於 `src/docker.rs`，使用 `bollard` crate 與 Docker Engine API 通訊：

```rust
pub struct DockerManager {
    docker: Docker,                                // bollard 客戶端
    base_image: String,                            // 基礎映像
    network: String,                               // Docker 網路
    memory_limit: i64,                             // 記憶體限制 (bytes)
    cpu_shares: i64,                               // CPU 權重
    ssh_port_range: (u16, u16),                    // SSH 埠範圍
    ssh_port_map: Arc<RwLock<HashMap<String, (u16, String)>>>,  // 使用者 → (SSH埠, 密碼)
    container_ip_map: Arc<RwLock<HashMap<String, String>>>,     // 使用者 → 容器IP
}
```

### 初始化與連線

```rust
pub async fn connect(&self) -> Result<(), DockerError> {
    // 1. 從預設 socket 連接 Docker Engine
    //    Unix: /var/run/docker.sock
    //    Windows: npipe:////./pipe/docker_engine
    let docker = Docker::connect_with_local_defaults()?;

    // 2. 拉取基礎映像（如 node:18-slim）
    self.pull_image(&self.base_image).await?;

    // 3. 重建 SSH port mapping 和 container IP 映射
    //    從現有 gitpage-* 容器讀取
    self.rebuild_port_mappings().await?;

    Ok(())
}
```

### 使用者容器建立

當使用者註冊或首次需要 Docker 時，建立容器：

```rust
pub async fn ensure_user_container(&self, username: &str) -> Result<(), DockerError> {
    let container_name = format!("gitpage-{}", username);

    // 1. 檢查容器是否已存在
    if self.container_exists(&container_name).await? {
        return Ok(()); // 已存在，跳過
    }

    // 2. 分配 SSH 埠（從 ssh_port_range 找可用埠）
    let ssh_host_port = self.find_free_ssh_port().await?;

    // 3. 產生隨機 SSH 密碼
    let ssh_password: String = (0..12)
        .map(|_| ALPHABET.chars().nth(rand::random::<usize>() % ALPHABET.len()).unwrap())
        .collect();

    // 4. 準備掛載點
    let apps_root = config.storage.base_path.join("apps");
    let workspace_bind = format!("{}:/workspace", apps_root.join(username).display());
    let user_home_volume = format!("gitpage-home-{}", username);

    // 5. 建立容器配置
    let config = Config {
        image: Some(self.base_image.clone()),
        cmd: Some(vec!["sleep", "infinity"]),  // 容器保持運行
        host_config: Some(HostConfig {
            port_bindings: Some({
                let mut map = HashMap::new();
                map.insert("22/tcp".to_string(), Some(vec![
                    PortBinding { host_port: Some(ssh_host_port.to_string()), ..Default::default() }
                ]));
                map
            }),
            binds: Some(vec![workspace_bind, format!("{}:/home/{}", user_home_volume, username)]),
            memory: Some(self.memory_limit),
            cpu_shares: Some(self.cpu_shares),
            network_mode: Some(self.network.clone()),
            ..Default::default()
        }),
        env: Some(vec![
            format!("USER={}", username),
            format!("PASSWORD={}", ssh_password),
            format!("HOME=/home/{}", username),
        ]),
        ..Default::default()
    };

    // 6. 啟動容器
    let options = CreateContainerOptions {
        name: container_name,
        platform: None,
    };
    let container = self.docker.create_container(Some(options), config).await?;
    self.docker.start_container(&container.id, None).await?;

    // 7. 記錄 SSH 埠和密碼
    self.ssh_port_map.write().unwrap()
        .insert(username.to_string(), (ssh_host_port, ssh_password));

    // 8. 取得並記錄容器 IP
    let container_ip = self.get_container_ip(username).await?;
    self.container_ip_map.write().unwrap()
        .insert(username.to_string(), container_ip);

    Ok(())
}
```

### 在容器內執行命令

使用 `docker exec` 在使用者容器中執行命令：

```rust
pub async fn exec_command(&self, username: &str, cmd: &[&str]) -> Result<String, DockerError> {
    let container_name = format!("gitpage-{}", username);

    // 1. 建立 exec instance
    let exec = self.docker.create_exec(
        &container_name,
        CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(cmd.to_vec()),
            ..Default::default()
        },
    ).await?;

    // 2. 啟動 exec 並捕獲輸出
    let output = self.docker.exec_start(
        &exec.id,
        None,
    ).await?;

    // 3. 將輸出流收集為字串
    let mut result = String::new();
    let mut stream = output;
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(LogOutput::StdOut { message }) |
            Ok(LogOutput::StdErr { message }) => {
                result.push_str(&String::from_utf8_lossy(&message));
            }
            _ => {}
        }
    }

    Ok(result)
}
```

### 應用部署（Docker 模式）

完整的部署流程使用 `docker exec`：

```rust
// Docker 模式下的 deploy_app
pub fn deploy_app(username: &str, repo_name: &str, config: &AppsConfig) -> Result<(), String> {
    // 1. 容器內建置
    // docker exec gitpage-alice sh -c "cd /workspace/myapp && npm install"
    let build_output = docker.exec_command(username, &[
        "sh", "-c",
        &format!("cd /workspace/{} && {}", repo_name, build_cmd),
    ])?;

    // 2. 容器內啟動應用
    // docker exec -d gitpage-alice sh -c "cd /workspace/myapp && PORT=4001 HOST=0.0.0.0 npm start"
    let start_output = docker.exec_start_detached(username, &[
        "sh", "-c",
        &format!("cd /workspace/{} && PORT={} HOST=0.0.0.0 {}", repo_name, port, start_cmd),
    ])?;

    // 3. 健康檢查
    // docker exec gitpage-alice sh -c "lsof -ti :4001"
    docker.exec_check_status(username, port)?;

    Ok(())
}
```

### SSH 資訊查詢

使用者可透過 API 查詢容器的 SSH 連線資訊：

```rust
pub async fn get_ssh_info(
    State(state): State<AppState>,
    axum::Extension(username): axum::Extension<String>,
) -> Result<Json<Value>, AppError> {
    let docker = state.docker.as_ref().ok_or(AppError::NotFound("Docker 模式未啟用".into()))?;

    let (ssh_port, password) = docker.ssh_port_map.read().unwrap()
        .get(&username)
        .cloned()
        .ok_or(AppError::NotFound("容器尚未建立".into()))?;

    let host = state.config.server.host.clone();

    Ok(Json(json!({
        "host": host,
        "port": ssh_port,
        "username": username,
        "password": password,
        "command": format!("ssh {}@{} -p {}", username, host, ssh_port),
    })))
}
```

## 容器重啟復原

當 Gitpage 伺服器重啟時，需要從現有容器重建 port mapping：

```rust
pub async fn rebuild_port_mappings(&self) -> Result<(), DockerError> {
    let containers = self.docker.list_containers(Some(ListContainersOptions {
        all: true,
        filters: HashMap::from([
            ("name".to_string(), vec!["gitpage-".to_string()]),
        ]),
        ..Default::default()
    })).await?;

    for container in containers {
        let name = container.names.unwrap_or_default()
            .into_iter().next().unwrap_or_default()
            .trim_start_matches('/').to_string();

        if let Some(username) = name.strip_prefix("gitpage-") {
            // 讀取 SSH port binding
            if let Some(ports) = &container.ports {
                for port in ports {
                    if port.private_port == 22 {
                        if let Some(host_port) = port.public_port {
                            self.ssh_port_map.write().unwrap()
                                .insert(username.to_string(), (host_port, "unknown".to_string()));
                        }
                    }
                }
            }
            // 取得容器 IP
            if let Some(ip) = self.get_container_ip(username).await.ok() {
                self.container_ip_map.write().unwrap()
                    .insert(username.to_string(), ip);
            }
        }
    }
    Ok(())
}
```

## Process 模式 vs Docker 模式

| 特性 | Process 模式 | Docker 模式 |
|------|-------------|-------------|
| 隔離性 | ❌ 共用主機環境 | ✅ 獨立容器 |
| 可重複性 | ❌ 取決於主機環境 | ✅ 一致環境 |
| SSH 存取 | ❌ 不支援 | ✅ 每使用者 SSH |
| 資源限制 | ❌ 無限制 | ✅ CPU/記憶體限制 |
| 啟動速度 | ✅ 即時 | ❌ 需等待容器 |
| 記憶體開銷 | ✅ 低 | ❌ 每容器數十 MB |
| 實作複雜度 | ✅ 低 | ❌ 需 Docker Engine |
| 磁碟使用 | ✅ 低 | ❌ 每容器基礎映像 |
| 重啟復原 | ❌ 行程遺失 | ✅ 容器自動重啟 |

## 設定範例

```toml
[runtime]
mode = "docker"              # 啟用 Docker 模式

[docker]
base_image = "node:18-slim"  # 基礎映像（支援 npm install）
network = "bridge"           # Docker 網路模式
memory_limit = 268435456     # 256MB 記憶體限制
cpu_shares = 512             # CPU 權重（預設 1024）
ssh_port_range_start = 2222  # SSH 埠範圍起始
ssh_port_range_end = 2299    # SSH 埠範圍結束
```

## 參考資料

- [Docker Engine API](https://docs.docker.com/engine/api/)
- [bollard crate](https://docs.rs/bollard/latest/bollard/)
- [Docker exec 原理](https://docs.docker.com/engine/reference/commandline/exec/)
- `src/docker.rs` — DockerManager 完整實作
- `src/deploy.rs` — Docker 模式下的應用部署
- `src/handlers/auth.rs` — 註冊時建立容器
- `src/handlers/apps.rs` — SSH 資訊 API
- `src/main.rs` — Docker 初始化與 restore_apps_on_startup
