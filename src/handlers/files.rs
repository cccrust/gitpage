use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path as StdPath;

use crate::app::AppState;
use crate::db::models::FileEntry;
use crate::git;
use crate::utils::errors::AppError;

fn staging_base<'a>(state: &'a AppState, username: &str, repo: &str) -> String {
    state.config.staging_path(username, repo)
}

fn safe_path(base: &str, file_path: &str) -> Result<String, AppError> {
    if file_path.contains("..") {
        return Err(AppError::BadRequest("不允許的路徑跳躍".into()));
    }
    let clean = file_path.trim_start_matches('/');
    Ok(format!("{}/{}", base.trim_end_matches('/'), clean))
}

fn list_staging_dir(base: &str, dir_path: &str) -> Result<Vec<FileEntry>, AppError> {
    let dir = StdPath::new(base).join(dir_path.trim_start_matches('/'));
    let mut entries = Vec::new();
    if dir.exists() && dir.is_dir() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') { continue; }
            let file_type = entry.file_type()?;
            let is_dir = file_type.is_dir();
            let size = if file_type.is_file() { Some(entry.metadata()?.len() as i64) } else { None };
            let modified = entry.metadata()?
                .modified()
                .ok()
                .and_then(|t| {
                    let duration = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                    let dt = chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0)?;
                    Some(dt.format("%Y-%m-%d %H:%M:%S").to_string())
                })
                .unwrap_or_default();
            entries.push(FileEntry { name, is_dir, size, updated_at: modified });
        }
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(entries)
}

fn list_staging_changes(base: &str) -> Result<Vec<serde_json::Value>, AppError> {
    let dir = StdPath::new(base);
    let mut changes = Vec::new();
    if !dir.exists() { return Ok(changes); }
    fn walk(d: &StdPath, prefix: &str, changes: &mut Vec<serde_json::Value>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(d)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') { continue; }
            let rel = if prefix.is_empty() { name.clone() } else { format!("{}/{}", prefix, name) };
            if entry.file_type()?.is_dir() {
                walk(&entry.path(), &rel, changes)?;
            } else {
                changes.push(json!({
                    "path": rel,
                    "change_type": "added"
                }));
            }
        }
        Ok(())
    }
    walk(dir, "", &mut changes)?;
    Ok(changes)
}

#[derive(Deserialize)]
pub struct TreeQuery {
    pub path: Option<String>,
}

pub async fn tree(
    State(state): State<AppState>,
    axum::Extension(username): axum::Extension<String>,
    Path(repo_id): Path<i64>,
    Query(query): Query<TreeQuery>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let base = staging_base(&state, &username, &repo.name);
    let entries = list_staging_dir(&base, query.path.as_deref().unwrap_or("/"))?;
    Ok(Json(json!({ "entries": entries, "path": query.path.unwrap_or_else(|| "/".to_string()) })))
}

#[derive(Deserialize)]
pub struct RawQuery {
    pub path: String,
}

pub async fn raw(
    State(state): State<AppState>,
    axum::Extension(username): axum::Extension<String>,
    Path(repo_id): Path<i64>,
    Query(query): Query<RawQuery>,
) -> Result<(StatusCode, [(String, String); 1], Vec<u8>), AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let base = staging_base(&state, &username, &repo.name);
    let full = safe_path(&base, &query.path)?;
    let content = std::fs::read(&full)
        .map_err(|_| AppError::NotFound("檔案不存在".into()))?;

    let ext = StdPath::new(&query.path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let mime = mime_guess::from_ext(ext).first_or_octet_stream().to_string();

    Ok((StatusCode::OK, [("Content-Type".to_string(), mime)], content))
}

#[derive(Deserialize)]
pub struct WriteQuery {
    pub path: String,
}

pub async fn write_file(
    State(state): State<AppState>,
    axum::Extension(username): axum::Extension<String>,
    Path(repo_id): Path<i64>,
    Query(query): Query<WriteQuery>,
    body: axum::body::Bytes,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let base = staging_base(&state, &username, &repo.name);
    let full = safe_path(&base, &query.path)?;
    if let Some(parent) = StdPath::new(&full).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full, &body)?;

    Ok(Json(json!({ "success": true, "path": query.path })))
}

#[derive(Deserialize)]
pub struct DeleteQuery {
    pub path: String,
}

pub async fn delete_file(
    State(state): State<AppState>,
    axum::Extension(username): axum::Extension<String>,
    Path(repo_id): Path<i64>,
    Query(query): Query<DeleteQuery>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let base = staging_base(&state, &username, &repo.name);
    let full = safe_path(&base, &query.path)?;
    let p = StdPath::new(&full);
    if p.exists() {
        if p.is_dir() {
            std::fs::remove_dir_all(p)?;
        } else {
            std::fs::remove_file(p)?;
        }
    }

    Ok(Json(json!({ "success": true, "path": query.path })))
}

#[derive(Deserialize)]
pub struct MkdirQuery {
    pub path: String,
}

pub async fn mkdir(
    State(state): State<AppState>,
    axum::Extension(username): axum::Extension<String>,
    Path(repo_id): Path<i64>,
    Query(query): Query<MkdirQuery>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let base = staging_base(&state, &username, &repo.name);
    let full = safe_path(&base, &query.path)?;
    std::fs::create_dir_all(&full)?;

    Ok(Json(json!({ "success": true, "path": query.path })))
}

#[derive(Deserialize)]
pub struct MoveQuery {
    pub from: String,
    pub to: String,
}

pub async fn move_file(
    State(state): State<AppState>,
    axum::Extension(username): axum::Extension<String>,
    Path(repo_id): Path<i64>,
    Query(query): Query<MoveQuery>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let base = staging_base(&state, &username, &repo.name);
    if query.from.contains("..") || query.to.contains("..") {
        return Err(AppError::BadRequest("不允許的路徑跳躍".into()));
    }
    let src = safe_path(&base, &query.from)?;
    let dst = safe_path(&base, &query.to)?;
    if let Some(parent) = StdPath::new(&dst).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&src, &dst)?;

    Ok(Json(json!({ "success": true, "from": query.from, "to": query.to })))
}

pub async fn status(
    State(state): State<AppState>,
    axum::Extension(username): axum::Extension<String>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let base = staging_base(&state, &username, &repo.name);
    let changes = list_staging_changes(&base)?;
    let pending = !changes.is_empty();

    Ok(Json(json!({ "pending": pending, "changes": changes })))
}

#[derive(Deserialize)]
pub struct CommitBody {
    pub message: String,
}

pub async fn commit(
    State(state): State<AppState>,
    axum::Extension(username): axum::Extension<String>,
    Path(repo_id): Path<i64>,
    Json(body): Json<CommitBody>,
) -> Result<Json<Value>, AppError> {
    if body.message.trim().is_empty() {
        return Err(AppError::BadRequest("Commit 訊息不能為空".into()));
    }

    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let bare_path = state.config.repo_path(&username, &repo.name);
    let staging_path = state.config.staging_path(&username, &repo.name);

    // Check staging has files
    let changes = list_staging_changes(&staging_path)?;
    if changes.is_empty() {
        return Err(AppError::BadRequest("沒有變更需要提交".into()));
    }

    git::commit_staging(&bare_path, &staging_path, &body.message, &username, &repo.default_branch)?;

    // Trigger auto-deploy
    tokio::spawn(crate::app::auto_deploy_pages(
        state.clone(),
        username.clone(),
        repo.name.clone(),
    ));
    tokio::spawn(crate::app::auto_deploy_app(
        state.clone(),
        username.clone(),
        repo.name.clone(),
    ));

    Ok(Json(json!({ "success": true, "message": body.message })))
}
