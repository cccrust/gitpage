# Gitpage 1.x 開發藍圖

```
v1.0 ─── v1.1 ─── v1.2 ─── v1.3 ─── v1.4 …
穩定版    Docker       自訂網域    CI/CD
         開發環境      +HTTPS      整合
```

核心思路：每個使用者一台開發容器，容器內預載工具鏈，使用者 SSH 進入操作，也透過網頁觸發建置。

---

## v1.1 — Docker 開發容器 ✓

### 已實作

- [x] `Dockerfile.base` — 開發工具基底（uv+Python, rustup+Rust, Node.js, opencode）
- [x] `Dockerfile` — 多階段建置（frontend Node → backend Rust → runtime）
- [x] `entrypoint.sh` — SSH host keys、使用者建立、repo sync、sshd + gitpage 啟動
- [x] `run_docker.sh` — 自動判斷 base/app image 是否需要 rebuild
- [x] `test_docker.sh` — 容器內完整整合測試
- [x] `.dockerignore` — 排除不必要的檔案
- [x] 共用容器模式，多使用者（alice, bob）SSH 登入
- [x] SSH 登入時自動 clone bare repos 到 `~/`
- [x] KaTeX/Mermaid 渲染修復
- [x] Git empty repo 500 錯誤修復（`repo_exists` 檢查 `objects/` + `refs/`）

### 未實作（待 v1.2 或之後）

- [x] `gitpage-agent` — 容器內常駐 agent → 改用 Docker exec 直接執行指令（`exec_build`, `exec_start_detached`）
- [x] Git push 自動觸發容器內建置 → `deploy.rs` 整合 exec 呼叫
- [ ] SSH host key 持久化（volume 保存 `/etc/ssh/ssh_host_*`）

---

## v1.2 — 每人獨立容器 ✓

### 容器管理

- [x] `src/docker.rs` — Docker API 封裝（bollard crate 0.21）
  - `ensure_user_container(user)` — 若容器不存在則建立（使用 `gitpage-dev-base`），含 named volume、apps bind mount、SSH port 暴露
  - `exec_build(user, repo, cmd)` — 容器內執行建置
  - `exec_start_detached(user, repo, cmd, port, env)` — 容器內背景啟動 app
  - `exec_check_status(user, repo, port)` — 查詢 app 狀態（lsof）
  - `exec_stop_app(user, port)` — 停止 app
  - `get_container_ip(user)` — 查詢容器 IP
  - `remove_container(user)` / `list_user_containers()`
- [x] 使用者註冊時 → `ensure_user_container()`（`handlers/auth.rs`）
- [x] Git push 後 → `exec_build()` / `exec_start_detached()`（`deploy.rs`）

### App Proxy 改造

- [x] `/app/{user}/{repo}/*` → 改成代理到容器 IP，不再走宿主機 subprocess
- [x] `deploy.rs` 保留為 `mode = "process"` 的 fallback

### SSH 隔離

- [ ] 每人一台獨立 sshd，宿主機分配 port（如 22001→alice, 22002→bob）→ 已完成：port 分配，但容器內尚無 sshd
- [x] SSH public key 自動同步到容器內 `~/.ssh/authorized_keys`

### 儲存

- [x] named volume：`gitpage-home-{user}` → `/home/{user}`（刪容器保留資料）
- [x] bind mount `data/apps/{user}` → `/workspace`（deploy 工作區）

### 向後相容

- [x] `mode = "process"` 維持現有 subprocess 行為
- [x] `mode = "docker"` 使用容器 exec 模式
- [x] Docker 不可用時自動退回 process 模式
- [x] `test.sh`（非 Docker）不受影響

### 設定

```toml
[runtime]
mode = "docker"            # "process" 向後相容

[docker]
memory_limit = "1g"
cpu_shares = 512
base_image = "gitpage-dev-base:latest"
network = "bridge"
```

### 基礎建設整頓

- [x] `config.toml`: `storage.base_path` 改為 `"data"`（原為 `"data/repos"`），路徑方法統一使用 base_path
- [x] `src/config.rs`: `repo_path`/`staging_path`/`app_workspace_dir` 路徑一致化
- [x] `src/main.rs`: `--config` CLI 引數、目錄建立邏輯更新
- [x] `test_docker_mode.sh`: Docker runtime mode 整合測試

---

## v1.3 — 自訂網域與 HTTPS

### Custom Domain

- [ ] Pages/Apps 設定 `custom_domain`
- [ ] Host header 路由
- [ ] CNAME 驗證

### Let's Encrypt

- [ ] 自動 TLS certificate 申請/續約
- [ ] HTTP-01 challenge
- [ ] 設定 `[domain]` 段落

---

## v1.4 — CI/CD 與 Webhook

### Push Webhook

- [ ] `webhook_configs` 資料表
- [ ] `POST /api/repos/:repo_id/webhooks` CRUD
- [ ] HMAC-SHA256 signature

### Git Event History

- [ ] `git_events` 資料表
- [ ] 前端活動時間軸

---

## v1.5 — 管理後台

### Admin 角色

- [ ] `users.admin` 欄位
- [ ] 管理員 API + 頁面

### 系統監控

- [ ] 儀表板：使用者、容器、磁碟用量
- [ ] Docker daemon 健康檢查

---

## 未納入規劃

| 功能 | 說明 |
|------|------|
| OAuth / SSO | GitHub/GitLab/Google 登入 |
| Merge Request | 類似 GitHub PR |
| Issue Tracker | 內建 issue 系統 |
| Serverless Functions | 輕量函數託管 |
| WebSocket Proxy | App proxy 支援 WebSocket |
| Multi-node | 多台 server 叢集 |
