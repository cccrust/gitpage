use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::app::AppState;
use crate::utils::errors::AppError;

#[derive(Deserialize)]
pub struct ListIssuesQuery {
    pub state: Option<String>,
}

pub async fn list_issues(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
    Query(query): Query<ListIssuesQuery>,
) -> Result<Json<Value>, AppError> {
    let issues = state.db.list_issues(repo_id, query.state.as_deref()).await?;
    Ok(Json(json!({ "issues": issues })))
}

#[derive(Deserialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub body: String,
    pub assignee_id: Option<i64>,
    pub label_ids: Option<Vec<i64>>,
}

pub async fn create_issue(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
    Json(req): Json<CreateIssueRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let can_write = repo.user_id == user_id || repo.owner_type == "org";
    if !can_write {
        return Err(AppError::Unauthorized("無權限建立 Issue".into()));
    }

    if req.title.is_empty() {
        return Err(AppError::BadRequest("標題不能為空".into()));
    }

    let number = state.db.next_issue_number(repo_id).await?;
    let issue = state.db.create_issue(repo_id, number, &req.title, &req.body, user_id, req.assignee_id).await?;

    if let Some(label_ids) = req.label_ids {
        if !label_ids.is_empty() {
            state.db.set_issue_labels(issue.issue.id, &label_ids).await?;
        }
    }

    Ok((StatusCode::CREATED, Json(json!({ "issue": issue }))))
}

#[derive(Deserialize)]
pub struct IssuePath {
    pub repo_id: i64,
    pub issue_number: i64,
}

pub async fn get_issue(
    State(state): State<AppState>,
    Path(path): Path<IssuePath>,
) -> Result<Json<Value>, AppError> {
    let issue = state.db.get_issue(path.repo_id, path.issue_number).await?
        .ok_or_else(|| AppError::NotFound("Issue 不存在".into()))?;
    Ok(Json(json!({ "issue": issue })))
}

#[derive(Deserialize)]
pub struct UpdateIssueRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub state: Option<String>,
    pub assignee_id: Option<Option<i64>>,
    pub label_ids: Option<Vec<i64>>,
}

pub async fn update_issue(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(path): Path<IssuePath>,
    Json(req): Json<UpdateIssueRequest>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(path.repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let can_write = repo.user_id == user_id || repo.owner_type == "org";
    if !can_write {
        return Err(AppError::Unauthorized("無權限修改 Issue".into()));
    }

    let existing = state.db.get_issue(path.repo_id, path.issue_number).await?
        .ok_or_else(|| AppError::NotFound("Issue 不存在".into()))?;

    let updated = state.db.update_issue(
        existing.issue.id,
        path.repo_id,
        req.title.as_deref(),
        req.body.as_deref(),
        req.state.as_deref(),
        req.assignee_id,
    ).await?;

    if let Some(label_ids) = req.label_ids {
        state.db.set_issue_labels(existing.issue.id, &label_ids).await?;
    }

    Ok(Json(json!({ "updated": updated })))
}

pub async fn delete_issue(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(path): Path<IssuePath>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(path.repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let can_delete = repo.user_id == user_id || repo.owner_type == "org";
    if !can_delete {
        return Err(AppError::Unauthorized("無權限刪除 Issue".into()));
    }

    let existing = state.db.get_issue(path.repo_id, path.issue_number).await?
        .ok_or_else(|| AppError::NotFound("Issue 不存在".into()))?;

    let deleted = state.db.delete_issue(existing.issue.id, path.repo_id).await?;
    Ok(Json(json!({ "deleted": deleted })))
}

// ── Labels ──

pub async fn list_labels(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let labels = state.db.list_labels(repo_id).await?;
    Ok(Json(json!({ "labels": labels })))
}

#[derive(Deserialize)]
pub struct CreateLabelRequest {
    pub name: String,
    pub color: Option<String>,
}

pub async fn create_label(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
    Json(req): Json<CreateLabelRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let can_write = repo.user_id == user_id || repo.owner_type == "org";
    if !can_write {
        return Err(AppError::Unauthorized("無權限建立標籤".into()));
    }

    let color = req.color.unwrap_or_else(|| "#0366d6".to_string());
    let label = state.db.create_label(repo_id, &req.name, &color).await?;
    Ok((StatusCode::CREATED, Json(json!({ "label": label }))))
}

#[derive(Deserialize)]
pub struct LabelPath {
    pub repo_id: i64,
    pub label_id: i64,
}

pub async fn delete_label(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(path): Path<LabelPath>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(path.repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let can_delete = repo.user_id == user_id || repo.owner_type == "org";
    if !can_delete {
        return Err(AppError::Unauthorized("無權限刪除標籤".into()));
    }

    let deleted = state.db.delete_label(path.label_id, path.repo_id).await?;
    Ok(Json(json!({ "deleted": deleted })))
}

// ── Comments ──

#[derive(Deserialize)]
pub struct AddCommentRequest {
    pub body: String,
}

pub async fn add_comment(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(path): Path<IssuePath>,
    Json(req): Json<AddCommentRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let issue = state.db.get_issue(path.repo_id, path.issue_number).await?
        .ok_or_else(|| AppError::NotFound("Issue 不存在".into()))?;

    if req.body.is_empty() {
        return Err(AppError::BadRequest("留言內容不能為空".into()));
    }

    let comment = state.db.add_comment(issue.issue.id, user_id, &req.body).await?;
    Ok((StatusCode::CREATED, Json(json!({ "comment": comment }))))
}

pub async fn list_comments(
    State(state): State<AppState>,
    Path(path): Path<IssuePath>,
) -> Result<Json<Value>, AppError> {
    let issue = state.db.get_issue(path.repo_id, path.issue_number).await?
        .ok_or_else(|| AppError::NotFound("Issue 不存在".into()))?;

    let comments = state.db.list_comments(issue.issue.id).await?;
    Ok(Json(json!({ "comments": comments })))
}
