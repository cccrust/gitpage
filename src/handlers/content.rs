use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::app::AppState;
use crate::db::models::Repository;
use crate::git;
use crate::utils::errors::AppError;

async fn resolve_repo(state: &AppState, username: &str, repo_name: &str, auth_user_id: Option<i64>) -> Result<Repository, AppError> {
    let user = state.db.find_user_by_username(username).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;
    let repo = state.db.find_repo_by_name(user.id, repo_name).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    if repo.is_private {
        match auth_user_id {
            Some(uid) if uid == repo.user_id => {}
            _ => return Err(AppError::Unauthorized("私有倉庫".into())),
        }
    }

    Ok(repo)
}

#[derive(Debug, Deserialize)]
pub struct TreeQuery {
    pub branch: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BlobQuery {
    pub branch: Option<String>,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct ReadmeQuery {
    pub branch: Option<String>,
}

pub async fn list_directory(
    State(state): State<AppState>,
    Path((username, repo_name)): Path<(String, String)>,
    Query(query): Query<TreeQuery>,
    user_id: Option<axum::Extension<i64>>,
) -> Result<Json<Value>, AppError> {
    let uid = user_id.map(|e| e.0);
    let repo = resolve_repo(&state, &username, &repo_name, uid).await?;

    let branch = query.branch.as_deref().unwrap_or(&repo.default_branch);
    let path = query.path.as_deref().unwrap_or("");

    let repo_path = state.config.repo_path(&username, &repo_name);
    if !git::repo_exists(&repo_path) {
        return Ok(Json(json!({ "entries": [], "repo": repo, "branch": branch, "path": path })));
    }

    let entries = git::list_directory(&repo_path, branch, path)?;
    let entries_json: Vec<Value> = entries.iter().map(|(name, is_dir)| {
        json!({ "name": name, "is_dir": is_dir })
    }).collect();

    Ok(Json(json!({
        "entries": entries_json,
        "repo": repo,
        "branch": branch,
        "path": path
    })))
}

pub async fn get_file_content(
    State(state): State<AppState>,
    Path((username, repo_name)): Path<(String, String)>,
    Query(query): Query<BlobQuery>,
    user_id: Option<axum::Extension<i64>>,
) -> Result<Json<Value>, AppError> {
    let uid = user_id.map(|e| e.0);
    let repo = resolve_repo(&state, &username, &repo_name, uid).await?;

    let branch = query.branch.as_deref().unwrap_or(&repo.default_branch);
    let path = query.path.trim_start_matches('/');

    let repo_path = state.config.repo_path(&username, &repo_name);
    let result = git::get_file_content(&repo_path, branch, path)?;

    match result {
        Some((content, kind)) => {
            let is_markdown = path.ends_with(".md") || path.ends_with(".markdown");
            let content_str = String::from_utf8_lossy(&content).to_string();

            let rendered = if is_markdown {
                Some(render_markdown(&content_str))
            } else {
                None
            };

            Ok(Json(json!({
                "content": content_str,
                "mime_type": kind,
                "is_markdown": is_markdown,
                "rendered": rendered,
                "repo": repo,
                "branch": branch,
                "path": path
            })))
        }
        None => Err(AppError::NotFound("檔案不存在".into())),
    }
}

pub async fn get_readme(
    State(state): State<AppState>,
    Path((username, repo_name)): Path<(String, String)>,
    Query(query): Query<ReadmeQuery>,
    user_id: Option<axum::Extension<i64>>,
) -> Result<Json<Value>, AppError> {
    let uid = user_id.map(|e| e.0);
    let _repo = resolve_repo(&state, &username, &repo_name, uid).await?;

    let branch = query.branch.as_deref().unwrap_or("main");
    let repo_path = state.config.repo_path(&username, &repo_name);
    let readme = git::get_readme(&repo_path, branch)?;

    match readme {
        Some(content) => {
            let rendered = render_markdown(&content);
            Ok(Json(json!({
                "content": content,
                "rendered": rendered,
                "has_readme": true
            })))
        }
        None => Ok(Json(json!({ "has_readme": false }))),
    }
}

pub async fn list_commits(
    State(state): State<AppState>,
    Path((username, repo_name, branch)): Path<(String, String, String)>,
    user_id: Option<axum::Extension<i64>>,
) -> Result<Json<Value>, AppError> {
    let uid = user_id.map(|e| e.0);
    let repo = resolve_repo(&state, &username, &repo_name, uid).await?;

    let repo_path = state.config.repo_path(&username, &repo_name);
    let commits = git::get_commit_log(&repo_path, &branch, 50)?;

    let commits_json: Vec<Value> = commits.iter().map(|(sha, msg, author, time)| {
        json!({ "sha": sha, "message": msg, "author": author, "time": time })
    }).collect();

    Ok(Json(json!({
        "commits": commits_json,
        "repo": repo,
        "branch": branch
    })))
}

fn render_markdown(text: &str) -> String {
    let parser = pulldown_cmark::Parser::new(text);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}
