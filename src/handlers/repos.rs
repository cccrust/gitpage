use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::app::AppState;
use crate::db::models::*;
use crate::git;
use crate::utils::errors::AppError;

pub async fn list_user_repos(
    State(state): State<AppState>,
    user_id: Option<axum::Extension<i64>>,
) -> Result<Json<Value>, AppError> {
    let uid = user_id.map(|e| e.0);
    match uid {
        Some(id) => {
            let repos = state.db.list_user_repos(id).await?;
            Ok(Json(json!({ "repos": repos })))
        }
        None => Ok(Json(json!({ "repos": [] }))),
    }
}

pub async fn list_public_repos(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<Value>, AppError> {
    let user = state.db.find_user_by_username(&username).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;
    let repos = state.db.list_public_user_repos(user.id).await?;
    Ok(Json(json!({ "repos": repos, "user": username })))
}

pub async fn create_repo(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Json(req): Json<CreateRepoRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    if req.name.is_empty() {
        return Err(AppError::BadRequest("倉庫名稱不能為空".into()));
    }

    let user = state.db.find_user_by_id(user_id).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;

    let is_private = req.is_private.unwrap_or(false);
    let description = req.description.unwrap_or_default();

    let repo = state.db.create_repo(user_id, &req.name, &description, is_private).await?;

    let repo_path = state.config.repo_path(&user.username, &req.name);
    git::init_bare_repo(&repo_path)?;

    Ok((StatusCode::CREATED, Json(json!({ "repo": repo }))))
}

pub async fn get_repo(
    State(state): State<AppState>,
    Path((username, repo_name)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    let user = state.db.find_user_by_username(&username).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;

    let repo = state.db.find_repo_by_name(user.id, &repo_name).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    Ok(Json(json!({ "repo": repo, "user": username })))
}

pub async fn delete_repo(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    if repo.user_id != user_id {
        return Err(AppError::Unauthorized("無權限刪除此倉庫".into()));
    }

    let user = state.db.find_user_by_id(user_id).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;

    let repo_path = state.config.repo_path(&user.username, &repo.name);
    if std::path::Path::new(&repo_path).exists() {
        std::fs::remove_dir_all(&repo_path)?;
    }

    state.db.delete_repo(repo_id).await?;

    Ok(Json(json!({ "deleted": true })))
}

pub async fn get_repo_by_id(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;
    let user = state.db.find_user_by_id(repo.user_id).await?;
    let username = user.map(|u| u.username).unwrap_or_default();
    Ok(Json(json!({ "repo": repo, "username": username })))
}
