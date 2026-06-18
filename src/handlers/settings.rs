use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use rand::Rng;
use sha2::{Sha256, Digest};
use aes_gcm::{Aes256Gcm, Key, Nonce, AeadInPlace, AeadCore, KeyInit};
use rand::rngs::OsRng;

use crate::app::AppState;
use crate::auth;
use crate::utils::errors::AppError;

// ── Access Tokens ──

pub async fn list_tokens(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
) -> Result<Json<Value>, AppError> {
    let tokens = state.db.list_access_tokens(user_id).await?;
    Ok(Json(json!({ "tokens": tokens })))
}

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub scopes: Option<String>,
    pub expires_at: Option<String>,
}

pub async fn create_token(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Json(req): Json<CreateTokenRequest>,
) -> Result<Json<Value>, AppError> {
    let raw: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(40)
        .map(char::from)
        .collect();
    let token = format!("gpt_{}", raw);
    let prefix = token[..12].to_string();

    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let hash = hex::encode(hasher.finalize());

    let created = state.db.create_access_token(
        user_id, &req.name, &hash, &prefix,
        req.scopes.as_deref().unwrap_or("repo"),
        req.expires_at.as_deref(),
    ).await?;

    Ok(Json(json!({
        "token": created,
        "raw_token": token
    })))
}

#[derive(Deserialize)]
pub struct DeleteTokenPath {
    pub token_id: i64,
}

pub async fn delete_token(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(path): Path<DeleteTokenPath>,
) -> Result<Json<Value>, AppError> {
    let deleted = state.db.delete_access_token(path.token_id, user_id).await?;
    if !deleted {
        return Err(AppError::NotFound("Token 不存在".into()));
    }
    Ok(Json(json!({ "success": true })))
}

// ── Collaborators ──

#[derive(Deserialize)]
pub struct AddCollaboratorRequest {
    pub username: String,
    pub permission: Option<String>,
}

pub async fn add_collaborator(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
    Json(req): Json<AddCollaboratorRequest>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    if repo.user_id != user_id {
        return Err(AppError::Unauthorized("只有倉庫擁有者可以管理協作者".into()));
    }

    let target = state.db.find_user_by_username(&req.username).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;

    let perm = req.permission.as_deref().unwrap_or("write");
    state.db.add_collaborator(repo_id, target.id, perm).await?;

    Ok(Json(json!({ "success": true })))
}

pub async fn list_collaborators(
    State(state): State<AppState>,
    axum::Extension(_user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let collabs = state.db.list_collaborators(repo_id).await?;
    Ok(Json(json!({ "collaborators": collabs })))
}

#[derive(Deserialize)]
pub struct RemoveCollaboratorPath {
    pub repo_id: i64,
    pub user_id: i64,
}

pub async fn remove_collaborator(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(path): Path<RemoveCollaboratorPath>,
) -> Result<Json<Value>, AppError> {
    let repo = state.db.find_repo_by_id(path.repo_id).await?
        .ok_or_else(|| AppError::NotFound("倉庫不存在".into()))?;

    if repo.user_id != user_id {
        return Err(AppError::Unauthorized("只有倉庫擁有者可以管理協作者".into()));
    }

    let deleted = state.db.remove_collaborator(path.repo_id, path.user_id).await?;
    if !deleted {
        return Err(AppError::NotFound("協作者不存在".into()));
    }
    Ok(Json(json!({ "success": true })))
}

// ── Secrets ──

#[derive(Deserialize)]
pub struct CreateSecretRequest {
    pub name: String,
    pub value: String,
}

pub async fn create_secret(
    State(state): State<AppState>,
    axum::Extension(_user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
    Json(req): Json<CreateSecretRequest>,
) -> Result<Json<Value>, AppError> {
    let encrypted = encrypt_secret(req.value.as_bytes())?;
    let secret = state.db.create_secret(repo_id, &req.name, &encrypted).await?;
    Ok(Json(json!({ "secret": secret })))
}

pub async fn list_secrets(
    State(state): State<AppState>,
    axum::Extension(_user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let secrets = state.db.list_secrets(repo_id).await?;
    Ok(Json(json!({ "secrets": secrets })))
}

#[derive(Deserialize)]
pub struct SecretIdPath {
    pub repo_id: i64,
    pub secret_id: i64,
}

pub async fn delete_secret(
    State(state): State<AppState>,
    axum::Extension(_user_id): axum::Extension<i64>,
    Path(path): Path<SecretIdPath>,
) -> Result<Json<Value>, AppError> {
    let deleted = state.db.delete_secret(path.secret_id, path.repo_id).await?;
    if !deleted {
        return Err(AppError::NotFound("Secret 不存在".into()));
    }
    Ok(Json(json!({ "success": true })))
}

// ── Branch Protection ──

#[derive(Deserialize)]
pub struct CreateBranchProtectionRequest {
    pub pattern: String,
    pub require_pr: Option<bool>,
    pub require_approvals: Option<i64>,
    pub dismiss_stale_reviews: Option<bool>,
}

pub async fn create_branch_protection(
    State(state): State<AppState>,
    axum::Extension(_user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
    Json(req): Json<CreateBranchProtectionRequest>,
) -> Result<Json<Value>, AppError> {
    let bp = state.db.create_branch_protection(
        repo_id,
        &req.pattern,
        req.require_pr.unwrap_or(true),
        req.require_approvals.unwrap_or(1),
        req.dismiss_stale_reviews.unwrap_or(true),
    ).await?;
    Ok(Json(json!({ "branch_protection": bp })))
}

pub async fn list_branch_protections(
    State(state): State<AppState>,
    axum::Extension(_user_id): axum::Extension<i64>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let protections = state.db.list_branch_protections(repo_id).await?;
    Ok(Json(json!({ "branch_protections": protections })))
}

#[derive(Deserialize)]
pub struct BranchProtectionIdPath {
    pub repo_id: i64,
    pub protection_id: i64,
}

pub async fn delete_branch_protection(
    State(state): State<AppState>,
    axum::Extension(_user_id): axum::Extension<i64>,
    Path(path): Path<BranchProtectionIdPath>,
) -> Result<Json<Value>, AppError> {
    let deleted = state.db.delete_branch_protection(path.protection_id, path.repo_id).await?;
    if !deleted {
        return Err(AppError::NotFound("Branch protection 不存在".into()));
    }
    Ok(Json(json!({ "success": true })))
}

// ── Encryption helpers ──

fn encrypt_secret(plaintext: &[u8]) -> Result<Vec<u8>, AppError> {
    let key_bytes = auth::get_encryption_key();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let mut buf = plaintext.to_vec();
    cipher.encrypt_in_place(&nonce, &[], &mut buf)
        .map_err(|e| AppError::Internal(format!("加密失敗: {}", e)))?;
    let mut result = nonce.to_vec();
    result.extend_from_slice(&buf);
    Ok(result)
}

pub fn decrypt_secret(data: &[u8]) -> Result<Vec<u8>, AppError> {
    if data.len() < 12 {
        return Err(AppError::Internal("無效的密文".into()));
    }
    let key_bytes = auth::get_encryption_key();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let mut buf = ciphertext.to_vec();
    cipher.decrypt_in_place(nonce, &[], &mut buf)
        .map_err(|e| AppError::Internal(format!("解密失敗: {}", e)))?;
    Ok(buf)
}
