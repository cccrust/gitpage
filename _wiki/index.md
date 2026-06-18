# Gitpage Wiki Index

Gitpage 是一個自託管的 Git 平台（類似極簡版 GitHub/GitLab），採用 Rust + React 技術棧。本 wiki 收集與本專案相關的專有名詞與技術概念說明。

## 類別 A — Git 核心技術

| 詞項 | 說明 |
|------|------|
| [Git HTTP Smart Protocol](git-http-smart-protocol.md) | Git 透過 HTTP 進行 push/pull/clone 的智慧型傳輸協定 |
| [libgit2](libgit2.md) | 可程式化操控 Git 儲存庫的 C 語言函式庫 |
| [3-Way Merge](three-way-merge.md) | Git 的三方合併演算法及其在 Pull Request 中的應用 |
| [Staging Area](staging-area.md) | Git 暫存區概念在 Gitpage 檔案管理器中的實作 |

## 類別 B — 認證與安全

| 詞項 | 說明 |
|------|------|
| [JWT Authentication](jwt-auth.md) | JSON Web Token 無狀態認證機制 |
| [Argon2](argon2.md) | 記憶體硬性密碼雜湊演算法 |
| [AES-256-GCM](aes-256-gcm.md) | 認證加密標準在 Secrets 儲存中的應用 |
| [SSH Chroot](ssh-chroot.md) | SSH 連線限制於特定目錄的機制 |

## 類別 C — 部署與執行

| 詞項 | 說明 |
|------|------|
| [Auto-Deploy Pipeline](auto-deploy.md) | Git push 後自動建置與部署的流水線 |
| [Reverse Proxy App](reverse-proxy-app.md) | 反向代理架構在 App 託管中的應用 |
| [Docker Runtime](docker-runtime.md) | Docker 容器作為應用執行環境的模式 |
| [bollard](bollard.md) | Rust 的 Docker Engine API 非同步客戶端 |
| [Process vs Docker](process-vs-docker.md) | 兩種應用執行模式的比較 |

## 類別 D — 架構與基礎設施

| 詞項 | 說明 |
|------|------|
| [SPA Fallback](spa-fallback.md) | 單頁應用在前端路由中的後備機制 |
| [WAL Mode](wal-mode.md) | SQLite 的 Write-Ahead Logging 並發模式 |
| [Owner Resolution](owner-resolution.md) | 使用者與組織雙重擁有權的解析模式 |
| [Axum](axum.md) | Rust 的非同步 Web 框架 |
| [rusqlite](rusqlite.md) | Rust 的 SQLite 繫結 |
| [Partial Index](partial-index.md) | SQLite 條件式索引在擁有者解析中的應用 |
| [Vite Proxy](vite-proxy.md) | 開發代理伺服器在前後端分離中的角色 |

## 類別 E — 設計模式

| 詞項 | 說明 |
|------|------|
| [OnceLock](onceLock-init.md) | Rust 的延遲全域初始化模式 |
| [tokio Mutex](tokio-mutex.md) | 非同步環境中的互斥鎖設計 |
| [AppError Pattern](apperror-pattern.md) | Rust 的統一錯誤處理模式 |

## 類別 F — 前端與渲染

| 詞項 | 說明 |
|------|------|
| [React 19 + TypeScript](react-typescript.md) | 前端架構與極簡狀態管理模式 |
| [pulldown-cmark](pulldown-cmark.md) | Rust Markdown 解析器與安全渲染 |
| [Revwalk](revwalk.md) | Git Commit 拓樸遍歷機制 |
| [Nonce 管理策略](nonce-management.md) | AES-256-GCM Nonce 隨機生成與碰撞分析 |
