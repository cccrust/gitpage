use std::sync::OnceLock;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use chrono::{Duration, Utc};

use crate::db::models::UserPublic;

pub static JWT_SECRET: OnceLock<String> = OnceLock::new();
pub static ENCRYPTION_KEY: OnceLock<[u8; 32]> = OnceLock::new();

pub fn init_jwt_secret(secret: String) {
    JWT_SECRET.set(secret).ok();
}

pub fn init_encryption_key(key: &str) {
    if key.is_empty() {
        return;
    }
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(key.as_bytes());
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&hash);
    ENCRYPTION_KEY.set(arr).ok();
}

pub fn get_encryption_key() -> [u8; 32] {
    *ENCRYPTION_KEY.get().expect("ENCRYPTION_KEY not initialized")
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i64,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn create_token(user: &UserPublic, expires_in_hours: u64) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = JWT_SECRET.get().expect("JWT_SECRET not initialized");
    let now = Utc::now();
    let exp = now + Duration::hours(expires_in_hours as i64);
    let claims = Claims {
        sub: user.id,
        username: user.username.clone(),
        iat: now.timestamp() as usize,
        exp: exp.timestamp() as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
}

pub fn verify_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret = JWT_SECRET.get().expect("JWT_SECRET not initialized");
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}
