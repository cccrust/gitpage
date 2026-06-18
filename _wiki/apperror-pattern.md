# AppError Pattern（統一錯誤處理模式）

## 概述

AppError Pattern 是 Rust Web 應用中常見的設計模式：定義一個統一的錯誤列舉型別，涵蓋應用中所有可能發生的錯誤情況，並實作 `IntoResponse`（Axum）或其他 HTTP 框架對應的 trait，將錯誤統一轉換為 HTTP 回應。Gitpage 的 `AppError` 定義於 `src/utils/errors.rs`，作為所有 handler 和服務層函數的統一回傳錯誤型別。

## 為什麼需要統一錯誤處理？

在沒有統一錯誤處理的專案中，常見的問題：

```rust
// ❌ 糟糕的設計：每個函數回傳不同的錯誤型別
async fn get_user(id: i64) -> Result<User, rusqlite::Error> { ... }
async fn get_repo(id: i64) -> Result<Repo, git2::Error> { ... }
async fn auth_check(token: &str) -> Result<i64, jsonwebtoken::Error> { ... }

// handler 需要逐個轉換
async fn handler() -> Result<Json<Value>, AppError> {
    let user = get_user(id).map_err(|e| AppError::Internal(e.to_string()))?;
    let repo = get_repo(id).map_err(|e| AppError::Internal(e.to_string()))?;
    // ... 大量重複的 map_err
}
```

統一錯誤處理的目標：
1. **單一回傳型別**：所有函數使用 `Result<T, AppError>`
2. **自動轉換**：透過 `From` trait 將底層錯誤自動轉為 `AppError`
3. **型別分級**：區分 NotFound、Unauthorized、BadRequest、Conflict 等不同類別
4. **統一格式**：所有錯誤回應使用相同的 JSON 結構

## Gitpage 的 AppError

### 定義

```rust
// src/utils/errors.rs
use axum::response::{IntoResponse, Response};
use axum::Json;
use http::StatusCode;
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),         // 404
    Unauthorized(String),     // 401
    BadRequest(String),       // 400
    Internal(String),         // 500
    Conflict(String),         // 409
}
```

### IntoResponse 實作

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
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

## From 轉換實作

關鍵的設計：為每個可能出現的底層錯誤型別實作 `From`，讓 `?` 運算子自動轉換：

```rust
// rusqlite 錯誤
impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound("資料不存在".into())
            }
            rusqlite::Error::SqliteFailure(err, _) => {
                if err.code == rusqlite::ErrorCode::ConstraintViolation {
                    AppError::Conflict("資料已存在".into())
                } else {
                    AppError::Internal(format!("資料庫錯誤: {}", e))
                }
            }
            _ => AppError::Internal(format!("資料庫錯誤: {}", e)),
        }
    }
}

// git2 錯誤
impl From<git2::Error> for AppError {
    fn from(e: git2::Error) -> Self {
        match e.code() {
            git2::ErrorCode::NotFound => {
                AppError::NotFound(format!("Git 物件不存在: {}", e))
            }
            _ => AppError::Internal(format!("Git 錯誤: {}", e)),
        }
    }
}

// IO 錯誤
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => {
                AppError::NotFound(format!("檔案不存在: {}", e))
            }
            std::io::ErrorKind::PermissionDenied => {
                AppError::Unauthorized("沒有權限".into())
            }
            _ => AppError::Internal(format!("系統錯誤: {}", e)),
        }
    }
}

// JSON 解析錯誤
impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::BadRequest(format!("JSON 格式錯誤: {}", e))
    }
}

// JWT 錯誤
impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        match e.kind() {
            jsonwebtoken::errors::ErrorKind::InvalidToken
            | jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                AppError::Unauthorized("Token 無效或已過期".into())
            }
            _ => AppError::Internal(format!("認證錯誤: {}", e)),
        }
    }
}

// Argon2 錯誤
impl From<argon2::password_hash::Error> for AppError {
    fn from(e: argon2::password_hash::Error) -> Self {
        AppError::Internal(format!("密碼雜湊錯誤: {}", e))
    }
}
```

## 使用範例

### Handler 中的使用

得益於 `From` 實作和 `?` 運算子，handler 變得非常簡潔：

```rust
pub async fn get_repo_handler(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    // 自動轉換 rusqlite::Error → AppError
    let repo = state.db.get_repo(repo_id)?;

    // 權限檢查
    if repo.is_private {
        if repo.owner_id != user_id {
            return Err(AppError::Unauthorized("沒有存取權限".into()));
        }
    }

    // Git 操作自動轉換 git2::Error → AppError
    let git_path = state.config.repo_path(&repo.owner_name, &repo.name);
    let git_repo = git2::Repository::open_bare(&git_path)?;
    let head = git_repo.head()?.peel_to_commit()?;

    Ok(Json(json!({
        "repo": repo,
        "head_commit": head.id().to_string(),
    })))
}
```

### 服務層的使用

```rust
// 服務層也回傳 AppError
impl Database {
    pub fn get_user_by_username(&self, username: &str) -> Result<User, AppError> {
        let conn = self.conn.lock().unwrap();
        let user = conn.query_row(
            "SELECT * FROM users WHERE username = ?1",
            params![username],
            |row| { /* 反序列化 */ },
        )?;  // rusqlite::Error → AppError::NotFound
        Ok(user)
    }

    pub fn create_repo(&self, ...) -> Result<Repository, AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO repositories ...",
            params![...],
        )?;  // 如果違反 UNIQUE 約束 → AppError::Conflict
        Ok(repo)
    }
}
```

## 錯誤回應格式

所有錯誤回應的 JSON 格式統一：

```json
// 404
HTTP/1.1 404 Not Found
Content-Type: application/json
{"error": "儲存庫不存在"}

// 401
HTTP/1.1 401 Unauthorized
Content-Type: application/json
{"error": "需要登入"}

// 400
HTTP/1.1 400 Bad Request
Content-Type: application/json
{"error": "路徑不合法"}

// 409
HTTP/1.1 409 Conflict
Content-Type: application/json
{"error": "倉庫名稱已存在"}

// 500
HTTP/1.1 500 Internal Server Error
Content-Type: application/json
{"error": "資料庫錯誤: ..."}
```

## 錯誤類型選擇準則

| 錯誤類型 | HTTP 狀態碼 | 使用時機 |
|----------|------------|---------|
| `NotFound` | 404 | 資料庫查無結果、Git 物件不存在 |
| `Unauthorized` | 401 | Token 無效、密碼錯誤、非公開倉庫 |
| `BadRequest` | 400 | 輸入驗證失敗、路徑穿越偵測 |
| `Conflict` | 409 | 唯一性約束違反、合併衝突 |
| `Internal` | 500 | 檔案 I/O 錯誤、意外錯誤 |

## 與 Axum 的整合

Axum 的 handler 可以直接回傳 `Result<impl IntoResponse, AppError>`，框架會自動呼叫 `into_response()`：

```rust
// Axum 的 IntoResponse 實作允許：
pub type HandlerResult = Result<Json<Value>, AppError>;

// Axum 自動將 AppError 轉換為 HTTP 回應
```

## 參考資料

- [Rust Error Handling - Rust Book](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Axum Error Handling](https://docs.rs/axum/latest/axum/response/index.html#returning-errors)
- [thiserror crate](https://crates.io/crates/thiserror)（可簡化 AppError 實作）
- `src/utils/errors.rs` — Gitpage 的 AppError 定義與 IntoResponse 實作
- `src/db/mod.rs` — 資料庫操作中的錯誤轉換
