use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::app::AppState;
use crate::ssh::regenerate_authorized_keys;
use crate::utils::errors::AppError;

#[derive(Debug, Deserialize)]
pub struct AddSshKeyRequest {
    pub name: String,
    pub public_key: String,
}

fn validate_public_key(key: &str) -> Result<(), AppError> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest("請輸入 SSH Public Key".into()));
    }
    let first_space = trimmed.find(' ');
    let prefix = match first_space {
        Some(i) => &trimmed[..i],
        None => trimmed,
    };
    if !prefix.starts_with("ssh-rsa")
        && !prefix.starts_with("ssh-ed25519")
        && !prefix.starts_with("ecdsa-sha2-")
    {
        return Err(AppError::BadRequest(
            "SSH Public Key 格式錯誤，開頭須為 ssh-rsa、ssh-ed25519 或 ecdsa-sha2-".into(),
        ));
    }
    Ok(())
}

pub async fn list_keys(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
    axum::Extension(user_id): axum::Extension<i64>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    if repo.user_id != user_id {
        return Err(AppError::Unauthorized("無權限操作".into()));
    }

    let keys = state.db.list_ssh_keys(repo_id).await?;
    Ok(Json(json!({ "ssh_keys": keys })))
}

pub async fn add_key(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
    axum::Extension(user_id): axum::Extension<i64>,
    Json(req): Json<AddSshKeyRequest>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    if repo.user_id != user_id {
        return Err(AppError::Unauthorized("無權限操作".into()));
    }

    validate_public_key(&req.public_key)?;

    let key = state.db.create_ssh_key(user_id, repo_id, &req.name, req.public_key.trim()).await?;

    if let Err(e) = regenerate_authorized_keys(&state.db).await {
        tracing::warn!("Failed to regenerate authorized_keys: {}", e);
    }

    Ok(Json(json!({ "success": true, "ssh_key": key })))
}

pub async fn delete_key(
    State(state): State<AppState>,
    Path((repo_id, key_id)): Path<(i64, i64)>,
    axum::Extension(user_id): axum::Extension<i64>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    if repo.user_id != user_id {
        return Err(AppError::Unauthorized("無權限操作".into()));
    }

    let deleted = state.db.delete_ssh_key(key_id, user_id).await?;
    if !deleted {
        return Err(AppError::NotFound("SSH Key 不存在".into()));
    }

    if let Err(e) = regenerate_authorized_keys(&state.db).await {
        tracing::warn!("Failed to regenerate authorized_keys: {}", e);
    }

    Ok(Json(json!({ "success": true })))
}
