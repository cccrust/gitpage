# SPA Fallback（單頁應用後備）

## 概述

SPA Fallback 是單頁應用（Single Page Application）部署在靜態伺服器時常用的路由策略。當使用者直接存取 `/repo/123/settings` 或 `/org/myteam/members` 等「非根路徑」時，伺服器需要將所有請求導向 `index.html`，由前端 JavaScript 路由解析實際路徑。Gitpage 的 fallback handler 實作此模式，同時處理 API、Git、Pages、App 等多種路由。

## 問題：SPA 的深層連結

傳統多頁應用（MPA）中，每個 URL 對應一個實際的 HTML 檔案：

```
/about.html   → /var/www/about.html
/contact.html → /var/www/contact.html
```

但在 SPA 中，所有路由由 JavaScript 在客戶端處理：

```
/about       ← index.html + React Router 解析
/contact     ← index.html + React Router 解析
/settings    ← index.html + React Router 解析
```

當使用者在根路徑 `/` 進入 SPA 後點擊導航，React Router 透過 HTML5 History API 更新 URL，不產生實際的 HTTP 請求。但以下情況會產生直接請求：

1. 使用者直接在瀏覽器網址列輸入 `/settings`
2. 使用者重新整理 `/repo/123/pages`
3. 從外部網站連結到 `/org/myteam`

上述情況下，伺服器需要回傳 `index.html`，讓前端應用程式讀取 URL 並顯示對應頁面。

## Gitpage 的 Fallback 策略

Gitpage 在 `src/app.rs` 的 `fallback_handler()` 中實現了分層的請求處理：

### 路由優先級

```
1. /git/{user}/{repo}/*     → Git HTTP Smart Protocol
2. /pages/{user}/{repo}/*   → 靜態 Pages 託管
3. /app/{user}/{repo}/*     → App 反向代理
4. /api/*                   → API handler（由 Axum 路由器處理）
5. 靜態檔案（frontend/dist/ → static/）
6. 全部 fallback → index.html（SPA fallback）
```

### 實作

```rust
pub async fn fallback_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response<Body>, AppError> {
    let path = req.uri().path().to_string();

    // 1. Git HTTP Smart Protocol
    if let Some(caps) = GIT_PATH_RE.captures(&path) {
        return handle_git_backend(state, caps, req).await;
    }

    // 2. Pages 靜態服務
    if let Some(caps) = PAGES_PATH_RE.captures(&path) {
        return serve_pages(state, caps, req).await;
    }

    // 3. App 反向代理
    if let Some(caps) = APP_PATH_RE.captures(&path) {
        return proxy_app_request(state, caps, req).await;
    }

    // 4. 靜態檔案服務（frontend/dist/ → static/）
    // 先搜尋 frontend/dist/，再搜尋 static/
    for base in &["frontend/dist", "static"] {
        let file_path = PathBuf::from(base).join(path.trim_start_matches('/'));
        if file_path.exists() && file_path.is_file() {
            return serve_static_file(&file_path).await;
        }
    }

    // 5. SPA Fallback：回傳 index.html
    for base in &["frontend/dist", "static"] {
        let index_path = PathBuf::from(base).join("index.html");
        if index_path.exists() {
            return serve_static_file(&index_path).await;
        }
    }

    // 6. 什麼都沒有 → 404
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found"))
        .unwrap())
}
```

### 靜態檔案服務

```rust
async fn serve_static_file(path: &Path) -> Result<Response<Body>, AppError> {
    // 1. 根據副檔名決定 Content-Type
    let content_type = mime_guess::from_path(path)
        .first_or_octet_stream();

    // 2. 讀取檔案
    let content = tokio::fs::read(path).await?;

    // 3. 建構回應
    let response = Response::builder()
        .header("Content-Type", content_type.to_string())
        .header("Content-Length", content.len())
        .body(Body::from(content))
        .unwrap();

    Ok(response)
}
```

## 為什麼需要兩層目錄？

Gitpage 搜尋 `frontend/dist/` 和 `static/` 兩個目錄：

1. **frontend/dist/**：開發時 `npm run build` 的輸出目錄
2. **static/**：使用者手動放置的靜態檔案，或伺服器產生的檔案

這允許在不重建前端的情況下放入自訂靜態資源。

## SPA Fallback 的陷阱

### 1. 靜態檔案路徑衝突

如果使用者建立了一個前端路由 `/repo/123/settings`，同時也存在實際的靜態檔案 `frontend/dist/repo/123/settings`，則靜態檔案優先。但在正常開發中，SPA 不應產生實際的路徑層級。

### 2. 404 頁面

所有未知路徑都回傳 `index.html`，導致 404 錯誤無法被正確報告。前端 React Router 需自行處理 NotFound 頁面：

```typescript
// frontend/src/App.tsx
<Route path="*" element={<NotFound />} />
```

### 3. 快取問題

`index.html` 不應被長期快取，否則使用者看到的是舊版應用：

```rust
// 建議針對 index.html 設定 no-cache
if path == "/" || path == "/index.html" {
    response.headers_mut().insert(
        "Cache-Control",
        HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
}
```

## Vite 的 SPA Fallback 支援

在開發環境中，Vite 內建了 SPA fallback：

```typescript
// frontend/vite.config.ts
export default defineConfig({
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:8080',
      '/git': 'http://localhost:8080',
      '/pages': 'http://localhost:8080',
    },
  },
});
```

Vite 的開發伺服器自動將所有 404 回應改為 `index.html`。

## 路由層級示意

```
使用者請求: /repo/42/settings
                              ┌─────────────────────┐
                              │  fallback_handler()  │
                              └──────────┬──────────┘
                                         │
          ┌──────────────────────────────┼──────────────────────────────┐
          │                              │                              │
   /git/ 不符                    /pages/ 不符                 /app/ 不符
          │                              │                              │
          └──────────────────────────────┼──────────────────────────────┘
                                         │
                                ┌────────▼────────┐
                                │ 靜態檔案搜尋     │
                                │ frontend/dist/   │
                                │  + static/       │
                                └────────┬────────┘
                                         │
                                    repo/42/settings 不存在
                                         │
                                ┌────────▼────────┐
                                │ SPA Fallback     │
                                │ → index.html     │
                                │ + HTTP 200       │
                                └─────────────────┘
                                         │
                                ┌────────▼────────┐
                                │ React Router     │
                                │ 解析 /repo/42/   │
                                │ settings →       │
                                │ RepoSettingsPage │
                                └─────────────────┘
```

## 參考資料

- [React Router - Browser Router](https://reactrouter.com/en/main/router-components/browser-router)
- [Vite SPA Fallback](https://vitejs.dev/guide/static-deploy.html)
- [HTML5 History API](https://developer.mozilla.org/en-US/docs/Web/API/History_API)
- `src/app.rs` — fallback_handler 與靜態檔案服務
- `frontend/src/App.tsx` — React Router 路由配置
