use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};
use serde::Deserialize;

use crate::app::AppState;
use crate::utils::errors::AppError;

#[derive(Debug, Deserialize)]
pub struct UpdateAppsConfig {
    pub branch: Option<String>,
    pub source_dir: Option<String>,
    pub build_command: Option<String>,
    pub start_command: Option<String>,
    pub env_vars: Option<String>,
    pub enabled: Option<bool>,
}

pub async fn get_apps_config(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let cfg = state.db.get_apps_config(repo_id).await?;
    let status = state.app_manager.get(repo_id).await;
    let port = status.as_ref().map(|p| p.port);
    let app_status = status.as_ref().map(|p| p.status.clone());
    let url = if let (Ok(Some(repo)), Some(_port)) = (
        state.db.find_repo_by_id(repo_id).await,
        port,
    ) {
        if let Ok(Some(user)) = state.db.find_user_by_id(repo.user_id).await {
            Some(format!("/app/{}/{}", user.username, repo.name))
        } else { None }
    } else { None };

    Ok(Json(json!({
        "apps_config": cfg,
        "status": app_status.map(|s| format!("{:?}", s).to_lowercase()),
        "port": port,
        "url": url,
    })))
}

async fn do_deploy(state: &AppState, repo_id: i64, user_id: i64) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;
    let user = state.db.find_user_by_id(user_id).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;
    let cfg = state.db.get_apps_config(repo_id).await?
        .ok_or_else(|| AppError::NotFound("App 設定不存在".into()))?;

    let repo_path = state.config.repo_path(&user.username, &repo.name);
    let workspace = state.config.app_workspace_dir(&user.username, &repo.name);

    // Create deploy log
    let mut deploy_log = state.db.create_deploy_log(repo_id).await?;

    match crate::deploy::deploy_app(
        &state.app_manager,
        &repo_path,
        &workspace,
        &cfg,
        &user.username,
        &repo.name,
        repo_id,
    ).await {
        Ok((port, log)) => {
            state.db.update_deploy_log(deploy_log.id, "success", &log).await?;
            Ok(Json(json!({
                "success": true,
                "port": port,
                "url": format!("/app/{}/{}", user.username, repo.name),
                "deploy_log_id": deploy_log.id,
            })))
        }
        Err(e) => {
            let log = format!("Deploy failed: {}", e);
            state.db.update_deploy_log(deploy_log.id, "failed", &log).await?;
            Ok(Json(json!({
                "success": false,
                "deploy_error": format!("{}", e),
                "deploy_log_id": deploy_log.id,
            })))
        }
    }
}

pub async fn update_apps_config(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
    Json(req): Json<UpdateAppsConfig>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    if repo.user_id != user_id {
        return Err(AppError::Unauthorized("無權限修改設定".into()));
    }

    let branch = req.branch.unwrap_or_else(|| "main".to_string());
    let source_dir = req.source_dir.unwrap_or_else(|| "/".to_string());
    let build_command = req.build_command.unwrap_or_default();
    let start_command = req.start_command.unwrap_or_default();
    let env_vars = req.env_vars.unwrap_or_else(|| "{}".to_string());
    let enabled = req.enabled.unwrap_or(false);

    state.db.upsert_apps_config(repo_id, &branch, &source_dir, &build_command, &start_command, &env_vars, enabled).await?;

    if enabled {
        do_deploy(&state, repo_id, user_id).await
    } else {
        crate::deploy::stop_app(&state.app_manager, repo_id).await;
        state.db.delete_apps_config(repo_id).await?;
        state.app_manager.unregister(repo_id).await;
        Ok(Json(json!({ "success": true })))
    }
}

pub async fn deploy_apps_handler(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    if repo.user_id != user_id {
        return Err(AppError::Unauthorized("無權限操作".into()));
    }

    do_deploy(&state, repo_id, user_id).await
}

pub async fn delete_apps_handler(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    if repo.user_id != user_id {
        return Err(AppError::Unauthorized("無權限操作".into()));
    }

    crate::deploy::stop_app(&state.app_manager, repo_id).await;
    state.db.delete_apps_config(repo_id).await?;
    state.app_manager.unregister(repo_id).await;

    Ok(Json(json!({ "success": true })))
}

pub async fn list_deploys(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let deploy_logs = state.db.get_deploy_logs(repo_id).await?;
    Ok(Json(json!({ "deploy_logs": deploy_logs })))
}

pub async fn get_deploy_log(
    State(state): State<AppState>,
    Path((repo_id, deploy_id)): Path<(i64, i64)>,
) -> Result<Json<Value>, AppError> {
    let log = state.db.get_deploy_log(deploy_id).await?
        .ok_or_else(|| AppError::NotFound("部署日誌不存在".into()))?;

    if log.repo_id != repo_id {
        return Err(AppError::NotFound("該部署日誌不屬於此倉庫".into()));
    }

    Ok(Json(json!({ "deploy_log": log })))
}
