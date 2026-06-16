use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};
use serde::Deserialize;

use crate::app::AppState;
use crate::git;
use crate::utils::errors::AppError;

#[derive(Debug, Deserialize)]
pub struct UpdatePagesConfig {
    pub branch: Option<String>,
    pub source_dir: Option<String>,
    pub custom_domain: Option<String>,
    pub enabled: Option<bool>,
}

pub async fn get_pages_config(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let cfg = state.db.get_pages_config(repo_id).await?;
    match cfg {
        Some(c) => Ok(Json(json!({ "pages_config": c }))),
        None => Ok(Json(json!({ "pages_config": null }))),
    }
}

pub async fn update_pages_config(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
    Json(req): Json<UpdatePagesConfig>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    if repo.user_id != user_id {
        return Err(AppError::Unauthorized("無權限修改設定".into()));
    }

    let branch = req.branch.unwrap_or_else(|| "main".to_string());
    let source_dir = req.source_dir.unwrap_or_else(|| "/".to_string());
    let custom_domain = req.custom_domain.unwrap_or_default();
    let enabled = req.enabled.unwrap_or(false);

    state.db.upsert_pages_config(repo_id, &branch, &source_dir, &custom_domain, enabled).await?;

    // Auto-deploy if enabled
    if enabled {
        let user = state.db.find_user_by_id(user_id).await?
            .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;
        let repo_path = state.config.repo_path(&user.username, &repo.name);
        let pages_dir = state.config.pages_dir(&user.username, &repo.name);
        if let Err(e) = git::deploy_pages(&repo_path, &pages_dir, &branch, &source_dir) {
            return Ok(Json(json!({
                "success": true,
                "deploy_error": format!("{}", e)
            })));
        }
    }

    Ok(Json(json!({ "success": true })))
}

pub async fn deploy_pages_handler(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    if repo.user_id != user_id {
        return Err(AppError::Unauthorized("无权操作".into()));
    }

    let user = state.db.find_user_by_id(user_id).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;

    let cfg = state.db.get_pages_config(repo_id).await?;
    let (branch, source_dir) = match cfg {
        Some(c) => (c.branch, c.source_dir),
        None => ("main".to_string(), "/".to_string()),
    };

    let repo_path = state.config.repo_path(&user.username, &repo.name);
    let pages_dir = state.config.pages_dir(&user.username, &repo.name);

    git::deploy_pages(&repo_path, &pages_dir, &branch, &source_dir)?;

    Ok(Json(json!({ "success": true, "pages_dir": pages_dir })))
}
