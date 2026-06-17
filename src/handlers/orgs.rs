use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::app::AppState;
use crate::db::models::*;
use crate::utils::errors::AppError;

#[derive(Deserialize)]
pub struct CreateOrgRequest {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

pub async fn create_org(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Json(req): Json<CreateOrgRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    if req.name.len() < 2 {
        return Err(AppError::BadRequest("組織名稱至少需要 2 個字元".into()));
    }

    if state.db.find_org_by_name(&req.name).await?.is_some() {
        return Err(AppError::Conflict("組織名稱已存在".into()));
    }
    if (state.db.find_user_by_username(&req.name).await?).is_some() {
        return Err(AppError::Conflict("該名稱已被使用者使用".into()));
    }

    let display_name = req.display_name.unwrap_or_else(|| req.name.clone());
    let description = req.description.unwrap_or_default();

    let org = state.db.create_org(&req.name, &display_name, &description, user_id).await?;

    // Creator becomes admin
    state.db.add_org_member(org.id, user_id, "admin").await?;

    Ok((StatusCode::CREATED, Json(json!({ "org": org }))))
}

pub async fn get_org(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, AppError> {
    let org = state.db.find_org_by_name(&name).await?
        .ok_or_else(|| AppError::NotFound("組織不存在".into()))?;

    let owner = state.db.find_user_by_id(org.owner_id).await?;
    let owner_name = owner.map(|u| u.username).unwrap_or_default();

    Ok(Json(json!({ "org": org, "owner_name": owner_name })))
}

pub async fn update_org(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(name): Path<String>,
    Json(req): Json<CreateOrgRequest>,
) -> Result<Json<Value>, AppError> {
    let org = state.db.find_org_by_name(&name).await?
        .ok_or_else(|| AppError::NotFound("組織不存在".into()))?;

    // Only admin or owner can update
    let role = get_user_org_role(&state, org.id, user_id).await?;
    if role != "admin" && org.owner_id != user_id {
        return Err(AppError::Unauthorized("無權限修改組織".into()));
    }

    let display_name = req.display_name.unwrap_or(org.display_name);
    let description = req.description.unwrap_or(org.description);
    state.db.update_org(org.id, &display_name, &description).await?;

    Ok(Json(json!({ "success": true })))
}

pub async fn delete_org(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(name): Path<String>,
) -> Result<Json<Value>, AppError> {
    let org = state.db.find_org_by_name(&name).await?
        .ok_or_else(|| AppError::NotFound("組織不存在".into()))?;

    if org.owner_id != user_id {
        return Err(AppError::Unauthorized("只有建立者可以刪除組織".into()));
    }

    state.db.delete_org(org.id).await?;

    Ok(Json(json!({ "deleted": true })))
}

pub async fn list_my_orgs(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
) -> Result<Json<Value>, AppError> {
    let orgs: Vec<OrganizationWithRole> = state.db.list_user_orgs(user_id).await?;
    Ok(Json(json!({ "orgs": orgs })))
}

pub async fn list_org_repos(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, AppError> {
    let org = state.db.find_org_by_name(&name).await?
        .ok_or_else(|| AppError::NotFound("組織不存在".into()))?;

    let repos: Vec<OrgRepoResult> = state.db.list_org_repos_with_orgname(org.id).await?;
    Ok(Json(json!({ "repos": repos, "org": org })))
}

pub async fn list_members(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, AppError> {
    let org = state.db.find_org_by_name(&name).await?
        .ok_or_else(|| AppError::NotFound("組織不存在".into()))?;

    let members = state.db.list_org_members(org.id).await?;
    let member_list: Vec<Value> = members.into_iter().map(|(m, u)| {
        json!({
            "id": m.id,
            "user_id": m.user_id,
            "role": m.role,
            "username": u.username,
            "bio": u.bio,
            "created_at": m.created_at,
        })
    }).collect();

    Ok(Json(json!({ "members": member_list, "org": org })))
}

#[derive(Deserialize)]
pub struct AddMemberRequest {
    pub username: String,
    pub role: Option<String>,
}

pub async fn add_member(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(name): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> Result<Json<Value>, AppError> {
    let org = state.db.find_org_by_name(&name).await?
        .ok_or_else(|| AppError::NotFound("組織不存在".into()))?;

    let role = get_user_org_role(&state, org.id, user_id).await?;
    if role != "admin" && org.owner_id != user_id {
        return Err(AppError::Unauthorized("無權限管理成員".into()));
    }

    let target = state.db.find_user_by_username(&req.username).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;

    let member_role = req.role.unwrap_or_else(|| "member".to_string());
    let member: OrganizationMember = state.db.add_org_member(org.id, target.id, &member_role).await?;

    Ok(Json(json!({ "success": true, "member": member })))
}

pub async fn remove_member(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path((name, target_user_id)): Path<(String, i64)>,
) -> Result<Json<Value>, AppError> {
    let org = state.db.find_org_by_name(&name).await?
        .ok_or_else(|| AppError::NotFound("組織不存在".into()))?;

    let role = get_user_org_role(&state, org.id, user_id).await?;
    if role != "admin" && org.owner_id != user_id {
        return Err(AppError::Unauthorized("無權限管理成員".into()));
    }

    if target_user_id == org.owner_id {
        return Err(AppError::BadRequest("不能移除組織建立者".into()));
    }

    state.db.remove_org_member(org.id, target_user_id).await?;

    Ok(Json(json!({ "success": true })))
}

async fn get_user_org_role(state: &AppState, org_id: i64, user_id: i64) -> Result<String, AppError> {
    let members = state.db.list_org_members(org_id).await?;
    Ok(members.into_iter()
        .find(|(_, u)| u.id == user_id)
        .map(|(m, _)| m.role)
        .unwrap_or_default())
}
