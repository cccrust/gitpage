use axum::{extract::{Path, State}, Json};
use serde_json::{json, Value};

use crate::app::AppState;
use crate::utils::errors::AppError;

pub async fn star_repo(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    state.db.star_repo(user_id, repo_id).await?;
    let repo = state.db.find_repo_by_id(repo_id).await?.unwrap();
    Ok(Json(json!({ "starred": true, "stars_count": repo.stars_count })))
}

pub async fn unstar_repo(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    state.db.unstar_repo(user_id, repo_id).await?;
    let repo = state.db.find_repo_by_id(repo_id).await?.unwrap();
    Ok(Json(json!({ "starred": false, "stars_count": repo.stars_count })))
}

pub async fn get_star_status(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let starred = state.db.is_starred(user_id, repo_id).await?;
    Ok(Json(json!({ "starred": starred })))
}

pub async fn list_stargazers(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let users = state.db.list_stargazers(repo_id).await?;
    Ok(Json(json!({ "stargazers": users, "count": users.len() })))
}

pub async fn watch_repo(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    state.db.watch_repo(user_id, repo_id, "participating").await?;
    let repo = state.db.find_repo_by_id(repo_id).await?.unwrap();
    Ok(Json(json!({ "watching": true, "watch_count": repo.watch_count })))
}

pub async fn unwatch_repo(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    state.db.unwatch_repo(user_id, repo_id).await?;
    let repo = state.db.find_repo_by_id(repo_id).await?.unwrap();
    Ok(Json(json!({ "watching": false, "watch_count": repo.watch_count })))
}

pub async fn get_watch_status(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let watch_type = state.db.get_watch_type(user_id, repo_id).await?;
    Ok(Json(json!({ "watching": watch_type.is_some(), "watch_type": watch_type })))
}

pub async fn list_user_stars(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<Value>, AppError> {
    let user = state.db.find_user_by_username(&username).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;
    let repos = state.db.list_user_stars(user.id).await?;
    Ok(Json(json!({ "repos": repos })))
}
