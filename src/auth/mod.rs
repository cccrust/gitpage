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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::UserPublic;

    fn setup() {
        init_jwt_secret("test-jwt-secret-for-unit-tests".to_string());
    }

    fn make_user(id: i64) -> UserPublic {
        UserPublic {
            id,
            username: format!("user{}", id),
            bio: "".to_string(),
            avatar_url: "".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_create_token_success() {
        setup();
        let user = make_user(42);
        let token = create_token(&user, 24).unwrap();
        assert!(!token.is_empty());
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_verify_token_valid() {
        setup();
        let user = make_user(42);
        let token = create_token(&user, 24).unwrap();
        let claims = verify_token(&token).unwrap();
        assert_eq!(claims.sub, 42);
        assert_eq!(claims.username, "user42");
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_verify_token_invalid_signature() {
        setup();
        let user = make_user(1);
        let token = create_token(&user, 24).unwrap();
        let parts: Vec<&str> = token.split('.').collect();
        let tampered = format!("{}.{}.invalidsig", parts[0], parts[1]);
        let result = verify_token(&tampered);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_garbage() {
        setup();
        let result = verify_token("not-a-jwt-token-at-all");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_empty() {
        setup();
        let result = verify_token("");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_users_have_distinct_tokens() {
        setup();
        let user1 = make_user(1);
        let user2 = make_user(2);
        let token1 = create_token(&user1, 24).unwrap();
        let token2 = create_token(&user2, 24).unwrap();
        assert_ne!(token1, token2);

        let c1 = verify_token(&token1).unwrap();
        let c2 = verify_token(&token2).unwrap();
        assert_eq!(c1.sub, 1);
        assert_eq!(c2.sub, 2);
    }

    #[test]
    fn test_claims_username_matches() {
        setup();
        let user = make_user(7);
        let token = create_token(&user, 24).unwrap();
        let claims = verify_token(&token).unwrap();
        assert_eq!(claims.username, "user7");
    }

    #[test]
    fn test_expires_in_hours_affects_exp() {
        setup();
        let user = make_user(1);
        let token_short = create_token(&user, 1).unwrap();
        let token_long = create_token(&user, 720).unwrap();
        let c_short = verify_token(&token_short).unwrap();
        let c_long = verify_token(&token_long).unwrap();
        let short_duration = c_short.exp - c_short.iat;
        let long_duration = c_long.exp - c_long.iat;
        assert!(long_duration > short_duration);
    }

    #[test]
    fn test_init_encryption_key_sets_global() {
        init_encryption_key("test-encryption-key-12345");
        let key = get_encryption_key();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_init_encryption_key_empty_does_nothing() {
        // Initially may not be set; calling with empty string is a no-op
        init_encryption_key("");
        // Should not panic even if not set — get_encryption_key will
        // only panic if never set, but init_encryption_key("") is no-op
        // so we just verify no crash during init
    }
}
