use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use rand::rngs::OsRng;

use crate::app::AppState;
use crate::auth::create_token;
use crate::db::models::*;
use crate::utils::errors::AppError;

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    if req.username.len() < 3 {
        return Err(AppError::BadRequest("使用者名稱至少需要 3 個字元".into()));
    }
    if req.password.len() < 6 {
        return Err(AppError::BadRequest("密碼至少需要 6 個字元".into()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Hash error: {}", e)))?
        .to_string();

    let user = state.db.create_user(&req.username, &req.email, &password_hash).await
        .map_err(|e| {
            if let rusqlite::Error::SqliteFailure(_, Some(msg)) = &e {
                if msg.contains("UNIQUE") {
                    return AppError::Conflict("使用者名稱或 Email 已存在".into());
                }
            }
            AppError::Internal(format!("DB error: {}", e))
        })?;

    let user_public: UserPublic = user.into();
    let token = create_token(&user_public, state.jwt_expires_hours)
        .map_err(|e| AppError::Internal(format!("JWT error: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "token": token,
            "user": user_public
        })),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<Value>, AppError> {
    let user = state.db.find_user_by_username(&req.username).await?
        .ok_or_else(|| AppError::Unauthorized("使用者名稱或密碼錯誤".into()))?;

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|e| AppError::Internal(format!("Parse hash error: {}", e)))?;

    let argon2 = Argon2::default();
    argon2.verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Unauthorized("使用者名稱或密碼錯誤".into()))?;

    let user_public: UserPublic = user.into();
    let token = create_token(&user_public, state.jwt_expires_hours)
        .map_err(|e| AppError::Internal(format!("JWT error: {}", e)))?;

    Ok(Json(json!({
        "token": token,
        "user": user_public
    })))
}

pub async fn me(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
) -> Result<Json<Value>, AppError> {
    let user = state.db.find_user_by_id(user_id).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;
    let user_public: UserPublic = user.into();
    Ok(Json(json!({ "user": user_public })))
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

pub async fn change_password(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<Value>, AppError> {
    if req.new_password.len() < 6 {
        return Err(AppError::BadRequest("新密碼至少需要 6 個字元".into()));
    }

    let user = state.db.find_user_by_id(user_id).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|e| AppError::Internal(format!("解析雜湊錯誤: {}", e)))?;
    let argon2 = Argon2::default();
    argon2.verify_password(req.current_password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Unauthorized("目前密碼錯誤".into()))?;

    let salt = SaltString::generate(&mut OsRng);
    let new_hash = argon2
        .hash_password(req.new_password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Hash error: {}", e)))?
        .to_string();

    state.db.change_password(user_id, &new_hash).await?;

    Ok(Json(json!({ "success": true })))
}

pub async fn get_user_profile(
    State(state): State<AppState>,
    Path(username): Path<String>,
    _user_id: Option<axum::Extension<i64>>,
) -> Result<Json<Value>, AppError> {
    let user = state.db.find_user_by_username(&username).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;
    let uid = user.id;
    let user_public: UserPublic = user.into();

    let repos = state.db.list_public_user_repos(uid).await?;

    Ok(Json(json!({
        "user": user_public,
        "repos": repos
    })))
}

pub async fn update_profile(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Path(username): Path<String>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<Value>, AppError> {
    let user = state.db.find_user_by_id(user_id).await?
        .ok_or_else(|| AppError::NotFound("使用者不存在".into()))?;

    if user.username != username {
        return Err(AppError::Unauthorized("不能修改其他使用者的資料".into()));
    }

    let bio = req.bio.unwrap_or(user.bio);
    let avatar_url = req.avatar_url.unwrap_or(user.avatar_url);

    state.db.update_user(user_id, &bio, &avatar_url).await?;

    Ok(Json(json!({ "success": true })))
}
