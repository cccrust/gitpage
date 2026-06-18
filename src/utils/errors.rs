use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    Unauthorized(String),
    BadRequest(String),
    Internal(String),
    Conflict(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotFound(msg) => write!(f, "找不到: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "未授權: {}", msg),
            AppError::BadRequest(msg) => write!(f, "請求錯誤: {}", msg),
            AppError::Internal(msg) => write!(f, "伺服器錯誤: {}", msg),
            AppError::Conflict(msg) => write!(f, "衝突: {}", msg),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
        };
        (status, Json(json!({"error": message}))).into_response()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Internal(format!("資料庫錯誤: {}", e))
    }
}

impl From<git2::Error> for AppError {
    fn from(e: git2::Error) -> Self {
        AppError::Internal(format!("Git 錯誤: {}", e))
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Internal(format!("IO 錯誤: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_not_found_status() {
        let err = AppError::NotFound("repo not found".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_unauthorized_status() {
        let err = AppError::Unauthorized("login required".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_bad_request_status() {
        let err = AppError::BadRequest("invalid input".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_internal_status() {
        let err = AppError::Internal("server error".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_conflict_status() {
        let err = AppError::Conflict("already exists".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn test_display_format() {
        let err = AppError::NotFound("頁面不存在".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("找不到"));
        assert!(msg.contains("頁面不存在"));
    }

    #[test]
    fn test_display_unauthorized() {
        let err = AppError::Unauthorized("請先登入".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("未授權"));
    }

    #[test]
    fn test_display_bad_request() {
        let err = AppError::BadRequest("參數錯誤".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("請求錯誤"));
    }

    #[test]
    fn test_display_internal() {
        let err = AppError::Internal("資料庫連線失敗".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("伺服器錯誤"));
    }

    #[test]
    fn test_display_conflict() {
        let err = AppError::Conflict("名稱已被使用".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("衝突"));
    }

    #[test]
    fn test_from_rusqlite_error() {
        let sqlite_err = rusqlite::Error::InvalidParameterName("?99".to_string());
        let app_err: AppError = sqlite_err.into();
        match app_err {
            AppError::Internal(msg) => assert!(msg.contains("資料庫錯誤")),
            _ => panic!("expected Internal variant"),
        }
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_err: AppError = io_err.into();
        match app_err {
            AppError::Internal(msg) => assert!(msg.contains("IO 錯誤")),
            _ => panic!("expected Internal variant"),
        }
    }
}
