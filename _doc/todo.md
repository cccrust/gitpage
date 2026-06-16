# Gitpage 開發藍圖

## 總覽

```
v0.1 ─ v0.2 ─ v0.3 ─ v0.4 ─ v0.5 ─ v0.6 ─ v0.7 ─ v0.8 ─ v0.9 ─ v1.0
                                               │
                                        ┌──────┴──────┐
                                  直接 Process      Docker 容器
                                  (v0.6 過渡)       (v0.7 正式)
```

v0.6 先用直接 process 管理快速實作功能與 API 介面，v0.7 換成 Docker 作為正式執行環境。

---

## v0.6 — App Hosting（直接 Process）
**目標**：用 subprocess 管理跑 Node.js / Rust 應用程式

| 模組 | 實作內容 |
|------|----------|
| 資料庫 | `apps_config` 表（branch, source_dir, build/start command, env_vars, enabled） |
| 專案偵測 | 自動辨識 Node.js (`package.json`) 與 Rust (`Cargo.toml`) |
| 部署引擎 | checkout → build → 分配 port → start process |
| Process Manager | port 管理、lifecycle、狀態監控、crash 偵測 |
| Reverse Proxy | `/app/{user}/{repo}/*` → `http://127.0.0.1:{port}/*` |
| API | `GET/PUT/POST/DELETE /api/apps/:repo_id` |
| 前端 | AppSettingsPage 設定頁面 + 狀態顯示 |
| 自動部署 | git push 成功後背景觸發 `auto_deploy_app()` |

**限制**：無隔離、無資源限制、process crash 不自動重啟

詳見 `_doc/v0.6.md`。

---

## v0.7 — App Hosting（Docker 容器）
**目標**：用 Docker 取代直接 process 管理，提供隔離性與可靠性

### 核心轉換

| v0.6 做法 | v0.7 做法 |
|-----------|-----------|
| 直接 `npm start` | 建立 Docker image + `docker run` |
| 手動管理 port | Docker 自動 port mapping |
| 無隔離 | 容器檔案系統隔離 |
| 無資源限制 | `--memory` / `--cpus` |
| crash 僅記錄 | `--restart=always` 自動重啟 |
| 手動 cleanup | `docker rm` 清理 |

### 新增模組

#### Dockerfile 產生器
- 自動偵測專案類型，產生對應的 Dockerfile
- Node.js 範本：
  ```dockerfile
  FROM node:20-alpine
  WORKDIR /app
  COPY . .
  RUN npm install
  ENV PORT=3000
  EXPOSE 3000
  CMD ["npm", "start"]
  ```
- Rust 範本：
  ```dockerfile
  FROM rust:alpine AS builder
  WORKDIR /app
  COPY . .
  RUN cargo build --release

  FROM alpine
  WORKDIR /app
  COPY --from=builder /app/target/release/myapp .
  ENV PORT=3000
  EXPOSE 3000
  CMD ["./myapp"]
  ```
- 若 repo 內已有 `Dockerfile`，直接使用

#### Docker Image 管理
- Build image: `docker build -t gitpage/{user}/{repo}:{sha} .`
- Tag 管理（支援 rollback 到前一個版本）
- 定期清理舊 image

#### Docker Container 管理
- `docker run -d --name gitpage-{repo_id} --restart=always -p {port}:3000 gitpage/{user}/{repo}:latest`
- `docker stop` / `docker start` / `docker rm`
- 健康檢查：`docker inspect` + HTTP health check

#### Port 管理
- Docker 自動分配 host port，或指定 port range
- 記錄 port mapping 到 DB（`apps_config.port`）

### 改寫對照

| v0.6 模組 | v0.7 變化 |
|-----------|-----------|
| `src/deploy/detector.rs` | 保留，僅用於決定 Dockerfile 範本 |
| `src/deploy/mod.rs` | 改為 `docker build` + `docker run` |
| `src/deploy/manager.rs` | 改為操作 Docker API（或 CLI） |
| Reverse Proxy | 不變，仍透過 port 轉發 |
| API / 前端 | 不變，API response 新增 container 狀態 |
| AppState | `docker_client` 取代 `app_manager` |

### 新相依套件
- `bollard` — Rust Docker Engine API 非同步客戶端（比 CLI 更穩定、不回傳錯誤碼）

### 新增功能
- **資源限制**：`--memory=512m --cpus=1`（可設定在 apps_config）
- **自動重啟**：`--restart=always`，container crash 自動重啟
- **日誌**：`docker logs gitpage-{repo_id}` 可透過 API 串流
- **Rollback**：保留前一個 image tag，一鍵回滾

---

## v0.8 — 增強功能

### 即時日誌
- 前端 logs 頁面即時顯示 `docker logs -f`
- 使用 SSE（Server-Sent Events）串流
- 支援過去日誌查詢（最近 1000 行）

### Custom Domain
- CNAME 綁定自訂網域
- 自動申請 Let's Encrypt 憑證（acme.sh 或 rustls）
- 專用 VirtualHost 或 SNI 區分

### WebSocket 代理
- Reverse proxy 升級支援 WebSocket（Upgrade header 轉發）
- 適用於即時應用（socket.io, live reload 等）

### 多語言擴充
- Python：偵測 `requirements.txt` 或 `pyproject.toml`
- Go：偵測 `go.mod`
- Deno / Bun：偵測 `deno.json` / `bun.lock`

---

## v0.9 — 生產準備

### 監控與 Alert
- App 狀態儀表板（所有 app 一覽）
- Uptime 監控 + 異常通知
- 資源使用量圖表（CPU, 記憶體, 網路）

### 用量配額
- 每個 user 的 app 數量上限
- 磁碟、記憶體、CPU 配額
- 超限警告與自動停止

### HTTPS / TLS
- 全站 TLS（gitpage 本身）
- Custom domain 自動 Let's Encrypt
- HTTP/2 支援

### 資料備份
- DB 自動備份
- Docker image 備份／匯出
- 設定檔版本管理

---

## v1.0 — 穩定版本

- API 進入 semver 穩定承諾
- 完整說明文件與 API 文件（OpenAPI）
- Docker Compose 一鍵部署
- CI/CD 整合教學
- 安全性審計

---

## 技術決策記錄

### 為何 v0.6 先做直接 process 而非 Docker？

1. **快速驗證 API 與前端流程** — 先確定 AppSettings UI、API 路由、proxy 邏輯是否合理
2. **降低初期複雜度** — 不需處理 Docker daemon、image build time、registry 等問題
3. **v0.7 可無痛遷移** — Manager trait 封裝 process 操作，Docker 版本只需換底層實作

### 介面抽象

```rust
trait AppRuntime {
    async fn deploy(&self, config: &AppsConfig) -> Result<DeployResult>;
    async fn stop(&self, repo_id: i64) -> Result<()>;
    async fn restart(&self, config: &AppsConfig) -> Result<DeployResult>;
    async fn status(&self, repo_id: i64) -> Result<AppStatus>;
    async fn logs(&self, repo_id: i64) -> Result<String>;
}
```

v0.6 實作為 `ProcessRuntime`，v0.7 實作為 `DockerRuntime`，透過設定切換。

---

## 未納入規劃

- Serverless Functions（如 AWS Lambda）— 與 container hosting 概念不同，暫不支援
- 資料庫服務（如 Heroku Postgres）— 建議使用者自行搭配外部服務
- Buildpack 支援（如 Heroku）— Dockerfile 已涵蓋相同需求
