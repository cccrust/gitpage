# Gitpage 開發藍圖

```
v0.1 ─ v0.2 ─ … ─ v0.9 ─ v0.10 ─ v0.11 ─ … ─ v1.0 ─ v1.1 ─ v1.2 …
                                                         │
                                                   Docker 容器
```

v0.x：功能開發與迭代
v1.0：完整穩定版本（所有 v0.x 功能完備）
v1.x：Docker 容器化版本

---

## 已完成（v0.1 – v0.9）

| 版本 | 功能 |
|------|------|
| v0.1 | 使用者認證（註冊/登入/JWT） |
| v0.2 | Repository CRUD + `git init --bare` |
| v0.3 | Git HTTP Smart Protocol（push/pull/clone） |
| v0.4 | 前端 UI（Repo 頁面、檔案瀏覽、README、commits） |
| v0.5 | Pages 靜態網站託管 |
| v0.6 | App Hosting（直接 subprocess） |
| v0.7 | Dropbox 風格檔案管理（staging + batch commit） |
| v0.8 | 部署日誌檢視器 |
| v0.9 | SSH Shell（public key 管理 + handler script） |

詳見各 `_doc/v0.*.md`。

---

## v0.10 — 錯誤訊息與使用者設定

### 10.1 錯誤訊息統一
- [ ] `src/utils/errors.rs` 全部改為中文
- [ ] 前端 error-box 顯示友善訊息

### 10.2 使用者設定
- [ ] 修改密碼 API + 前端頁面
- [ ] 個人資料編輯（bio、avatar）

---

## v0.11 — 搜尋與 UI Polish

### 11.1 搜尋強化
- [ ] repo 搜尋支援分頁
- [ ] 搜尋結果顯示使用者 + repo 名稱
- [ ] 前端搜尋欄即時下拉建議

### 11.2 UI/UX
- [ ] 首頁顯示近期活動（最近 push 的 repos）
- [ ] Loading state 統一（spinner）
- [ ] 空狀態提示
- [ ] 行動裝置 responsive
- [ ] 分頁元件（repos / commits / deploy logs）
- [ ] Clone URL 顯示

---

## v0.12 — 管理功能與安全

### 12.1 Repo 管理
- [ ] Repo 刪除確認對話框
- [ ] Repo 設定頁面（rename、visibility、delete）
- [ ] 公開 repo 的 README 首頁

### 12.2 安全與配置
- [ ] `config.toml` 加入 `[ssh]` 設定段落
- [ ] JWT secret 可透過環境變數覆蓋
- [ ] CORS 可設定 allowed origins
- [ ] 檔案上傳大小限制（configurable）

---

## v0.13 — 開發者體驗

- [ ] README.md 完整說明（安裝、設定、執行）
- [ ] seed.sh 更新（demo 使用者 + repos + SSH keys）
- [ ] API 文件（OpenAPI 或 `_doc/api.md`）

---

## v1.0 — 穩定版

- 所有 v0.x 功能完成並穩定
- 無 Docker，適用於輕量部署

---

## v1.1+ — Docker 容器化

將 App Hosting 從直接 subprocess 改為 Docker 容器。

### 相依套件
- 加入 `bollard` crate（Rust Docker Engine API）

### Dockerfile 產生器
- Node.js / Rust 範本 Dockerfile
- 若 repo 內已有 `Dockerfile` 則直接使用

### Image 管理
- `docker build` → tag → 清理舊 image

### Container 管理
- `docker run` / `stop` / `rm`
- `--restart=always` 自動重啟
- Port mapping、資源限制、log 串流

### 設定
```toml
[runtime]
mode = "docker"  # 或 "process"（向後相容）
```

---

## 未納入規劃

- Custom Domain + Let's Encrypt
- WebSocket proxy
- Serverless Functions
- Buildpack
- 用量配額與計費
