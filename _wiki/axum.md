# Axum（Rust 非同步 Web 框架）

## 概述

Axum 是一個基於 Tokio、Tower 和 Hyper 的 Rust 非同步 Web 框架，由 Tokio 團隊開發與維護。Axum 以 **模組化**、**型別安全**、**無巨集** 的設計哲學聞名。Gitpage 使用 Axum 作為 HTTP 伺服器框架，處理所有的 API 路由、中介軟體、靜態檔案服務和反向代理。

## 設計哲學

Axum 的設計與其他 Rust Web 框架的比較：

| 特性 | Axum | Actix-Web | Rocket | Warp |
|------|------|-----------|--------|------|
| 架構 | 基於 Tower | 自有 Actor | 巨集驅動 | Filter 組合 |
| 非同步運行時 | Tokio | Tokio | Tokio | Tokio |
| 提取器 (Extractor) | ✅ 自訂 Trait | ✅ 整合 | ✅ 自動 | ❌ Filter |
| 中介軟體 | Tower Layer/Service | actix-web Middleware | Fairing | Filter |
| 狀態共享 | State 提取器 | Data | State | Filter |
| OpenAPI | aide / utoipa | utoipa | Rocket 內建 | utoipa |
| 學習曲線 | 中 | 中高 | 低 | 高 |
| 流行度 | 快速增長 | 成熟 | 成熟 | 穩定 |

## Gitpage 中的 Axum 應用

### 1. 建立 Router

實作於 `src/app.rs` 的 `create_app()` 函數：

```rust
pub fn create_app(state: AppState) -> Router {
    Router::new()
        // Auth 路由
        .route("/api/auth/register", post(handlers::auth::register))
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/me", get(handlers::auth::me))
        .route("/api/auth/password", put(handlers::auth::change_password))

        // Repo 路由
        .route("/api/repos", get(handlers::repos::list_repos))
        .route("/api/repos", post(handlers::repos::create_repo))
        .route("/api/repos/{id}", get(handlers::repos::get_repo))
        .route("/api/repos/{id}", put(handlers::repos::update_repo))
        .route("/api/repos/{id}", delete(handlers::repos::delete_repo))
        .route("/api/repos/search", get(handlers::repos::search_repos))
        .route("/api/repos/{id}/fork", post(handlers::repos::fork_repo))

        // 內容路由
        .route("/api/{username}/{repo}/tree", get(handlers::content::list_tree))
        .route("/api/{username}/{repo}/blob", get(handlers::content::get_blob))
        .route("/api/{username}/{repo}/readme", get(handlers::content::get_readme))
        .route("/api/{username}/{repo}/commits/{branch}", get(handlers::content::list_commits))

        // ... 更多路由

        // 中介軟體
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))

        // Fallback
        .fallback(fallback_handler)

        // 注入共享狀態
        .with_state(state)
}
```

### 2. 提取器（Extractors）

Axum 的提取器是透過解構函數參數來提取請求中的資訊：

```rust
// 多種提取器的組合
pub async fn create_issue(
    State(state): State<AppState>,           // 共享狀態
    Path(repo_id): Path<i64>,                // URL 路徑參數
    axum::Extension(user_id): axum::Extension<i64>,  // 中間件注入
    Json(body): Json<CreateIssueRequest>,    // JSON body
) -> Result<Json<Value>, AppError> {
    // ...
}
```

每個提取器都實作 `FromRequestParts` 或 `FromRequest` trait：

```rust
// 自訂提取器範例
pub struct AuthenticatedUser {
    pub id: i64,
    pub username: String,
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let user_id = parts.extensions.get::<i64>()
            .ok_or(AppError::Unauthorized("需要登入".into()))?;
        let username = parts.extensions.get::<String>()
            .ok_or(AppError::Unauthorized("需要登入".into()))?;

        Ok(AuthenticatedUser {
            id: *user_id,
            username: username.clone(),
        })
    }
}
```

### 3. 狀態共享（State）

Gitpage 使用 `AppState` 結構作為全應用共享狀態：

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: Arc<Config>,
    pub jwt_expires_hours: u64,
    pub app_manager: AppProcessManager,
    pub docker: Option<DockerManager>,
}

// 建立後透過 Router::with_state() 注入
let state = AppState {
    db,
    config: Arc::new(config),
    jwt_expires_hours: jwt_config.expires_in_hours,
    app_manager,
    docker,
};

let app = create_app(state);
```

### 4. 中介軟體（Middleware）

Gitpage 使用多層中介軟體：

```rust
// 1. 日誌追蹤
.layer(TraceLayer::new_for_http());

// 2. CORS
.layer(CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any));

// 3. 認證（自訂）
.layer(axum::middleware::from_fn_with_state(state.clone(), auth_middleware));
```

認證中介軟體實作：

```rust
async fn auth_middleware<B>(
    State(state): State<AppState>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response<Body>, AppError> {
    // 公開路徑跳過
    if is_public_path(req.uri().path(), req.method()) {
        return Ok(next.run(req).await);
    }

    // 提取 Bearer Token
    let token = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized("需要登入".into()))?;

    // 驗證 JWT
    let claims = verify_token(token)?;

    // 注入使用者資訊
    req.extensions_mut().insert(claims.user_id.parse::<i64>().unwrap());
    req.extensions_mut().insert(claims.username);

    Ok(next.run(req).await)
}
```

### 5. 錯誤處理

Axum handler 可以回傳任何實作 `IntoResponse` 的型別。Gitpage 的 `AppError` 實作了 `IntoResponse`：

```rust
// src/utils/errors.rs
pub enum AppError {
    NotFound(String),
    Unauthorized(String),
    BadRequest(String),
    Internal(String),
    Conflict(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response<Body> {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
        };

        let body = Json(json!({ "error": message }));
        (status, body).into_response()
    }
}
```

這使得 handler 可以簡潔地回傳錯誤：

```rust
return Err(AppError::NotFound("使用者不存在".into()));
return Err(AppError::Unauthorized("密碼錯誤".into()));
return Err(AppError::Conflict("倉庫名稱已存在".into()));
```

### 6. Fallback Handler

Axum 的 `fallback()` 用於處理無匹配路由的請求：

```rust
// 優先級：/git/ > /pages/ > /app/ > 靜態檔案 > SPA index.html
async fn fallback_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response<Body>, AppError> {
    let path = req.uri().path().to_string();

    if let Some(caps) = GIT_PATH_RE.captures(&path) {
        return handle_git_backend(state, caps, req).await;
    }
    if let Some(caps) = PAGES_PATH_RE.captures(&path) {
        return serve_pages(state, caps, req).await;
    }
    if let Some(caps) = APP_PATH_RE.captures(&path) {
        return proxy_app_request(state, caps, req).await;
    }

    serve_static_or_spa(state, req).await
}
```

### 7. Tower 生態系

Axum 建立在 Tower 之上，這意味著：

- 所有中介軟體都相容 Tower 的 `Service` 和 `Layer` trait
- 可混合使用 tower-http 的現成中介軟體
- 自訂中介軟體只需實作 `Service` trait

```rust
// tower-http 提供的中介軟體
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
    limit::RequestBodyLimitLayer,
    timeout::TimeoutLayer,
    validate_request::ValidateRequestHeaderLayer,
};
```

## 參考資料

- [Axum 官方文件](https://docs.rs/axum/latest/axum/)
- [Axum GitHub](https://github.com/tokio-rs/axum)
- [Tower 文件](https://docs.rs/tower/latest/tower/)
- `src/app.rs` — Gitpage 路由、中介軟體、fallback 實作
- `src/utils/errors.rs` — AppError IntoResponse 實作
