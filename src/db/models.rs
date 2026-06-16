use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub bio: String,
    pub avatar_url: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPublic {
    pub id: i64,
    pub username: String,
    pub bio: String,
    pub avatar_url: String,
    pub created_at: String,
}

impl From<User> for UserPublic {
    fn from(u: User) -> Self {
        UserPublic {
            id: u.id,
            username: u.username,
            bio: u.bio,
            avatar_url: u.avatar_url,
            created_at: u.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub description: String,
    pub is_private: bool,
    pub default_branch: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppsConfig {
    pub id: i64,
    pub repo_id: i64,
    pub branch: String,
    pub source_dir: String,
    pub build_command: String,
    pub start_command: String,
    pub env_vars: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagesConfig {
    pub id: i64,
    pub repo_id: i64,
    pub branch: String,
    pub source_dir: String,
    pub custom_domain: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateRepoRequest {
    pub name: String,
    pub description: Option<String>,
    pub is_private: Option<bool>,
    pub created_via: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: Option<i64>,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserPublic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKey {
    pub id: i64,
    pub user_id: i64,
    pub repo_id: i64,
    pub name: String,
    pub public_key: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub name: String,
    pub description: String,
    pub is_private: bool,
    pub default_branch: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployLog {
    pub id: i64,
    pub repo_id: i64,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub log_output: String,
}
