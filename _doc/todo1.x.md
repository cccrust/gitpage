# Gitpage 1.x 開發藍圖

```
v1.0 ─ v1.1 ─ v1.2 ─ v1.3 ─ v1.4 …
穩定版  容器開發   自訂網域   CI/CD
        環境      +HTTPS     整合
```

核心思路：每個使用者一台開發容器，容器內預載工具鏈，使用者 SSH 進入操作，也透過網頁觸發建置。

---

## v1.1 — 共用開發者映像檔

### 1.1.1 基底映像檔 `gitpage/dev`

一個胖映像檔，包含所有開發所需工具：

| 類別 | 內容 |
|------|------|
| 系統 | Ubuntu 24.04 LTS, OpenSSH server, git, curl, build-essential |
| Node.js | Node.js 22 LTS, npm, create-vite, express-generator |
| Rust | Rust toolchain (stable), cargo, cargo-watch |
| 資料庫 | SQLite 3, sqlite3 CLI |
| Gitpage | `gitpage-agent` — 背景服務，接收來自宿主機的 deploy 指令 |

Dockerfile 位置：`docker/dev.Dockerfile`

使用多階段建置，確保映像檔大小合理。

### 1.1.2 gitpage-agent（容器內服務）

容器內常駐的輕量代 agent：
- 監聽 unix socket 或 TCP port，接收宿主機發送的指令
- 指令：`deploy <username> <repo_name>` — clone/pull 裸倉庫 → 安裝相依 → 建置 → 啟動
- 回報：建置狀態、輸出 log、程式監聽 port
- 語言：Rust 二進位檔（與宿主機 gitpage 共用部分程式碼）
- 開機自動啟動：透過 systemd 或 supervisord

### 1.1.3 使用者容器生命週期

```
git clone / push (HTTP)
       │
       ▼
  gitpage 宿主機
  ┌─────────────────────┐
  │ 收到 push           │
  │ → 觸發 auto-deploy  │
  │ → docker exec       │
  │   gitpage-{user}    │
  │   gitpage-agent     │
  │   deploy {user} {r} │
  └──────┬──────────────┘
         │ docker exec
         ▼
  使用者容器 gitpage-{user}
  ┌─────────────────────────┐
  │ /home/{user}/           │
  │   projects/{repo}/      │ ← git clone from bare repo
  │     (code + node_modules)│
  │   .ssh/authorized_keys  │ ← SSH public key
  │                          │
  │ 服務監聽 127.0.0.1:PORT  │
  │ (host 可 proxy 對外)     │
  └─────────────────────────┘
```

### 1.1.4 實作項目

#### 映像檔建置
- [ ] `docker/dev.Dockerfile` — 基底映像檔定義
- [ ] `docker/gitpage-agent` — agent 原始碼（Rust binary）
- [ ] 建置腳本：`docker/build-dev.sh`
- [ ] CI：映像檔自動建置（可選）

#### gitpage-agent
- [ ] TCP listener（127.0.0.1:9730）
- [ ] `deploy` 指令實作：
  - 從 `data/repos/{user}/{repo}.git` clone 到 `~/projects/{repo}/`
  - 偵測專案類型（Node.js / Rust / static）
  - 執行對應建置指令
  - 啟動程式並回報 port
- [ ] `status` 指令：回報現有專案狀態
- [ ] `stop` 指令：停止指定專案
- [ ] stdout/stderr 串流回宿主機

#### 宿主機整合
- [ ] `src/docker.rs` — Docker API 封裝（bollard crate）
  - `ensure_user_container(user)` — 若容器不存在則建立
  - `exec_deploy(user, repo)` — 執行 agent deploy 指令
  - `exec_stop(user, repo)` — 執行 agent stop 指令
  - `get_container_port(user, repo)` — 查詢專案監聽 port
- [ ] 使用者註冊時 → `ensure_user_container()`
- [ ] Git push 後 → `exec_deploy()`
- [ ] `POST /api/apps/:repo_id/deploy` → `exec_deploy()`
- [ ] App proxy `/app/{user}/{repo}/*` → 轉發到容器內 port

#### SSH 整合
- [ ] 每個使用者容器對應一個宿主機 SSH port（例：22001 → 22）
- [ ] port 分配：`[ssh] port_range_start / port_range_end`
- [ ] SSH public key 自動同步到 `~/.ssh/authorized_keys`（容器內）
- [ ] 使用者 `ssh -p 22001 user@host` 即可登入容器
- [ ] 可選：透過宿主機 SSH gateway 自動轉送（`~/.ssh/config`）

#### 儲存與 Volume
- [ ] 掛載 volume：`gitpage-{user}-home` → `/home/{user}`
- [ ] 掛載唯讀裸倉庫：`data/repos` → `/repos`（唯讀）
- [ ] 容器刪除後 home volume 保留

### 1.1.5 設定
```toml
[runtime]
mode = "docker"            # "process" 向後相容

[docker]
memory_limit = "1g"        # 每個容器記憶體上限
cpu_shares = 512
base_image = "gitpage/dev:latest"
ssh_port_start = 22001     # 容器 SSH 映射起始 port
ssh_port_end = 22999

[apps]
port_range_start = 4000    # 容器內 app 使用（非宿主機）
port_range_end = 65535
```

### 1.1.6 向後相容
- [ ] `mode = "process"` 維持現有行為
- [ ] Docker 不可用時自動退回 process 模式
- [ ] 遷移工具：將現有 subprocess app 轉移到容器

---

## v1.2 — 自訂網域與 HTTPS

### 1.2.1 Custom Domain
- [ ] Pages/Apps 設定 `custom_domain`
- [ ] Host header 路由
- [ ] CNAME 驗證

### 1.2.2 Let's Encrypt
- [ ] 自動 TLS certificate 申請/續約
- [ ] HTTP-01 challenge
- [ ] 設定 `[domain]` 段落

---

## v1.3 — CI/CD 與 Webhook

### 1.3.1 Push Webhook
- [ ] `webhook_configs` 資料表
- [ ] `POST /api/repos/:repo_id/webhooks` CRUD
- [ ] HMAC-SHA256 signature

### 1.3.2 Git Event History
- [ ] `git_events` 資料表
- [ ] 前端活動時間軸

---

## v1.4 — 管理後台

### 1.4.1 Admin 角色
- [ ] `users.admin` 欄位
- [ ] 管理員 API + 頁面

### 1.4.2 系統監控
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
