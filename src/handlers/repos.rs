use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::process::Command;

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
    let mut repos: Vec<serde_json::Value> = Vec::new();
    let mut owner_type = "user";

    if let Ok(Some(user)) = state.db.find_user_by_username(&username).await {
        let user_repos = state.db.list_public_user_repos(user.id).await?;
        repos.extend(user_repos.into_iter().map(|r| json!({ "repo": r, "owner_type": "user" })));
    }

    if let Ok(Some(org)) = state.db.find_org_by_name(&username).await {
        owner_type = "org";
        let org_repos = state.db.list_org_repos_with_orgname(org.id).await?;
        repos.extend(org_repos.into_iter().map(|r| json!({ "repo": r, "org_name": r.org_name.clone(), "owner_type": "org" })));
    }

    Ok(Json(json!({ "repos": repos, "user": username, "owner_type": owner_type })))
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
    let owner_name: String;
    let owner_type: &str;
    let org_id: Option<i64>;

    if let Some(ref org_name) = req.org_name {
        let org = state.db.find_org_by_name(org_name).await?
            .ok_or_else(|| AppError::NotFound("組織不存在".into()))?;
        owner_name = org_name.clone();
        owner_type = "org";
        org_id = Some(org.id);
        if (state.db.find_org_repo_by_name(org.id, &req.name).await?).is_some() {
            return Err(AppError::BadRequest("組織內已有同名倉庫".into()));
        }
    } else {
        owner_name = user.username.clone();
        owner_type = "user";
        org_id = None;
        if (state.db.find_repo_by_name(user_id, &req.name).await?).is_some() {
            return Err(AppError::BadRequest("已有同名倉庫".into()));
        }
    }

    let repo = state.db.create_repo(user_id, &req.name, &description, is_private, owner_type, org_id).await?;

    let repo_path = state.config.repo_path(&owner_name, &req.name);
    git::init_bare_repo(&repo_path)?;

    let staging_path = state.config.staging_path(&owner_name, &req.name);
    std::fs::create_dir_all(&staging_path)?;

    Ok((StatusCode::CREATED, Json(json!({ "repo": repo }))))
}

pub async fn get_repo(
    State(state): State<AppState>,
    Path((username, repo_name)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    // Try user first, then org
    if let Ok(Some(user)) = state.db.find_user_by_username(&username).await {
        if let Some(repo) = state.db.find_repo_by_name(user.id, &repo_name).await? {
            return Ok(Json(json!({ "repo": repo, "user": username, "owner_type": "user" })));
        }
    }

    if let Ok(Some(org)) = state.db.find_org_by_name(&username).await {
        if let Some(repo) = state.db.find_org_repo_by_name(org.id, &repo_name).await? {
            return Ok(Json(json!({ "repo": repo, "org_name": username, "owner_type": "org" })));
        }
    }

    Err(AppError::NotFound("使用者或組織不存在".into()))
}

async fn resolve_owner_name(state: &AppState, repo: &Repository) -> Result<String, AppError> {
    if repo.owner_type == "org" {
        if let Some(oid) = repo.org_id {
            let org = state.db.find_org_by_id(oid).await?
                .ok_or_else(|| AppError::NotFound("組織不存在".into()))?;
            return Ok(org.name);
        }
    }
    let user = state.db.find_user_by_id(repo.user_id).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;
    Ok(user.username)
}

pub async fn delete_repo(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    // Allow owner or org admin to delete
    let can_delete = if repo.owner_type == "org" && repo.org_id.is_some() {
        let members = state.db.list_org_members(repo.org_id.unwrap()).await?;
        members.iter().any(|(m, u)| u.id == user_id && (m.role == "admin" || u.id == repo.user_id))
    } else {
        repo.user_id == user_id
    };

    if !can_delete {
        return Err(AppError::Unauthorized("無權限刪除此倉庫".into()));
    }

    let owner_name = resolve_owner_name(&state, &repo).await?;

    let repo_path = state.config.repo_path(&owner_name, &repo.name);
    if std::path::Path::new(&repo_path).exists() {
        std::fs::remove_dir_all(&repo_path)?;
    }

    let staging_path = state.config.staging_path(&owner_name, &repo.name);
    if std::path::Path::new(&staging_path).exists() {
        std::fs::remove_dir_all(&staging_path)?;
    }

    let pages_dir = state.config.pages_dir(&owner_name, &repo.name);
    if std::path::Path::new(&pages_dir).exists() {
        std::fs::remove_dir_all(&pages_dir)?;
    }

    let app_workspace = state.config.app_workspace_dir(&owner_name, &repo.name);
    if std::path::Path::new(&app_workspace).exists() {
        std::fs::remove_dir_all(&app_workspace)?;
    }

    // Kill running app if any
    crate::deploy::stop_app(&state.app_manager, repo_id, state.docker.as_ref()).await;

    state.db.delete_repo(repo_id).await?;

    Ok(Json(json!({ "deleted": true })))
}

pub async fn get_repo_by_id(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let owner_name = resolve_owner_name(&state, &repo).await?;

    if repo.owner_type == "org" {
        Ok(Json(json!({ "repo": repo, "org_name": owner_name, "username": "" })))
    } else {
        Ok(Json(json!({ "repo": repo, "username": owner_name })))
    }
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub async fn search_repos(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Value>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let (repos, total) = state.db.search_repos(&query.q, page, page_size).await?;
    let total_pages = (total as f64 / page_size as f64).ceil() as i64;
    Ok(Json(json!({
        "repos": repos,
        "total": total,
        "page": page,
        "page_size": page_size,
        "total_pages": total_pages,
        "query": query.q
    })))
}

#[derive(Deserialize)]
pub struct UpdateRepoRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_private: Option<bool>,
}

pub async fn update_repo_handler(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
    Json(req): Json<UpdateRepoRequest>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let can_update = if repo.owner_type == "org" && repo.org_id.is_some() {
        let members = state.db.list_org_members(repo.org_id.unwrap()).await?;
        members.iter().any(|(m, u)| u.id == user_id && (m.role == "admin" || u.id == repo.user_id))
    } else {
        repo.user_id == user_id
    };

    if !can_update {
        return Err(AppError::Unauthorized("無權限修改".into()));
    }

    let owner_name = resolve_owner_name(&state, &repo).await?;

    let description = req.description.unwrap_or(repo.description);
    let is_private = req.is_private.unwrap_or(repo.is_private);

    if let Some(ref new_name) = req.name {
        if !new_name.is_empty() && *new_name != repo.name {
            let old_repo_path = state.config.repo_path(&owner_name, &repo.name);
            let new_repo_path = state.config.repo_path(&owner_name, new_name);
            let old_staging = state.config.staging_path(&owner_name, &repo.name);
            let new_staging = state.config.staging_path(&owner_name, new_name);

            if std::path::Path::new(&old_repo_path).exists() {
                std::fs::rename(&old_repo_path, &new_repo_path)?;
            }
            if std::path::Path::new(&old_staging).exists() {
                std::fs::rename(&old_staging, &new_staging)?;
            }
        }
    }

    let name = req.name.as_deref().unwrap_or(&repo.name);
    state.db.update_repo(repo_id, name, &description, is_private).await?;

    Ok(Json(json!({ "success": true })))
}

#[derive(Deserialize)]
pub struct ForkRequest {
    pub owner_name: String,
}

pub async fn fork_repo(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(source_id): Path<i64>,
    Json(_req): Json<ForkRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let source_repo = state.db.find_repo_by_id(source_id).await?
        .ok_or_else(|| AppError::NotFound("來源倉庫不存在".into()))?;

    let user = state.db.find_user_by_id(user_id).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;

    // Check for existing fork
    let user_repos = state.db.list_user_repos_all(user_id).await?;
    if user_repos.iter().any(|r| r.forked_from == Some(source_id)) {
        return Err(AppError::BadRequest("已經 Fork 過此倉庫".into()));
    }

    // Ensure unique name
    if user_repos.iter().any(|r| r.name == source_repo.name) {
        return Err(AppError::BadRequest("已有同名倉庫".into()));
    }

    let source_owner = if source_repo.owner_type == "org" {
        if let Some(oid) = source_repo.org_id {
            let org = state.db.find_org_by_id(oid).await?
                .ok_or_else(|| AppError::NotFound("組織不存在".into()))?;
            org.name
        } else {
            user.username.clone()
        }
    } else {
        let owner = state.db.find_user_by_id(source_repo.user_id).await?
            .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;
        owner.username
    };

    // Create the new repo
    let new_repo = state.db.create_repo(
        user_id, &source_repo.name, &source_repo.description,
        source_repo.is_private, "user", None,
    ).await?;

    state.db.set_repo_forked_from(new_repo.id, source_id).await?;

    // Clone the bare repo
    let source_path = state.config.repo_path(&source_owner, &source_repo.name);
    let new_path = state.config.repo_path(&user.username, &source_repo.name);

    let output = Command::new("git")
        .args(["clone", "--bare", &source_path, &new_path])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Cleanup
        let _ = std::fs::remove_dir_all(&new_path);
        let _ = state.db.delete_repo(new_repo.id).await;
        return Err(AppError::Internal(format!("Failed to clone repo: {}", stderr)));
    }

    // Create staging directory
    let staging_path = state.config.staging_path(&user.username, &source_repo.name);
    std::fs::create_dir_all(&staging_path)?;

    Ok((StatusCode::CREATED, Json(json!({ "repo": new_repo }))))
}
