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
    pub owner_type: String,
    pub org_id: Option<i64>,
    pub forked_from: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub owner_id: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMember {
    pub id: i64,
    pub org_id: i64,
    pub user_id: i64,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationWithRole {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub owner_id: i64,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrgRepoResult {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub description: String,
    pub is_private: bool,
    pub default_branch: String,
    pub owner_type: String,
    pub org_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub org_name: Option<String>,
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
    pub port: i64,
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
    pub org_name: Option<String>,
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

#[derive(Debug, Clone)]
pub struct EnabledAppWithOwner {
    pub config: AppsConfig,
    pub username: String,
    pub repo_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: i64,
    pub repo_id: i64,
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub author_id: i64,
    pub assignee_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueWithAuthor {
    pub issue: Issue,
    pub author_username: String,
    pub labels: Vec<IssueLabel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLabel {
    pub id: i64,
    pub repo_id: i64,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueComment {
    pub id: i64,
    pub issue_id: i64,
    pub author_id: i64,
    pub author_username: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: i64,
    pub repo_id: i64,
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub author_id: i64,
    pub head_repo_id: i64,
    pub head_ref: String,
    pub base_ref: String,
    pub merge_commit_sha: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub merged_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestWithAuthor {
    pub pr: PullRequest,
    pub author_username: String,
    pub head_repo_name: String,
    pub head_repo_owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub status: String,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
}

// ── v2.1 Settings models ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub token_prefix: String,
    pub scopes: String,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoCollaborator {
    pub repo_id: i64,
    pub user_id: i64,
    pub permission: String,
    pub username: String,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoSecret {
    pub id: i64,
    pub repo_id: i64,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchProtection {
    pub id: i64,
    pub repo_id: i64,
    pub pattern: String,
    pub require_pr: bool,
    pub require_approvals: i64,
    pub dismiss_stale_reviews: bool,
}
