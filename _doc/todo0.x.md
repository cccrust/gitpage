# Gitpage 開發藍圖

```
v0.1 ─ v0.2 ─ … ─ v0.9 ─ v0.10 ─ v0.11 ─ v0.12 ─ v0.13 ─ v1.0 ─ v1.1 ─ v1.2 …
                                                                     │
                                                              Docker 容器
```

v0.x：功能開發與迭代
v1.0：完整穩定版本（所有 v0.x 功能完備）✅
v1.x：Docker 容器化版本

---

## 已完成（v0.1 – v1.0）

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
| v0.10 | 錯誤訊息中文化 + 使用者設定（密碼、個人資料） |
| v0.11 | 搜尋分頁 + UI Polish（Spinner、Pagination、Clone URL） |
| v0.12 | Repo 管理（重新命名、刪除確認）+ 安全配置（JWT env、CORS、SSH 設定、上傳限制） |
| v0.13 | README、API 文件、seed.sh 更新 |
| v1.0 | 穩定版：所有功能完成，無 Docker，適用輕量部署 |

詳見各 `_doc/v0.*.md` 及 `_doc/api.md`。

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
