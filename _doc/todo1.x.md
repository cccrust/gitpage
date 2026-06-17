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

- [ ] `gitpage-agent` — 容器內常駐 agent，接收宿主機 deploy 指令
- [ ] Git push 自動觸發容器內建置
- [ ] SSH host key 持久化（volume 保存 `/etc/ssh/ssh_host_*`）
- [ ] 瀏覽器內 terminal（xterm.js + websocket）

---

## v1.2 — 每人獨立容器

### 容器管理

- [ ] `src/docker.rs` — Docker API 封裝（bollard crate）
  - `ensure_user_container(user)` — 若容器不存在則建立（使用 `gitpage-dev-base`）
  - `exec_deploy(user, repo)` — 執行 agent deploy
  - `exec_stop(user, repo)` — 停止 app
  - `get_container_ip(user)` — 查詢容器 IP
- [ ] 使用者註冊時 → `ensure_user_container()`
- [ ] Git push 後 → `exec_deploy()`

### App Proxy 改造

- [ ] `/app/{user}/{repo}/*` → 改成代理到容器 IP，不再走宿主機 subprocess
- [ ] 移除現有 `deploy.rs` subprocess 管理（或保留為 fallback）

### SSH 隔離

- [ ] 每人一台獨立 sshd，宿主機分配 port（如 22001→alice, 22002→bob）
- [ ] port 分配：`[ssh] port_range_start / port_range_end`
- [ ] SSH public key 自動同步到容器內 `~/.ssh/authorized_keys`

### 儲存

- [ ] named volume：`gitpage-{user}-home` → `/home/{user}`（刪容器保留資料）
- [ ] 唯讀掛載 `data/repos` → `/repos`

### 向後相容

- [ ] `mode = "process"` 維持現有 subprocess 行為
- [ ] Docker 不可用時自動退回 process 模式
- [ ] 遷移工具：將現有 subprocess app 轉移到容器

### 設定

```toml
[runtime]
mode = "docker"            # "process" 向後相容

[docker]
memory_limit = "1g"
cpu_shares = 512
base_image = "gitpage-dev-base:latest"
ssh_port_start = 22001
ssh_port_end = 22999
```

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
