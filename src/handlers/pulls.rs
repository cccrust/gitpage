use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::app::AppState;
use crate::db::models::*;
use crate::utils::errors::AppError;

#[derive(Deserialize)]
pub struct ListPrsQuery {
    pub state: Option<String>,
}

pub async fn list_prs(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
    Query(query): Query<ListPrsQuery>,
) -> Result<Json<Value>, AppError> {
    let prs = state.db.list_prs(repo_id, query.state.as_deref()).await?;
    Ok(Json(json!({ "pulls": prs })))
}

#[derive(Deserialize)]
pub struct CreatePrRequest {
    pub title: String,
    pub body: String,
    pub head_repo_id: i64,
    pub head_ref: String,
    pub base_ref: String,
}

pub async fn create_pr(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
    Json(req): Json<CreatePrRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let can_write = repo.user_id == user_id || repo.owner_type == "org";
    if !can_write {
        return Err(AppError::Unauthorized("無權限建立 Pull Request".into()));
    }

    if req.title.is_empty() {
        return Err(AppError::BadRequest("標題不能為空".into()));
    }

    let number = state.db.next_pr_number(repo_id).await?;
    let pr = state.db.create_pr(
        repo_id, number, &req.title, &req.body, user_id,
        req.head_repo_id, &req.head_ref, &req.base_ref,
    ).await?;

    Ok((StatusCode::CREATED, Json(json!({ "pull": pr }))))
}

#[derive(Deserialize)]
pub struct PrPath {
    pub repo_id: i64,
    pub pr_number: i64,
}

pub async fn get_pr(
    State(state): State<AppState>,
    Path(path): Path<PrPath>,
) -> Result<Json<Value>, AppError> {
    let pr = state.db.get_pr(path.repo_id, path.pr_number).await?
        .ok_or_else(|| AppError::NotFound("Pull Request 不存在".into()))?;
    Ok(Json(json!({ "pull": pr })))
}

#[derive(Deserialize)]
pub struct UpdatePrRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub state: Option<String>,
}

pub async fn update_pr(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(path): Path<PrPath>,
    Json(req): Json<UpdatePrRequest>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(path.repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let can_write = repo.user_id == user_id || repo.owner_type == "org";
    if !can_write {
        return Err(AppError::Unauthorized("無權限修改 Pull Request".into()));
    }

    let existing = state.db.get_pr(path.repo_id, path.pr_number).await?
        .ok_or_else(|| AppError::NotFound("Pull Request 不存在".into()))?;

    let updated = state.db.update_pr(
        existing.pr.id, path.repo_id,
        req.title.as_deref(),
        req.body.as_deref(),
        req.state.as_deref(),
    ).await?;

    Ok(Json(json!({ "updated": updated })))
}

pub async fn merge_pr(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(path): Path<PrPath>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(path.repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let can_write = repo.user_id == user_id || repo.owner_type == "org";
    if !can_write {
        return Err(AppError::Unauthorized("無權限合併 Pull Request".into()));
    }

    let pr_with_author = state.db.get_pr(path.repo_id, path.pr_number).await?
        .ok_or_else(|| AppError::NotFound("Pull Request 不存在".into()))?;
    let pr_id = pr_with_author.pr.id;
    let pr_state = pr_with_author.pr.state.clone();
    let base_ref_str = pr_with_author.pr.base_ref.clone();
    let head_ref_str = pr_with_author.pr.head_ref.clone();
    let pr_title = pr_with_author.pr.title.clone();

    if pr_state != "open" {
        return Err(AppError::BadRequest("Pull Request 已關閉或已合併".into()));
    }

    let owner_name = resolve_owner_by_repo(&state, &repo).await?;

    let head_repo = state.db.find_repo_by_id(pr_with_author.pr.head_repo_id).await?
        .ok_or_else(|| AppError::NotFound("來源倉庫不存在".into()))?;
    let head_owner = resolve_owner_by_repo(&state, &head_repo).await?;

    let base_path = state.config.repo_path(&owner_name, &repo.name);
    let head_path = state.config.repo_path(&head_owner, &head_repo.name);

    // Perform the merge (sync git2 operations in a scope so objects drop before awaits)
    let merge_sha = {
        let base_repo = git2::Repository::open_bare(&base_path)?;
        let head_repo_obj = git2::Repository::open_bare(&head_path)?;

        let base_ref = format!("refs/heads/{}", base_ref_str);
        let head_ref = format!("refs/heads/{}", head_ref_str);

        let base_commit = base_repo.find_commit(
            base_repo.refname_to_id(&base_ref)?
        )?;
        let head_commit = head_repo_obj.find_commit(
            head_repo_obj.refname_to_id(&head_ref)?
        )?;

        let base_tree = base_commit.tree()?;
        let head_tree = head_commit.tree()?;

        let merge_base_oid = base_repo.merge_base(
            base_commit.id(),
            head_commit.id(),
        )?;
        let ancestor_tree = base_repo.find_commit(merge_base_oid)?.tree()?;

        let merge_opts = git2::MergeOptions::new();
        let mut index = base_repo.merge_trees(&base_tree, &head_tree, &ancestor_tree, Some(&merge_opts))?;

        if index.has_conflicts() {
            return Err(AppError::Conflict("合併衝突，無法自動合併".into()));
        }

        let tree_oid = index.write_tree_to(&base_repo)?;
        let merged_tree = base_repo.find_tree(tree_oid)?;

        let sig = git2::Signature::now("gitpage", "gitpage@localhost")?;
        let merge_message = format!("Merge pull request #{} from {}/{}: {}", path.pr_number, head_owner, head_repo.name, pr_title);

        let merge_commit_oid = base_repo.commit(
            Some(&base_ref),
            &sig, &sig,
            &merge_message,
            &merged_tree,
            &[&base_commit, &head_commit],
        )?;

        merge_commit_oid.to_string()
    };

    state.db.set_pr_merge_sha(pr_id, &merge_sha).await?;
    state.db.update_pr(pr_id, path.repo_id, None, None, Some("merged")).await?;

    Ok(Json(json!({
        "merged": true,
        "merge_commit_sha": merge_sha
    })))
}

pub async fn get_pr_diff(
    State(state): State<AppState>,
    Path(path): Path<PrPath>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(path.repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    let pr_with_author = state.db.get_pr(path.repo_id, path.pr_number).await?
        .ok_or_else(|| AppError::NotFound("Pull Request 不存在".into()))?;
    let pr = pr_with_author.pr;

    let owner_name = resolve_owner_by_repo(&state, &repo).await?;

    let head_repo = state.db.find_repo_by_id(pr.head_repo_id).await?
        .ok_or_else(|| AppError::NotFound("來源倉庫不存在".into()))?;
    let head_owner = resolve_owner_by_repo(&state, &head_repo).await?;

    let base_path = state.config.repo_path(&owner_name, &repo.name);
    let head_path = state.config.repo_path(&head_owner, &head_repo.name);

    let base_repo = git2::Repository::open_bare(&base_path)?;
    let head_repo_obj = git2::Repository::open_bare(&head_path)?;

    let base_tree = base_repo.find_commit(
        base_repo.refname_to_id(&format!("refs/heads/{}", pr.base_ref))?
    )?.tree()?;

    let head_tree = head_repo_obj.find_commit(
        head_repo_obj.refname_to_id(&format!("refs/heads/{}", pr.head_ref))?
    )?.tree()?;

    let diff = base_repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;

    let mut entries = Vec::new();
    for delta in diff.deltas() {
        let status = match delta.status() {
            git2::Delta::Added => "added",
            git2::Delta::Deleted => "deleted",
            git2::Delta::Modified => "modified",
            git2::Delta::Renamed => "renamed",
            git2::Delta::Copied => "copied",
            _ => "unknown",
        };
        let old_path = delta.old_file().path().map(|p| p.to_string_lossy().to_string());
        let new_path = delta.new_file().path().map(|p| p.to_string_lossy().to_string());
        entries.push(DiffEntry {
            status: status.to_string(),
            old_path,
            new_path,
        });
    }

    Ok(Json(json!({ "diff": entries, "pull": pr })))
}

async fn resolve_owner_by_repo(state: &AppState, repo: &Repository) -> Result<String, AppError> {
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
