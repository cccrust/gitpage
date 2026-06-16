# Gitpage 系統規劃書

輕量級 Git 程式碼託管平台，類似 GitHub/GitLab。

## 技術架構

```
前端 (React + Vite)  ──REST API──>  後端 (Rust + Axum)
                                        │
                                        ├── SQLite (使用者/倉庫元資料)
                                        ├── git2 (Git 操作)
                                        └── 檔案系統 (bare repos / pages)
```

### 後端：Rust
- **Axum** — Web 框架
- **git2** — Git 操作 (init, list refs, read tree/blob, commit log)
- **git http-backend** — Git HTTP Smart Protocol (clone/fetch/push)
- **pulldown-cmark** — Markdown → HTML
- **rusqlite** — SQLite
- **jsonwebtoken + argon2** — 認證
- **tower-http** — 中介軟體 (CORS, 靜態檔案)

### 前端：React + Vite + TypeScript
- React Router — 客戶端路由
- 純 CSS (mobile-first, Threads/X.com 風格)
- highlight.js — 程式碼高亮
- 無大型 UI 框架

## 目錄結構

```
/
├── Cargo.toml
├── config.toml
├── src/                    # Rust 後端
│   ├── main.rs             # 入口
│   ├── app.rs              # Axum 路由 + 狀態 + SPA fallback
│   ├── config.rs           # 設定
│   ├── db/
│   │   ├── mod.rs          # 資料庫操作
│   │   └── models.rs       # 資料結構
│   ├── auth/mod.rs         # JWT 驗證
│   ├── git/mod.rs          # Git 操作 + HTTP Backend
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── auth.rs         # 登入/註冊
│   │   ├── repos.rs        # 倉庫 CRUD
│   │   ├── content.rs      # 檔案/目錄/commits
│   │   └── pages.rs        # Pages 設定
│   └── utils/errors.rs     # 錯誤處理
├── frontend/               # React 前端 (Vite + TypeScript)
│   ├── src/
│   │   ├── main.tsx        # 入口
│   │   ├── App.tsx         # 路由 + 頁面
│   │   ├── index.css       # 全域樣式 (mobile-first)
│   │   ├── api.ts          # API 客戶端
│   │   ├── pages/          # 頁面元件
│   │   │   ├── LoginPage.tsx
│   │   │   ├── RegisterPage.tsx
│   │   │   ├── Dashboard.tsx
│   │   │   ├── NewRepoPage.tsx
│   │   │   ├── RepoPage.tsx
│   │   │   ├── FileViewPage.tsx
│   │   │   └── CommitsPage.tsx
│   │   └── components/
│   │       ├── Layout.tsx
│   │       └── MarkdownView.tsx
│   ├── public/
│   ├── dist/               # Vite build output (served by Rust)
│   └── vite.config.ts
├── migrations/init.sql
├── static/
└── _doc/plan.md
```

## API 路由

### 認證
| 方法 | 路徑 | 說明 |
|------|------|------|
| POST | /api/auth/register | 註冊 |
| POST | /api/auth/login | 登入，回傳 JWT |
| GET | /api/auth/me | 取得當前使用者 |

### 倉庫
| 方法 | 路徑 | 說明 |
|------|------|------|
| GET | /api/repos | 列出我的倉庫 |
| POST | /api/repos | 建立倉庫 |
| GET | /api/repos/:id | 取得倉庫資訊 |
| DELETE | /api/repos/:id | 刪除倉庫 |
| GET | /api/users/:username/repos | 列出使用者公開倉庫 |

### 內容瀏覽
| 方法 | 路徑 | 說明 |
|------|------|------|
| GET | /api/:username/:repo/content/:branch/*path | 取得檔案內容 |
| GET | /api/:username/:repo/tree/:branch/*path | 列出目錄 |
| GET | /api/:username/:repo/readme/:branch | 取得 README |
| GET | /api/:username/:repo/commits/:branch | 列出提交紀錄 |

### Pages
| 方法 | 路徑 | 說明 |
|------|------|------|
| GET | /api/pages/:repo_id | 取得 Pages 設定 |
| PUT | /api/pages/:repo_id | 更新 Pages 設定 |

### Git Smart Protocol
| 方法 | 路徑 | 說明 |
|------|------|------|
| GET/POST | /:username/:repo.git/* | Git HTTP 智慧協定 |

## 測試策略

1. **CLI 測試** — `cargo build` / `cargo test` / `cargo run`
2. **REST API 測試** — curl 測試所有端點 (test.sh)
3. **前端測試** — 待前端完成後整合測試

## 實作進度

- **v0.1** — Rust 後端 API + Git Server (CLI + REST 測試) ✅
- **v0.2** — React 前端 + Markdown 渲染 + 完整 UI ✅
- **v0.3** — Pages 功能 ✅
- **v0.4** — 強化功能 (private repo, user profile, repo settings, search) ✅
