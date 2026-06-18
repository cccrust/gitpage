# Vite Proxy（開發代理伺服器）

## 概述

Vite Proxy 是 Gitpage 開發環境中的關鍵基礎設施。在開發模式下，前端（Vite dev server，埠 5173）和後端（Axum server，埠 8080）運行在不同的埠上，需要一個代理層來統一 API 請求、Git 操作和 Pages 資源的存取路徑。Vite 的內建 proxy 中間件提供了這個功能，使得前端開發者在 `localhost:5173` 上就能無縫使用所有後端功能。

## 為什麼需要開發代理

### 前後端分離的開發模式

```
開發環境：

瀏覽器 ──→ localhost:5173 (Vite Dev Server) ── HMR, React 元件
               │
               ├── /api/*     ──proxy──→ localhost:8080 (Axum Backend)
               ├── /git/*     ──proxy──→ localhost:8080 (git http-backend)
               └── /pages/*   ──proxy──→ localhost:8080 (靜態頁面託管)
```

### 問題的本質

1. **不同埠 = 不同 origin**：`localhost:5173` 和 `localhost:8080` 被瀏覽器視為不同的 origin，直接從前端 `fetch()` 到後端會觸發 CORS。
2. **路徑結構統一**：前端程式碼中的 API 請求路徑（如 `/api/repos`）在生產環境中與靜態檔案來自同一個 origin，因此在開發環境中也應該保持同樣的路徑。
3. **Git HTTP 傳輸**：`git push/pull/clone` 操作走 `git http-backend`，需要 `/git/` 路徑的代理。

## Vite 的內建代理中間件

### 設定方式

Vite Proxy 的設定在 `frontend/vite.config.ts` 中：

```typescript
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
    plugins: [react()],
    server: {
        proxy: {
            '/api': 'http://localhost:8080',
            '/git': 'http://localhost:8080',
            '/pages': 'http://localhost:8080',
        },
    },
})
```

這是 Vite 最簡潔的代理設定形式—只指定前綴和目標 URL。

### 代理中層實作

Vite 內部使用 `http-proxy-middleware`（基於 `http-proxy`）實現代理。當 Vite dev server 接收到一個以 `/api` 開頭的請求時：

1. **攔截請求**：檢查請求路徑是否匹配設定的前綴
2. **修改路徑**：預設情況下，前綴會被保留（例如 `/api/repos` → `/api/repos`）
3. **轉發到目標**：將請求轉發到 `http://localhost:8080/api/repos`
4. **轉發回應**：將後端的回應直接傳回瀏覽器

### 更完整的設定選項

雖然 Gitpage 使用最簡潔的寫法，Vite 支援更詳細的設定：

```typescript
proxy: {
    '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,        // 修改 Host header 為目標 origin
        rewrite: (path) => path,  // 自訂路徑重寫
        timeout: 30000,            // 超時設定
        on: {
            proxyReq: (proxyReq, req, res) => {
                // 請求轉發前的 hook
            },
            proxyRes: (proxyRes, req, res) => {
                // 回應轉發前的 hook
            },
        },
    },
},
```

## 路徑式代理

Gitpage 代理三個關鍵路徑，每個路徑對應不同的後端功能：

### `/api` — API 請求

所有前端與後端的資料通訊都透過 `/api/` 路徑：

```typescript
// 前端 API 呼叫範例
export function listRepos() {
    return request<{ repos: Repo[] }>('GET', '/api/repos')
}

export function getRepo(id: number) {
    return request<{ repo: Repo; username: string }>('GET', `/api/repos/${id}`)
}

export function listTree(username: string, repo: string, branch?: string, path?: string) {
    return request<TreeResponse>('GET', `/api/${username}/${repo}/tree?${params}`)
}
```

Axum 後端對應的路由：

```rust
// src/app.rs — API 路由
.route("/api/repos", get(handlers::repos::list_user_repos))
.route("/api/:username/:repo_name/tree", get(handlers::content::list_directory))
// ... 約 70 個 API 路由
```

### `/git` — Git HTTP Smart Protocol

Git push/pull/clone 操作使用 Git HTTP Smart Protocol，透過 `git http-backend` CGI 處理：

```rust
// src/app.rs:387 — Git 代理
async fn git_route_handler(...) {
    // 將請求轉發給 git http-backend
    // 使用 std::process::Command 執行 git http-backend
}
```

代理確保 `/git/` 路徑能夠正確路由到後端的 git handler。

### `/pages` — 靜態頁面託管

Pages 託管的靜態網站：

```rust
// src/app.rs — Pages 路由
"/pages/{user}/{repo}/*"
```

開發代理將 `/pages/` 請求轉發到後端，由 Axum 的 pages handler 處理。

## WebSocket 代理考量

### 目前的狀態

Gitpage 目前**不需要** WebSocket 代理，因為：
1. API 請求全部使用 HTTP REST
2. App 託管的 WebSocket 連線直接連到 app 的埠（如 `:3456`），不經過前端代理
3. 部署日誌使用輪詢（polling）而非 WebSocket

### WebSocket 代理設定（未來參考）

如果未來需要 WebSocket 支援（如即時部署日誌串流），Vite 支援 WebSocket 代理：

```typescript
proxy: {
    '/ws': {
        target: 'ws://localhost:8080',
        ws: true,  // 啟用 WebSocket 代理
    },
},
```

## CORS 的影響

### 代理環境（開發）：無 CORS

透過 Vite Proxy 請求時，瀏覽器看到的來源是 `localhost:5173`，而請求目標也是 `localhost:5173`（只是一個路徑，不是不同 origin）。代理在伺服器端將請求轉發到 `localhost:8080`，但瀏覽器完全不知道這件事。因此：

**代理請求不觸發 CORS 預檢請求（OPTIONS）**，也不需要任何 CORS header。

### 非代理環境（生產）：需要 CORS

在生產環境中，如果前端和後端分開部署在不同 origin，則需要 CORS 設定：

```rust
// src/app.rs:82 — CORS 設定
fn build_cors_layer(cfg: &CorsConfig) -> CorsLayer {
    if cfg.allowed_origins.contains(&"*".to_string()) {
        return CorsLayer::permissive();
    }
    // 指定 origin 的 CORS 設定
}
```

但在 Gitpage 的生產環境中，前端靜態檔案由 Axum 的 fallback handler 直接提供（從 `frontend/dist/` 載入），因此前端和後端來自同一個 origin，CORS 不是問題。

## 與生產環境的差異

### 開發環境（Vite Proxy）

```
瀏覽器 ──→ Vite Dev Server (:5173)
              │
              ├── 靜態資源 → Vite 即時打包（HMR）
              └── /api, /git, /pages → Proxy → Axum (:8080)
```

### 生產環境（Axum Fallback）

```
瀏覽器 ──→ Axum Server (:8080)
              │
              ├── 靜態資源 → frontend/dist/（由 fallback handler 提供）
              ├── /api/* → API handler
              ├── /git/* → git http-backend
              └── /pages/* → Pages handler
```

核心差異在於：
| 方面 | 開發環境 | 生產環境 |
|------|---------|---------|
| 靜態檔案提供者 | Vite Dev Server | Axum fallback handler |
| HMR | ✅ 即時熱更新 | ❌ 不適用 |
| 代理層 | Vite Proxy（http-proxy-middleware） | 無代理（同 origin） |
| 建置步驟 | 不需要 | 需要 `npm run build` |

### Axum 的 fallback handler

在生產環境中，fallback handler 負責：
1. 嘗試從 `frontend/dist/` 提供靜態檔案
2. 如果路徑不匹配任何靜態檔案，回傳 `index.html`（SPA fallback）

```rust
// src/app.rs — fallback handler
async fn fallback(...
    // 1. 嘗試提供靜態檔案（從 frontend/dist/）
    // 2. 否則回傳 index.html 讓前端路由處理
)
```

## 建置流程

開發時不需要建置前端，但生產環境需要：

```bash
# 前端建置
cd frontend && npm run build
# 這會執行：
#   1. tsc -b       — TypeScript 型別檢查
#   2. vite build   — Rollup 打包至 frontend/dist/

# 後端建置
cargo build --release

# 運行生產伺服器（Frontend dist 由 Axum 提供）
./target/release/gitpage config.toml
```

`./run.sh` 將這兩個步驟包裝在一起：

```bash
# run.sh
cd frontend && npm run build
cd .. && cargo build --release && ./target/release/gitpage config.toml
```

## Vite Proxy 的限制與風險

### 快取行為

Vite Proxy 不為代理的請求增加任何快取 header。如果後端需要快取控制，應該在 Axum handler 中設定相關 header。

### 請求體大小

Vite Proxy 預設不限制請求體大小。大檔案上傳（如 Git push）的請求體會完整緩衝後再轉發，可能導致記憶體壓力。對於非常大的請求，可以考慮 `stream: true` 選項。

### 代理超時

如果後端請求處理時間過長（如大型 Git 操作），Vite Proxy 可能會超時。預設超時為 30 秒，可透過 `proxy.timeout` 調整。

## 參考資料

- `frontend/vite.config.ts` — 完整的 proxy 設定（3 行）
- `src/app.rs` — Axum 路由 + fallback handler + CORS 設定
- `_wiki/spa-fallback.md` — 生產環境的 SPA 路由後備機制
- `run.sh` — 生產建置與啟動腳本
- [Vite Proxy 文檔](https://vite.dev/config/server-options.html#server-proxy)
- [http-proxy-middleware](https://github.com/chimurai/http-proxy-middleware) — Vite 內部使用的代理引擎
