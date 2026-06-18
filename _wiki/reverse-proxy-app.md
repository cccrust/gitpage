# Reverse Proxy for App Hosting（應用託管反向代理）

## 概述

Gitpage 的 App 託管功能允許使用者部署動態 Web 應用（Node.js、Rust 等）至平台，並透過 `http://host:8080/app/{user}/{repo}/*` 路徑存取。此功能的核心是一個**反向代理**（Reverse Proxy）機制：Axum 伺服器接收請求後，將其轉發至後端應用行程，再將回應回傳給用戶端。

## 反向代理概念

反向代理是位於用戶端與後端伺服器之間的中介服務。與正向代理（代表用戶端請求）不同，反向代理代表伺服器接收請求：

```
用戶端 ──► 反向代理 ──► 後端伺服器
              │
              ├── 負載均衡
              ├── TLS 終止
              ├── 快取
              └── 請求路由
```

在 Gitpage 中，Axum 伺服器本身充當反向代理，將 `/app/{user}/{repo}/*` 路徑的請求路由到不同的後端行程。

## Axum 的代理實作

### 請求捕獲

在 `src/app.rs` 的 fallback handler 中，路徑匹配 `/app/{user}/{repo}/*` 後進入代理邏輯：

```rust
if let Some(caps) = APP_PATH_RE.captures(&path) {
    let user = &caps[1];
    let repo = &caps[2];
    let rest = caps.get(3).map(|m| m.as_str()).unwrap_or("/");

    // 解析擁有者和儲存庫
    let (repo_info, owner_name) = resolve_owner_and_repo(&state, user, repo)?;

    // 查詢此 repo 是否有正在運行的應用
    let app_status = state.app_manager.get(repo_info.id);

    if let Some(status) = app_status {
        if status.status == "running" {
            // 代理請求
            return proxy_request(&state, &owner_name, repo, rest, req).await;
        }
    }
}
```

### 代理請求實作

代理函數需要處理 HTTP 請求的各個面向：

```rust
async fn proxy_request(
    state: &AppState,
    owner: &str,
    repo: &str,
    path: &str,
    req: Request<Body>,
) -> Result<Response<Body>, AppError> {
    // 1. 取得目標位址
    let (host, port) = get_target_address(state, owner, repo)?;

    // 2. 建構代理 URL
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
    let proxy_url = format!("http://{}:{}{}{}", host, port, path, query);

    // 3. 建立 HTTP client
    let client = reqwest::Client::new();
    let proxy_req = client
        .request(req.method().clone(), &proxy_url)
        .headers(req.headers().clone())
        .body(req.into_body());

    // 4. 發送請求並回傳回應
    let resp = proxy_req.send().await?;
    let mut response = Response::builder()
        .status(resp.status())
        .body(Body::from_stream(resp.bytes_stream()))
        .unwrap();
    // 複製回應標頭
    for (key, value) in resp.headers() {
        response.headers_mut().insert(key, value.clone());
    }

    Ok(response)
}
```

### 目標位址解析

根據執行模式決定代理目標：

```rust
fn get_target_address(state: &AppState, owner: &str, repo: &str) -> Result<(String, u16), AppError> {
    let app_status = state.app_manager.get_by_repo(owner, repo)
        .ok_or(AppError::NotFound("應用未運行".into()))?;

    match &state.docker {
        Some(docker) => {
            // Docker 模式：代理到容器 IP
            let container_ip = docker.get_container_ip(owner)?;
            Ok((container_ip, app_status.port))
        }
        None => {
            // Process 模式：代理到本機
            Ok(("127.0.0.1".to_string(), app_status.port))
        }
    }
}
```

## 請求轉發的完整性

一個完整的 HTTP 代理需要處理以下面向：

### 1. 請求方法與路徑

所有 HTTP 方法（GET、POST、PUT、DELETE、PATCH、OPTIONS）都需原樣轉發：

```
用戶端: GET /app/alice/myapp/api/users?page=1
代理:   GET http://127.0.0.1:4001/api/users?page=1
```

### 2. 請求標頭

部分標頭需要修改或過濾：

- `Host`：改為後端伺服器的 host
- `X-Forwarded-For`：加入用戶端真實 IP
- `X-Real-IP`：用戶端真實 IP
- `X-Forwarded-Proto`：原始協定（http/https）
- `Connection`：可能需要改為 `close`
- `Transfer-Encoding`：代理處理 chunked 編碼

### 3. 請求 Body

streaming body 需要被轉發：

```rust
// Axum 的 Body 實作 Stream trait，可以直接 pipe
.body(Body::from_stream(req.into_body()))
```

### 4. 回應處理

```rust
// 狀態碼直接傳遞
.status(resp.status())

// 標頭傳遞（Content-Type, Content-Length, Set-Cookie 等）
for (key, value) in resp.headers() {
    response.headers_mut().insert(key, value.clone());
}

// Body 作為 stream 傳遞
.body(Body::from_stream(resp.bytes_stream()))
```

## Process 模式 vs Docker 模式的代理差異

| 面向 | Process 模式 | Docker 模式 |
|------|-------------|------------|
| 目標 IP | `127.0.0.1` | 容器 IP（如 `172.17.0.3`） |
| 網路隔離 | 無（同機器行程） | 容器網路隔離 |
| TLS | 非必須（內部通訊） | 非必須（內部通訊） |
| 健康檢查 | HTTP GET `/health` | `lsof -ti :port` / docker exec |
| 埠管理 | 從 port_range 分配 | 從 port_range 分配 |

## 埠管理與分配

Gitpage 使用 `AppProcessManager` 集中管理埠號的分配與釋放：

```rust
pub struct AppProcessManager {
    port_range: RangeInclusive<u16>,
    processes: Arc<RwLock<HashMap<i64, AppProcess>>>,
}

impl AppProcessManager {
    pub fn allocate_port(&self) -> Result<u16, String> {
        let procs = self.processes.read().unwrap();
        // 從 range 起始掃描，找第一個未被使用的埠
        for port in self.port_range.clone() {
            if !procs.values().any(|p| p.port == port) {
                return Ok(port);
            }
        }
        Err("沒有可用埠".into())
    }

    pub fn register(&self, repo_id: i64, process: AppProcess) {
        let mut procs = self.processes.write().unwrap();
        procs.insert(repo_id, process);
    }

    pub fn unregister(&self, repo_id: i64) {
        let mut procs = self.processes.write().unwrap();
        if let Some(process) = procs.remove(&repo_id) {
            // kill 行程
        }
    }
}
```

## 進階主題

### WebSocket 代理

代理也需要支援 WebSocket 連線（透過 `Connection: Upgrade` 和 `Upgrade: websocket` 標頭）：

```rust
if req.headers().get("Upgrade")
    .map(|v| v.to_str().ok())
    .flatten() == Some("websocket")
{
    // WebSocket 代理邏輯
    return proxy_websocket(req, target_url).await;
}
```

### 請求逾時

防止長時間連線耗盡資源：

```rust
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .pool_max_idle_per_host(10)
    .build()?;
```

### 錯誤處理

當後端應用崩潰或無回應時，代理應回傳適當的錯誤訊息：

```rust
match proxy_req.send().await {
    Ok(resp) => Ok(adapt_response(resp)),
    Err(e) => {
        if e.is_timeout() {
            Err(AppError::Internal("應用無回應".into()))
        } else if e.is_connect() {
            Err(AppError::NotFound("應用未運行".into()))
        } else {
            Err(AppError::Internal("代理錯誤".into()))
        }
    }
}
```

## 與 Nginx 等專用反向代理的比較

| 特性 | Axum 代理 | Nginx | Caddy |
|------|----------|-------|-------|
| 實作複雜度 | 低（整合在應用中） | 中等（需獨立配置） | 中等 |
| 動態路由 | ✅ 可程式化動態 | ❌ 靜態配置 | ❌ 靜態配置 |
| 效能 | 中等（Rust async） | 高（C 語言事件循環） | 高 |
| TLS 終止 | ❌ 需前置代理 | ✅ 原生支援 | ✅ 自動 Let's Encrypt |
| WebSocket | ✅ 支援 | ✅ 支援 | ✅ 支援 |
| 負載均衡 | ❌ 不支援 | ✅ 內建 | 有限 |

Gitpage 選擇在 Axum 層實作反向代理，主要考量是**動態路由需求**：應用程式的埠號是動態分配的，且可能隨部署而變更。使用靜態配置的 Nginx 無法有效管理這種動態性。

## 參考資料

- [MDN - Reverse Proxy](https://developer.mozilla.org/en-US/docs/Web/HTTP/Proxy_servers_and_tunneling)
- [Axum Documentation](https://docs.rs/axum/latest/axum/)
- [reqwest crate](https://docs.rs/reqwest/latest/reqwest/)
- `src/app.rs` — fallback handler 中的代理邏輯
- `src/deploy.rs` — `AppProcessManager` 與埠管理
- `src/docker.rs` — `get_container_ip()` 容器 IP 解析
