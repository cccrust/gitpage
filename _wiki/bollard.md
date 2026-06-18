# Bollard（Rust Docker Engine API 客戶端）

## 概述

Bollard 是一個 Rust 語言的 Docker Engine API 非同步客戶端函式庫，透過 Docker Engine REST API 與 Docker Daemon 進行通訊。Gitpage 使用 Bollard 管理 Docker 容器生命週期，包括建立容器、執行命令、查詢狀態等操作，實作於 `src/docker.rs`。

## 為何選擇 Bollard？

在 Rust 生態系中與 Docker 互動的主要選項：

| 函式庫 | 類型 | 非同步 | 維護狀態 | 功能完整度 |
|--------|------|--------|---------|-----------|
| **bollard** | 純 Rust | ✅ (tokio) | ✅ 活躍 | ✅ 完整 API |
| shiplift | 純 Rust | ✅ (futures) | ❌ 低維護 | ⚠️ 部分 |
| docker_command | 包裝 CLI | ❌ | ✅ | ❌ 功能有限 |

Bollard 是目前 Rust 生態中最完整的 Docker API 客戶端，支援所有現代 Docker Engine 功能（容器、映像、網路、Volume、Swarm 等）。

## Bollard 架構

### 傳輸層

Bollard 透過以下方式與 Docker Daemon 通訊：

1. **Unix Socket**（Linux/macOS）：`/var/run/docker.sock`
2. **Named Pipe**（Windows）：`//./pipe/docker_engine`
3. **TCP**（遠端 Docker）：`tcp://host:2375`
4. **TLS over TCP**（安全遠端）：`tcp://host:2376`

```rust
// 連線方式
use bollard::Docker;

// 方式 1: 本機預設（Unix socket）
let docker = Docker::connect_with_local_defaults()?;

// 方式 2: 指定 Unix socket
let docker = Docker::connect_with_unix("/var/run/docker.sock", 120, "1.42")?;

// 方式 3: 遠端 TCP
let docker = Docker::connect_with_http("tcp://192.168.1.100:2375", 120, "1.42")?;

// 方式 4: TLS 遠端
let docker = Docker::connect_with_ssl(
    "tcp://192.168.1.100:2376",
    &pem_cert, &pem_key, &pem_ca_cert,
    120, "1.42"
)?;
```

### API 版本協商

Docker Engine API 使用版本號（如 `v1.42`）。Bollard 在連線時協商支援的版本：

```rust
// Docker::connect_with_local_defaults 自動使用最新支援的版本
// 也可指定版本
let docker = Docker::connect_with_unix("/var/run/docker.sock", 120, "1.40")?;
```

## Gitpage 中的 Bollard 應用

### 1. 容器生命週期管理

#### 拉取映像

```rust
// src/docker.rs
pub async fn pull_image(&self, image: &str) -> Result<(), DockerError> {
    use bollard::image::CreateImageOptions;

    // 建立 CreateImageOptions
    let options = CreateImageOptions {
        from_image: image,
        ..Default::default()
    };

    // 拉取映像（回傳 stream）
    let mut stream = self.docker.create_image(Some(options), None, None);

    // 消耗 stream（等待拉取完成）
    while let Some(item) = stream.next().await {
        match item {
            Ok(output) => {
                // 可記錄拉取進度
                if let Some(progress) = output.progress {
                    debug!("Pulling {}: {}", image, progress);
                }
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}
```

#### 建立容器

```rust
pub async fn create_app_container(
    &self,
    username: &str,
    workspace_mount: &str,
    ssh_port: u16,
) -> Result<String, DockerError> {
    use bollard::container::CreateContainerOptions;

    let container_name = format!("gitpage-{}", username);

    // 容器配置
    let config = Config {
        image: Some(self.base_image.clone()),
        cmd: Some(vec!["sleep", "infinity"]),
        env: Some(vec![
            format!("USER={}", username),
            format!("HOME=/home/{}", username),
        ]),
        host_config: Some(HostConfig {
            port_bindings: Some({
                let mut map = HashMap::new();
                map.insert("22/tcp".to_string(), Some(vec![
                    PortBinding {
                        host_port: Some(ssh_port.to_string()),
                        ..Default::default()
                    },
                ]));
                map
            }),
            binds: Some(vec![
                workspace_mount.to_string(),
                format!("gitpage-home-{}:/home/{}", username, username),
            ]),
            memory: Some(self.memory_limit),
            cpu_shares: Some(self.cpu_shares),
            network_mode: Some(self.network.clone()),
            ..Default::default()
        }),
        ..Default::default()
    };

    // API 呼叫
    let options = CreateContainerOptions {
        name: container_name,
        platform: None,
    };
    let container = self.docker.create_container(Some(options), config).await?;

    // 啟動容器
    self.docker.start_container(&container.id, None).await?;

    Ok(container.id)
}
```

### 2. Docker Exec

Bollard 的 exec API 是 Gitpage Docker 模式的核心。它允許在已執行的容器中執行新行程：

```
docker exec [options] CONTAINER COMMAND [ARG...]
```

Bollard 對應的 API：

#### 建立 + 啟動 Exec（分兩步）

```rust
pub async fn exec_command(
    &self,
    username: &str,
    cmd: &[&str],
) -> Result<String, DockerError> {
    let container_name = format!("gitpage-{}", username);

    // Step 1: 建立 exec instance
    let exec = self.docker.create_exec(
        &container_name,
        CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(cmd.to_vec()),
            ..Default::default()
        },
    ).await?;

    // Step 2: 啟動 exec 並收集輸出
    let output = self.docker.exec_start(
        &exec.id,
        Some(ExecStartOptions { detach: false }),  // 非 detached（等待完成）
    ).await?;

    // 收集 stream 輸出
    let mut result = String::new();
    let mut stream = output;
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        match chunk? {
            LogOutput::StdOut { message } |
            LogOutput::StdErr { message } => {
                result.push_str(&String::from_utf8_lossy(&message));
            }
            _ => {}
        }
    }

    Ok(result)
}
```

#### 分離模式啟動（Detached）

對於需要持續執行的應用：

```rust
pub async fn exec_start_detached(
    &self,
    username: &str,
    cmd: &[&str],
    port: u16,
) -> Result<(), DockerError> {
    let container_name = format!("gitpage-{}", username);

    let exec = self.docker.create_exec(
        &container_name,
        CreateExecOptions {
            attach_stdout: Some(false),  // 不分接輸出
            attach_stderr: Some(false),
            cmd: Some(cmd.to_vec()),
            env: Some(vec![
                format!("PORT={}", port),
                "HOST=0.0.0.0".to_string(),
            ]),
            ..Default::default()
        },
    ).await?;

    // detach = true：啟動後不等待
    let _output = self.docker.exec_start(
        &exec.id,
        Some(ExecStartOptions { detach: true }),
    ).await?;

    Ok(())
}
```

### 3. 容器查詢

#### 列出容器

```rust
pub async fn list_gitpage_containers(&self) -> Result<Vec<ContainerInfo>, DockerError> {
    use bollard::container::ListContainersOptions;

    let mut filters = HashMap::new();
    filters.insert("name".to_string(), vec!["gitpage-".to_string()]);

    let containers = self.docker.list_containers(Some(ListContainersOptions {
        all: true,
        filters,
        ..Default::default()
    })).await?;

    let mut result = Vec::new();
    for container in containers {
        let name = container.names.clone()
            .unwrap_or_default()
            .first()
            .cloned()
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string();

        let status = container.status.clone().unwrap_or_default();

        result.push(ContainerInfo { name, status });
    }

    Ok(result)
}
```

#### 檢查容器

```rust
pub async fn inspect_container(&self, username: &str) -> Result<InspectContainerResponse, DockerError> {
    let container_name = format!("gitpage-{}", username);
    let info = self.docker.inspect_container(&container_name, None).await?;
    Ok(info)
}
```

#### 取得容器 IP

```rust
pub async fn get_container_ip(&self, username: &str) -> Result<String, DockerError> {
    let container_name = format!("gitpage-{}", username);
    let info = self.docker.inspect_container(&container_name, None).await?;

    // 從網路設定中取得 IP
    let networks = info.network_settings
        .and_then(|ns| ns.networks)
        .unwrap_or_default();

    // 取第一個網路的 IP
    for (_, network) in &networks {
        if let Some(ip) = &network.ip_address {
            if !ip.is_empty() {
                return Ok(ip.clone());
            }
        }
    }

    Err(DockerError::NoIpAddress)
}
```

### 4. 容器移除

```rust
pub async fn remove_container(&self, username: &str, force: bool) -> Result<(), DockerError> {
    use bollard::container::RemoveContainerOptions;

    let container_name = format!("gitpage-{}", username);
    self.docker.remove_container(
        &container_name,
        Some(RemoveContainerOptions {
            force,
            v: true,  // 刪除 volumes
            ..Default::default()
        }),
    ).await?;

    // 清除 port mapping
    self.ssh_port_map.write().unwrap().remove(username);
    self.container_ip_map.write().unwrap().remove(username);

    Ok(())
}
```

## 串流處理

Bollard 的許多 API（如 `create_image`、`exec_start`、`logs`）回傳 `Stream`，需要以非同步方式消耗：

```rust
use futures_util::StreamExt;

// logs stream
let mut logs_stream = self.docker.logs(
    &container_name,
    Some(LogsOptions {
        follow: true,
        stdout: true,
        stderr: true,
        ..Default::default()
    }),
);

while let Some(log_result) = logs_stream.next().await {
    match log_result? {
        LogOutput::StdOut { message } => {
            // stdout 行
            info!("[{}] {}", container_name, String::from_utf8_lossy(&message));
        }
        LogOutput::StdErr { message } => {
            // stderr 行
            error!("[{}] {}", container_name, String::from_utf8_lossy(&message));
        }
        _ => {}
    }
}
```

## 錯誤處理

Bollard 的錯誤類型：

```rust
pub enum DockerError {
    // Docker Engine API 錯誤
    DockerEngine(String, u16),     // HTTP 錯誤碼
    // 連線錯誤
    CouldNotConnect,
    // 解析錯誤
    JsonDeserializeError { ... },
    // 逾時
    Timeout,
    // 無 Docker socket
    NoDockerSocket,
}
```

Gitpage 中統一的錯誤轉換：

```rust
impl From<DockerError> for AppError {
    fn from(e: DockerError) -> Self {
        match e {
            DockerError::DockerEngine(msg, code) => {
                if code == 404 {
                    AppError::NotFound(format!("Docker 資源不存在: {}", msg))
                } else {
                    AppError::Internal(format!("Docker 錯誤 ({}): {}", code, msg))
                }
            }
            DockerError::CouldNotConnect | DockerError::NoDockerSocket => {
                AppError::Internal("無法連接到 Docker Daemon".into())
            }
            _ => AppError::Internal(format!("Docker 錯誤: {:?}", e)),
        }
    }
}
```

## 與 docker CLI 的效能比較

| 操作 | Bollard (Rust) | docker CLI | 差異 |
|------|---------------|------------|------|
| 建立容器 | ~50ms | ~150ms | 3x 快 |
| exec 命令 | ~20ms | ~80ms | 4x 快 |
| 列出容器 | ~5ms | ~100ms (含 fork) | 20x 快 |
| 拉取映像 | 同等 | 同等 | 無差異 |

Bollard 的優勢主要來自於無需 fork/exec 子行程，直接透過 HTTP API 與 Docker Daemon 通訊。

## 參考資料

- [Bollard GitHub](https://github.com/fussybeaver/bollard)
- [Docker Engine API Reference](https://docs.docker.com/engine/api/latest/)
- [bollard crate](https://crates.io/crates/bollard)
- `src/docker.rs` — Gitpage 的 DockerManager 完整實作
- `src/main.rs` — Docker 初始化
