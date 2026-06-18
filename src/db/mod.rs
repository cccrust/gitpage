pub mod models;

use rusqlite::{Connection, params};
use std::sync::Arc;
use tokio::sync::Mutex;
use models::{User, UserPublic, Repository, PagesConfig, AppsConfig, DeployLog, SshKey, SearchResultItem, Organization, OrganizationMember, OrganizationWithRole, OrgRepoResult, EnabledAppWithOwner, Issue, IssueWithAuthor, IssueLabel, IssueComment, PullRequest, PullRequestWithAuthor, AccessToken, RepoCollaborator, RepoSecret, BranchProtection, Star, Watch};

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(Database {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn run_migrations(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS organizations (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT NOT NULL UNIQUE,
                display_name TEXT DEFAULT '',
                description TEXT DEFAULT '',
                owner_id    INTEGER NOT NULL REFERENCES users(id),
                created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS organization_members (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                org_id      INTEGER NOT NULL REFERENCES organizations(id),
                user_id     INTEGER NOT NULL REFERENCES users(id),
                role        TEXT NOT NULL DEFAULT 'member',
                created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(org_id, user_id)
            );

            CREATE TABLE IF NOT EXISTS users (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                username    TEXT NOT NULL UNIQUE,
                email       TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                bio         TEXT DEFAULT '',
                avatar_url  TEXT DEFAULT '',
                created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS repositories (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id         INTEGER NOT NULL REFERENCES users(id),
                name            TEXT NOT NULL,
                description     TEXT DEFAULT '',
                is_private      INTEGER DEFAULT 0,
                default_branch  TEXT DEFAULT 'main',
                owner_type      TEXT NOT NULL DEFAULT 'user',
                org_id          INTEGER REFERENCES organizations(id),
                created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS pages_config (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id     INTEGER NOT NULL UNIQUE REFERENCES repositories(id),
                branch      TEXT DEFAULT 'main',
                source_dir  TEXT DEFAULT '/',
                custom_domain TEXT DEFAULT '',
                enabled     INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS apps_config (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id         INTEGER NOT NULL UNIQUE REFERENCES repositories(id),
                branch          TEXT DEFAULT 'main',
                source_dir      TEXT DEFAULT '/',
                build_command   TEXT DEFAULT '',
                start_command   TEXT DEFAULT '',
                env_vars        TEXT DEFAULT '{}',
                enabled         INTEGER DEFAULT 0,
                port            INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS deploy_logs (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id     INTEGER NOT NULL REFERENCES repositories(id),
                status      TEXT NOT NULL DEFAULT 'running',
                started_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
                finished_at DATETIME,
                log_output  TEXT DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS ssh_keys (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id     INTEGER NOT NULL REFERENCES users(id),
                repo_id     INTEGER NOT NULL REFERENCES repositories(id),
                name        TEXT DEFAULT '',
                public_key  TEXT NOT NULL,
                created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
            );"
        )?;

        // Add owner_type and org_id columns if they don't exist
        let _ = conn.execute("ALTER TABLE repositories ADD COLUMN owner_type TEXT NOT NULL DEFAULT 'user'", []);
        let _ = conn.execute("ALTER TABLE repositories ADD COLUMN org_id INTEGER REFERENCES organizations(id)", []);

        // Migrate away from old UNIQUE(user_id, name) constraint — replace with partial indexes
        let has_old_constraint: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='repositories' AND sql LIKE '%UNIQUE(user_id, name)%'",
            [],
            |r| r.get(0),
        ).unwrap_or(false);

        if has_old_constraint {
            // Temporarily disable FK checks — pages_config/apps_config
            // reference repositories(id) and would block DROP TABLE
            conn.execute_batch("PRAGMA foreign_keys=OFF;")?;
            let migration = conn.execute_batch(
                "DROP TABLE IF EXISTS repositories_migrated;

                CREATE TABLE repositories_migrated (
                    id              INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id         INTEGER NOT NULL REFERENCES users(id),
                    name            TEXT NOT NULL,
                    description     TEXT DEFAULT '',
                    is_private      INTEGER DEFAULT 0,
                    default_branch  TEXT DEFAULT 'main',
                    owner_type      TEXT NOT NULL DEFAULT 'user',
                    org_id          INTEGER REFERENCES organizations(id),
                    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP
                );

                INSERT INTO repositories_migrated (id, user_id, name, description, is_private, default_branch, owner_type, org_id, created_at, updated_at)
                SELECT id, user_id, name, description, is_private, default_branch, COALESCE(owner_type, 'user'), org_id, created_at, updated_at FROM repositories;

                DROP TABLE repositories;
                ALTER TABLE repositories_migrated RENAME TO repositories;"
            );
            conn.execute_batch("PRAGMA foreign_keys=ON;")?;
            migration?;
        }

        // Partial unique indexes for user and org repo names
        conn.execute_batch(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_repos_user_name ON repositories(user_id, name) WHERE owner_type = 'user';
             CREATE UNIQUE INDEX IF NOT EXISTS idx_repos_org_name ON repositories(org_id, name) WHERE org_id IS NOT NULL;"
        )?;

        // Migration: add port column to apps_config (v1.2)
        conn.execute_batch(
            "ALTER TABLE apps_config ADD COLUMN port INTEGER DEFAULT 0;"
        ).ok();

        // v2.0 tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS issues (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id     INTEGER NOT NULL REFERENCES repositories(id),
                number      INTEGER NOT NULL,
                title       TEXT NOT NULL,
                body        TEXT DEFAULT '',
                state       TEXT NOT NULL DEFAULT 'open',
                author_id   INTEGER NOT NULL REFERENCES users(id),
                assignee_id INTEGER REFERENCES users(id),
                created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
                closed_at   DATETIME,
                UNIQUE(repo_id, number)
            );

            CREATE TABLE IF NOT EXISTS issue_labels (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id     INTEGER NOT NULL REFERENCES repositories(id),
                name        TEXT NOT NULL,
                color       TEXT NOT NULL DEFAULT '0366d6',
                UNIQUE(repo_id, name)
            );

            CREATE TABLE IF NOT EXISTS issue_label_map (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                issue_id    INTEGER NOT NULL REFERENCES issues(id),
                label_id    INTEGER NOT NULL REFERENCES issue_labels(id),
                UNIQUE(issue_id, label_id)
            );

            CREATE TABLE IF NOT EXISTS issue_comments (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                issue_id    INTEGER NOT NULL REFERENCES issues(id),
                author_id   INTEGER NOT NULL REFERENCES users(id),
                body        TEXT NOT NULL,
                created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS pull_requests (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id         INTEGER NOT NULL REFERENCES repositories(id),
                number          INTEGER NOT NULL,
                title           TEXT NOT NULL,
                body            TEXT DEFAULT '',
                state           TEXT NOT NULL DEFAULT 'open',
                author_id       INTEGER NOT NULL REFERENCES users(id),
                head_repo_id    INTEGER NOT NULL REFERENCES repositories(id),
                head_ref        TEXT NOT NULL,
                base_ref        TEXT NOT NULL,
                merge_commit_sha TEXT,
                created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
                closed_at       DATETIME,
                merged_at       DATETIME,
                UNIQUE(repo_id, number)
            );"
        )?;

        // Add forked_from column to repositories
        conn.execute_batch(
            "ALTER TABLE repositories ADD COLUMN forked_from INTEGER REFERENCES repositories(id);"
        ).ok();

        // Add count columns to repositories (v2.2)
        conn.execute_batch(
            "ALTER TABLE repositories ADD COLUMN stars_count INTEGER NOT NULL DEFAULT 0;"
        ).ok();
        conn.execute_batch(
            "ALTER TABLE repositories ADD COLUMN forks_count INTEGER NOT NULL DEFAULT 0;"
        ).ok();
        conn.execute_batch(
            "ALTER TABLE repositories ADD COLUMN watch_count INTEGER NOT NULL DEFAULT 0;"
        ).ok();

        // v2.2 tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS stars (
                user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                repo_id    INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (user_id, repo_id)
            );

            CREATE TABLE IF NOT EXISTS watches (
                user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                repo_id    INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
                watch_type TEXT NOT NULL DEFAULT 'participating',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (user_id, repo_id)
            );"
        )?;

        // v2.1 tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS access_tokens (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                name        TEXT NOT NULL,
                token_prefix TEXT NOT NULL DEFAULT '',
                token_hash  TEXT NOT NULL,
                scopes      TEXT NOT NULL DEFAULT 'repo',
                last_used_at DATETIME,
                created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
                expires_at  DATETIME
            );

            CREATE TABLE IF NOT EXISTS repo_collaborators (
                repo_id     INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
                user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                permission  TEXT NOT NULL DEFAULT 'write',
                PRIMARY KEY (repo_id, user_id)
            );

            CREATE TABLE IF NOT EXISTS repo_secrets (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id         INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
                name            TEXT NOT NULL,
                encrypted_value BLOB NOT NULL,
                created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(repo_id, name)
            );

            CREATE TABLE IF NOT EXISTS branch_protection (
                id                      INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id                 INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
                pattern                 TEXT NOT NULL,
                require_pr              INTEGER NOT NULL DEFAULT 1,
                require_approvals       INTEGER NOT NULL DEFAULT 1,
                dismiss_stale_reviews   INTEGER NOT NULL DEFAULT 1,
                UNIQUE(repo_id, pattern)
            );"
        )?;

        Ok(())
    }

    // ── User operations ──

    pub async fn create_user(&self, username: &str, email: &str, password_hash: &str) -> Result<User, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO users (username, email, password_hash) VALUES (?1, ?2, ?3)",
            params![username, email, password_hash],
        )?;
        let id = conn.last_insert_rowid();
        Ok(User {
            id,
            username: username.to_string(),
            email: email.to_string(),
            password_hash: password_hash.to_string(),
            bio: String::new(),
            avatar_url: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, username, email, password_hash, bio, avatar_url, created_at FROM users WHERE username = ?1"
        )?;
        let mut rows = stmt.query_map(params![username], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                email: row.get(2)?,
                password_hash: row.get(3)?,
                bio: row.get(4)?,
                avatar_url: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(Ok(user)) => Ok(Some(user)),
            _ => Ok(None),
        }
    }

    pub async fn find_user_by_id(&self, id: i64) -> Result<Option<User>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, username, email, password_hash, bio, avatar_url, created_at FROM users WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                email: row.get(2)?,
                password_hash: row.get(3)?,
                bio: row.get(4)?,
                avatar_url: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(Ok(user)) => Ok(Some(user)),
            _ => Ok(None),
        }
    }

    // ── Repository operations ──

    pub async fn create_repo(&self, user_id: i64, name: &str, description: &str, is_private: bool, owner_type: &str, org_id: Option<i64>) -> Result<Repository, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO repositories (user_id, name, description, is_private, owner_type, org_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![user_id, name, description, is_private as i32, owner_type, org_id],
        )?;
        let id = conn.last_insert_rowid();
        let now = chrono::Utc::now().to_rfc3339();
        Ok(Repository {
            id,
            user_id,
            name: name.to_string(),
            description: description.to_string(),
            is_private,
            default_branch: "main".to_string(),
            owner_type: owner_type.to_string(),
            org_id,
            forked_from: None,
            stars_count: 0,
            forks_count: 0,
            watch_count: 0,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn list_user_repos(&self, user_id: i64) -> Result<Vec<Repository>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, description, is_private, default_branch, owner_type, org_id, created_at, updated_at, forked_from, stars_count, forks_count, watch_count
             FROM repositories WHERE user_id = ?1 AND owner_type = 'user' ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map(params![user_id], map_repo_row)?;
        rows.collect()
    }

    pub async fn list_org_repos(&self, org_id: i64) -> Result<Vec<Repository>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, description, is_private, default_branch, owner_type, org_id, created_at, updated_at, forked_from, stars_count, forks_count, watch_count
             FROM repositories WHERE org_id = ?1 AND owner_type = 'org' ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map(params![org_id], map_repo_row)?;
        rows.collect()
    }

    pub async fn list_org_repos_with_orgname(&self, org_id: i64) -> Result<Vec<OrgRepoResult>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT r.id, r.user_id, r.name, r.description, r.is_private, r.default_branch, r.owner_type, r.org_id, r.created_at, r.updated_at, r.forked_from, r.stars_count, r.forks_count, r.watch_count, o.name as org_name
             FROM repositories r JOIN organizations o ON o.id = r.org_id
             WHERE r.org_id = ?1 AND r.owner_type = 'org' ORDER BY r.updated_at DESC"
        )?;
        let rows = stmt.query_map(params![org_id], |row| {
            Ok(OrgRepoResult {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                is_private: row.get::<_, i32>(4)? != 0,
                default_branch: row.get(5)?,
                owner_type: row.get(6)?,
                org_id: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                org_name: row.get(14)?,
            })
        })?;
        rows.collect()
    }

    pub async fn list_public_user_repos(&self, user_id: i64) -> Result<Vec<Repository>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, description, is_private, default_branch, owner_type, org_id, created_at, updated_at, forked_from, stars_count, forks_count, watch_count
             FROM repositories WHERE user_id = ?1 AND owner_type = 'user' AND is_private = 0 ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map(params![user_id], map_repo_row)?;
        rows.collect()
    }

    pub async fn find_repo_by_name(&self, user_id: i64, name: &str) -> Result<Option<Repository>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, description, is_private, default_branch, owner_type, org_id, created_at, updated_at, forked_from, stars_count, forks_count, watch_count
             FROM repositories WHERE user_id = ?1 AND name = ?2 AND owner_type = 'user'"
        )?;
        let mut rows = stmt.query_map(params![user_id, name], map_repo_row)?;
        match rows.next() {
            Some(Ok(repo)) => Ok(Some(repo)),
            _ => Ok(None),
        }
    }

    pub async fn find_org_repo_by_name(&self, org_id: i64, name: &str) -> Result<Option<Repository>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, description, is_private, default_branch, owner_type, org_id, created_at, updated_at, forked_from, stars_count, forks_count, watch_count
             FROM repositories WHERE org_id = ?1 AND name = ?2 AND owner_type = 'org'"
        )?;
        let mut rows = stmt.query_map(params![org_id, name], map_repo_row)?;
        match rows.next() {
            Some(Ok(repo)) => Ok(Some(repo)),
            _ => Ok(None),
        }
    }

    pub async fn find_repo_by_id(&self, id: i64) -> Result<Option<Repository>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, description, is_private, default_branch, owner_type, org_id, created_at, updated_at, forked_from, stars_count, forks_count, watch_count
             FROM repositories WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], map_repo_row)?;
        match rows.next() {
            Some(Ok(repo)) => Ok(Some(repo)),
            _ => Ok(None),
        }
    }

    pub async fn delete_repo(&self, id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM pages_config WHERE repo_id = ?1", params![id])?;
        conn.execute("DELETE FROM apps_config WHERE repo_id = ?1", params![id])?;
        let affected = conn.execute("DELETE FROM repositories WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    // ── User update ──

    pub async fn change_password(&self, id: i64, password_hash: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE users SET password_hash = ?1 WHERE id = ?2",
            params![password_hash, id],
        )?;
        Ok(())
    }

    pub async fn update_user(&self, id: i64, bio: &str, avatar_url: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE users SET bio = ?1, avatar_url = ?2 WHERE id = ?3",
            params![bio, avatar_url, id],
        )?;
        Ok(())
    }

    pub async fn search_repos(&self, query: &str, page: i64, page_size: i64) -> Result<(Vec<SearchResultItem>, i64), rusqlite::Error> {
        let conn = self.conn.lock().await;
        let pattern = format!("%{}%", query);
        let offset = (page - 1) * page_size;

        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM repositories WHERE (name LIKE ?1 OR description LIKE ?1) AND is_private = 0",
            params![pattern],
            |row| row.get(0),
        )?;

        let mut stmt = conn.prepare(
            "SELECT r.id, r.user_id, u.username, r.name, r.description, r.is_private, r.default_branch, r.created_at, r.updated_at
             FROM repositories r JOIN users u ON r.user_id = u.id
             WHERE (r.name LIKE ?1 OR r.description LIKE ?1) AND r.is_private = 0
             ORDER BY r.updated_at DESC LIMIT ?2 OFFSET ?3"
        )?;
        let rows = stmt.query_map(params![pattern, page_size, offset], |row| {
            Ok(SearchResultItem {
                id: row.get(0)?,
                user_id: row.get(1)?,
                username: row.get(2)?,
                name: row.get(3)?,
                description: row.get(4)?,
                is_private: row.get(5)?,
                default_branch: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        let items: Vec<SearchResultItem> = rows.collect::<Result<_, _>>()?;
        Ok((items, total))
    }

    pub async fn update_repo(&self, id: i64, name: &str, description: &str, is_private: bool) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE repositories SET name = ?1, description = ?2, is_private = ?3, updated_at = CURRENT_TIMESTAMP WHERE id = ?4",
            params![name, description, is_private as i32, id],
        )?;
        Ok(())
    }

    // ── Pages operations ──

    pub async fn upsert_pages_config(&self, repo_id: i64, branch: &str, source_dir: &str, custom_domain: &str, enabled: bool) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO pages_config (repo_id, branch, source_dir, custom_domain, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(repo_id) DO UPDATE SET branch=excluded.branch, source_dir=excluded.source_dir,
               custom_domain=excluded.custom_domain, enabled=excluded.enabled",
            params![repo_id, branch, source_dir, custom_domain, enabled as i32],
        )?;
        Ok(())
    }

    // ── Apps operations ──

    pub async fn upsert_apps_config(&self, repo_id: i64, branch: &str, source_dir: &str, build_command: &str, start_command: &str, env_vars: &str, enabled: bool) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO apps_config (repo_id, branch, source_dir, build_command, start_command, env_vars, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(repo_id) DO UPDATE SET
               branch=excluded.branch, source_dir=excluded.source_dir,
               build_command=excluded.build_command, start_command=excluded.start_command,
               env_vars=excluded.env_vars, enabled=excluded.enabled",
            params![repo_id, branch, source_dir, build_command, start_command, env_vars, enabled as i32],
        )?;
        Ok(())
    }

    pub async fn set_app_port(&self, repo_id: i64, port: u16) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE apps_config SET port = ?1 WHERE repo_id = ?2",
            params![port as i32, repo_id],
        )?;
        Ok(())
    }

    pub async fn get_apps_config(&self, repo_id: i64) -> Result<Option<AppsConfig>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, repo_id, branch, source_dir, build_command, start_command, env_vars, enabled, port FROM apps_config WHERE repo_id = ?1"
        )?;
        let mut rows = stmt.query_map(params![repo_id], |row| {
            Ok(AppsConfig {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                branch: row.get(2)?,
                source_dir: row.get(3)?,
                build_command: row.get(4)?,
                start_command: row.get(5)?,
                env_vars: row.get(6)?,
                enabled: row.get::<_, i32>(7)? != 0,
                port: row.get(8)?,
            })
        })?;
        match rows.next() {
            Some(Ok(cfg)) => Ok(Some(cfg)),
            _ => Ok(None),
        }
    }

    pub async fn delete_apps_config(&self, repo_id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM apps_config WHERE repo_id = ?1", params![repo_id])?;
        Ok(())
    }

    pub async fn get_enabled_apps(&self) -> Result<Vec<AppsConfig>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, repo_id, branch, source_dir, build_command, start_command, env_vars, enabled, port FROM apps_config WHERE enabled = 1"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(AppsConfig {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                branch: row.get(2)?,
                source_dir: row.get(3)?,
                build_command: row.get(4)?,
                start_command: row.get(5)?,
                env_vars: row.get(6)?,
                enabled: row.get::<_, i32>(7)? != 0,
                port: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub async fn get_enabled_apps_with_owner(&self) -> Result<Vec<EnabledAppWithOwner>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT a.id, a.repo_id, a.branch, a.source_dir, a.build_command, a.start_command, a.env_vars, a.enabled, a.port,
                    COALESCE(u.username, o.name) AS owner_name, r.name AS repo_name
             FROM apps_config a
             JOIN repositories r ON r.id = a.repo_id
             LEFT JOIN users u ON u.id = r.user_id AND r.owner_type = 'user'
             LEFT JOIN organizations o ON o.id = r.org_id AND r.owner_type = 'org'
             WHERE a.enabled = 1"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(EnabledAppWithOwner {
                config: AppsConfig {
                    id: row.get(0)?,
                    repo_id: row.get(1)?,
                    branch: row.get(2)?,
                    source_dir: row.get(3)?,
                    build_command: row.get(4)?,
                    start_command: row.get(5)?,
                    env_vars: row.get(6)?,
                    enabled: row.get::<_, i32>(7)? != 0,
                    port: row.get(8)?,
                },
                username: row.get(9)?,
                repo_name: row.get(10)?,
            })
        })?;
        rows.collect()
    }

    // ── SSH key operations ──

    pub async fn create_ssh_key(&self, user_id: i64, repo_id: i64, name: &str, public_key: &str) -> Result<SshKey, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO ssh_keys (user_id, repo_id, name, public_key) VALUES (?1, ?2, ?3, ?4)",
            params![user_id, repo_id, name, public_key],
        )?;
        let id = conn.last_insert_rowid();
        Ok(SshKey {
            id,
            user_id,
            repo_id,
            name: name.to_string(),
            public_key: public_key.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub async fn list_ssh_keys(&self, repo_id: i64) -> Result<Vec<SshKey>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, repo_id, name, public_key, created_at FROM ssh_keys WHERE repo_id = ?1 ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok(SshKey {
                id: row.get(0)?,
                user_id: row.get(1)?,
                repo_id: row.get(2)?,
                name: row.get(3)?,
                public_key: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    pub async fn delete_ssh_key(&self, id: i64, user_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let affected = conn.execute(
            "DELETE FROM ssh_keys WHERE id = ?1 AND user_id = ?2",
            params![id, user_id],
        )?;
        Ok(affected > 0)
    }

    pub async fn get_all_ssh_keys(&self) -> Result<Vec<(SshKey, User, Repository)>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT k.id, k.user_id, k.repo_id, k.name, k.public_key, k.created_at,
                    u.id, u.username, u.email, u.password_hash, u.bio, u.avatar_url, u.created_at,
                    r.id, r.user_id, r.name, r.description, r.is_private, r.default_branch, r.owner_type, r.org_id, r.created_at, r.updated_at, r.forked_from, r.stars_count, r.forks_count, r.watch_count
             FROM ssh_keys k
             JOIN users u ON u.id = k.user_id
             JOIN repositories r ON r.id = k.repo_id"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                SshKey {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    repo_id: row.get(2)?,
                    name: row.get(3)?,
                    public_key: row.get(4)?,
                    created_at: row.get(5)?,
                },
                User {
                    id: row.get(6)?,
                    username: row.get(7)?,
                    email: row.get(8)?,
                    password_hash: row.get(9)?,
                    bio: row.get(10)?,
                    avatar_url: row.get(11)?,
                    created_at: row.get(12)?,
                },
                Repository {
                    id: row.get(13)?,
                    user_id: row.get(14)?,
                    name: row.get(15)?,
                    description: row.get(16)?,
                    is_private: row.get(17)?,
                    default_branch: row.get(18)?,
                    owner_type: row.get(19)?,
                    org_id: row.get(20)?,
                    created_at: row.get(21)?,
                    updated_at: row.get(22)?,
                    forked_from: row.get(23)?,
                    stars_count: row.get(24)?,
                    forks_count: row.get(25)?,
                    watch_count: row.get(26)?,
                },
            ))
        })?;
        rows.collect()
    }

    // ── Deploy log operations ──

    pub async fn create_deploy_log(&self, repo_id: i64) -> Result<DeployLog, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO deploy_logs (repo_id, status) VALUES (?1, 'running')",
            params![repo_id],
        )?;
        let id = conn.last_insert_rowid();
        Ok(DeployLog {
            id,
            repo_id,
            status: "running".to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
            log_output: String::new(),
        })
    }

    pub async fn update_deploy_log(&self, id: i64, status: &str, log_output: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE deploy_logs SET status = ?1, log_output = ?2, finished_at = CURRENT_TIMESTAMP WHERE id = ?3",
            params![status, log_output, id],
        )?;
        Ok(())
    }

    pub async fn append_deploy_log(&self, id: i64, log_output: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE deploy_logs SET log_output = log_output || ?1 WHERE id = ?2",
            params![log_output, id],
        )?;
        Ok(())
    }

    pub async fn get_deploy_logs(&self, repo_id: i64) -> Result<Vec<DeployLog>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, repo_id, status, started_at, finished_at, log_output
             FROM deploy_logs WHERE repo_id = ?1 ORDER BY started_at DESC LIMIT 20"
        )?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok(DeployLog {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                status: row.get(2)?,
                started_at: row.get(3)?,
                finished_at: row.get(4)?,
                log_output: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    pub async fn get_deploy_log(&self, id: i64) -> Result<Option<DeployLog>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, repo_id, status, started_at, finished_at, log_output
             FROM deploy_logs WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(DeployLog {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                status: row.get(2)?,
                started_at: row.get(3)?,
                finished_at: row.get(4)?,
                log_output: row.get(5)?,
            })
        })?;
        match rows.next() {
            Some(Ok(log)) => Ok(Some(log)),
            _ => Ok(None),
        }
    }

    pub async fn get_pages_config(&self, repo_id: i64) -> Result<Option<PagesConfig>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, repo_id, branch, source_dir, custom_domain, enabled FROM pages_config WHERE repo_id = ?1"
        )?;
        let mut rows = stmt.query_map(params![repo_id], |row| {
            Ok(PagesConfig {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                branch: row.get(2)?,
                source_dir: row.get(3)?,
                custom_domain: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
            })
        })?;
        match rows.next() {
            Some(Ok(cfg)) => Ok(Some(cfg)),
            _ => Ok(None),
        }
    }

    // ── Organization operations ──

    pub async fn create_org(&self, name: &str, display_name: &str, description: &str, owner_id: i64) -> Result<Organization, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO organizations (name, display_name, description, owner_id) VALUES (?1, ?2, ?3, ?4)",
            params![name, display_name, description, owner_id],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Organization {
            id,
            name: name.to_string(),
            display_name: display_name.to_string(),
            description: description.to_string(),
            owner_id,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub async fn find_org_by_name(&self, name: &str) -> Result<Option<Organization>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, name, display_name, description, owner_id, created_at FROM organizations WHERE name = ?1"
        )?;
        let mut rows = stmt.query_map(params![name], |row| {
            Ok(Organization {
                id: row.get(0)?,
                name: row.get(1)?,
                display_name: row.get(2)?,
                description: row.get(3)?,
                owner_id: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        match rows.next() {
            Some(Ok(org)) => Ok(Some(org)),
            _ => Ok(None),
        }
    }

    pub async fn find_org_by_id(&self, id: i64) -> Result<Option<Organization>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, name, display_name, description, owner_id, created_at FROM organizations WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Organization {
                id: row.get(0)?,
                name: row.get(1)?,
                display_name: row.get(2)?,
                description: row.get(3)?,
                owner_id: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        match rows.next() {
            Some(Ok(org)) => Ok(Some(org)),
            _ => Ok(None),
        }
    }

    pub async fn update_org(&self, id: i64, display_name: &str, description: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE organizations SET display_name = ?1, description = ?2 WHERE id = ?3",
            params![display_name, description, id],
        )?;
        Ok(())
    }

    pub async fn delete_org(&self, id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM organization_members WHERE org_id = ?1", params![id])?;
        let affected = conn.execute("DELETE FROM organizations WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub async fn list_user_orgs(&self, user_id: i64) -> Result<Vec<OrganizationWithRole>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT o.id, o.name, o.display_name, o.description, o.owner_id, m.role, o.created_at
             FROM organizations o
             JOIN organization_members m ON m.org_id = o.id
             WHERE m.user_id = ?1
             ORDER BY o.name"
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(OrganizationWithRole {
                id: row.get(0)?,
                name: row.get(1)?,
                display_name: row.get(2)?,
                description: row.get(3)?,
                owner_id: row.get(4)?,
                role: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    // ── Organization member operations ──

    pub async fn add_org_member(&self, org_id: i64, user_id: i64, role: &str) -> Result<OrganizationMember, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO organization_members (org_id, user_id, role) VALUES (?1, ?2, ?3)",
            params![org_id, user_id, role],
        )?;
        let id = conn.last_insert_rowid();
        Ok(OrganizationMember {
            id,
            org_id,
            user_id,
            role: role.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub async fn remove_org_member(&self, org_id: i64, user_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let affected = conn.execute(
            "DELETE FROM organization_members WHERE org_id = ?1 AND user_id = ?2",
            params![org_id, user_id],
        )?;
        Ok(affected > 0)
    }

    pub async fn update_org_member_role(&self, org_id: i64, user_id: i64, role: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE organization_members SET role = ?1 WHERE org_id = ?2 AND user_id = ?3",
            params![role, org_id, user_id],
        )?;
        Ok(())
    }

    pub async fn list_org_members(&self, org_id: i64) -> Result<Vec<(OrganizationMember, User)>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT m.id, m.org_id, m.user_id, m.role, m.created_at,
                    u.id, u.username, u.email, u.password_hash, u.bio, u.avatar_url, u.created_at
             FROM organization_members m
             JOIN users u ON u.id = m.user_id
             WHERE m.org_id = ?1 ORDER BY m.role, u.username"
        )?;
        let rows = stmt.query_map(params![org_id], |row| {
            Ok((
                OrganizationMember {
                    id: row.get(0)?,
                    org_id: row.get(1)?,
                    user_id: row.get(2)?,
                    role: row.get(3)?,
                    created_at: row.get(4)?,
                },
                User {
                    id: row.get(5)?,
                    username: row.get(6)?,
                    email: row.get(7)?,
                    password_hash: row.get(8)?,
                    bio: row.get(9)?,
                    avatar_url: row.get(10)?,
                    created_at: row.get(11)?,
                },
            ))
        })?;
        rows.collect()
    }

    // ── Issue operations ──

    pub async fn next_issue_number(&self, repo_id: i64) -> Result<i64, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let max: Option<i64> = conn.query_row(
            "SELECT MAX(number) FROM issues WHERE repo_id = ?1",
            params![repo_id],
            |row| row.get(0),
        ).ok();
        Ok(max.unwrap_or(0) + 1)
    }

    pub async fn create_issue(&self, repo_id: i64, number: i64, title: &str, body: &str, author_id: i64, assignee_id: Option<i64>) -> Result<IssueWithAuthor, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO issues (repo_id, number, title, body, author_id, assignee_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![repo_id, number, title, body, author_id, assignee_id],
        )?;
        let id = conn.last_insert_rowid();
        let author_username: String = conn.query_row(
            "SELECT username FROM users WHERE id = ?1",
            params![author_id],
            |row| row.get(0),
        )?;
        let now = chrono::Utc::now().to_rfc3339();
        Ok(IssueWithAuthor {
            issue: Issue {
                id, repo_id, number,
                title: title.to_string(),
                body: Some(body.to_string()),
                state: "open".to_string(),
                author_id, assignee_id,
                created_at: now.clone(),
                updated_at: now,
                closed_at: None,
            },
            author_username,
            labels: vec![],
        })
    }

    pub async fn list_issues(&self, repo_id: i64, state: Option<&str>) -> Result<Vec<IssueWithAuthor>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let sql = match state {
            Some(s) if s == "open" || s == "closed" => format!(
                "SELECT i.id, i.repo_id, i.number, i.title, i.body, i.state, i.author_id, i.assignee_id, i.created_at, i.updated_at, i.closed_at,
                        u.username
                 FROM issues i JOIN users u ON u.id = i.author_id
                 WHERE i.repo_id = ?1 AND i.state = ?2
                 ORDER BY i.number DESC"
            ),
            _ => format!(
                "SELECT i.id, i.repo_id, i.number, i.title, i.body, i.state, i.author_id, i.assignee_id, i.created_at, i.updated_at, i.closed_at,
                        u.username
                 FROM issues i JOIN users u ON u.id = i.author_id
                 WHERE i.repo_id = ?1
                 ORDER BY i.number DESC"
            ),
        };

        let mut issues: Vec<IssueWithAuthor> = if state == Some("open") || state == Some("closed") {
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![repo_id, state], |row| {
                Ok(IssueWithAuthor {
                    issue: Issue {
                        id: row.get(0)?,
                        repo_id: row.get(1)?,
                        number: row.get(2)?,
                        title: row.get(3)?,
                        body: row.get(4)?,
                        state: row.get(5)?,
                        author_id: row.get(6)?,
                        assignee_id: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                        closed_at: row.get(10)?,
                    },
                    author_username: row.get(11)?,
                    labels: vec![],
                })
            })?;
            rows.collect::<Result<_, _>>()?
        } else {
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![repo_id], |row| {
                Ok(IssueWithAuthor {
                    issue: Issue {
                        id: row.get(0)?,
                        repo_id: row.get(1)?,
                        number: row.get(2)?,
                        title: row.get(3)?,
                        body: row.get(4)?,
                        state: row.get(5)?,
                        author_id: row.get(6)?,
                        assignee_id: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                        closed_at: row.get(10)?,
                    },
                    author_username: row.get(11)?,
                    labels: vec![],
                })
            })?;
            rows.collect::<Result<_, _>>()?
        };
        drop(conn);
        for issue in &mut issues {
            if let Ok(labels) = self.list_issue_labels(issue.issue.id).await {
                issue.labels = labels;
            }
        }
        Ok(issues)
    }

    pub async fn get_issue(&self, repo_id: i64, number: i64) -> Result<Option<IssueWithAuthor>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let sql = "SELECT i.id, i.repo_id, i.number, i.title, i.body, i.state, i.author_id, i.assignee_id, i.created_at, i.updated_at, i.closed_at,
                          u.username
                   FROM issues i JOIN users u ON u.id = i.author_id
                   WHERE i.repo_id = ?1 AND i.number = ?2";
        let mut result = {
            let mut stmt = conn.prepare(sql)?;
            let mut rows = stmt.query_map(params![repo_id, number], |row| {
                Ok(IssueWithAuthor {
                    issue: Issue {
                        id: row.get(0)?,
                        repo_id: row.get(1)?,
                        number: row.get(2)?,
                        title: row.get(3)?,
                        body: row.get(4)?,
                        state: row.get(5)?,
                        author_id: row.get(6)?,
                        assignee_id: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                        closed_at: row.get(10)?,
                    },
                    author_username: row.get(11)?,
                    labels: vec![],
                })
            })?;
            match rows.next() {
                Some(Ok(issue)) => issue,
                _ => return Ok(None),
            }
        };
        drop(conn);
        if let Ok(labels) = self.list_issue_labels(result.issue.id).await {
            result.labels = labels;
        }
        Ok(Some(result))
    }

    pub async fn update_issue(&self, id: i64, repo_id: i64, title: Option<&str>, body: Option<&str>, state: Option<&str>, assignee_id: Option<Option<i64>>) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut parts = vec!["updated_at = datetime('now')".to_string()];
        let mut vals: Vec<String> = vec![];
        let mut param_idx: usize = 0;

        if let Some(t) = title { parts.push(format!("title = ?{}", param_idx + 1)); vals.push(t.to_string()); param_idx += 1; }
        if let Some(b) = body { parts.push(format!("body = ?{}", param_idx + 1)); vals.push(b.to_string()); param_idx += 1; }
        if let Some(s) = state {
            parts.push(format!("state = ?{}", param_idx + 1)); vals.push(s.to_string()); param_idx += 1;
            if s == "closed" { parts.push("closed_at = datetime('now')".to_string()); }
            else { parts.push("closed_at = NULL".to_string()); }
        }
        if let Some(assign) = assignee_id {
            match assign {
                Some(aid) => { parts.push(format!("assignee_id = {}", aid)); },
                None => { parts.push("assignee_id = NULL".to_string()); },
            }
        }

        let sql = format!("UPDATE issues SET {} WHERE id = ?{} AND repo_id = ?{}", parts.join(", "), param_idx + 1, param_idx + 2);
        vals.push(id.to_string());
        vals.push(repo_id.to_string());

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|v| v as &dyn rusqlite::types::ToSql).collect();
        let affected = conn.execute(&sql, params_refs.as_slice())?;
        Ok(affected > 0)
    }

    pub async fn delete_issue(&self, id: i64, repo_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM issue_label_map WHERE issue_id = ?1", params![id])?;
        conn.execute("DELETE FROM issue_comments WHERE issue_id = ?1", params![id])?;
        let affected = conn.execute("DELETE FROM issues WHERE id = ?1 AND repo_id = ?2", params![id, repo_id])?;
        Ok(affected > 0)
    }

    // ── Labels ──

    pub async fn create_label(&self, repo_id: i64, name: &str, color: &str) -> Result<IssueLabel, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO issue_labels (repo_id, name, color) VALUES (?1, ?2, ?3)",
            params![repo_id, name, color],
        )?;
        let id = conn.last_insert_rowid();
        Ok(IssueLabel { id, repo_id: repo_id, name: name.to_string(), color: color.to_string() })
    }

    pub async fn list_labels(&self, repo_id: i64) -> Result<Vec<IssueLabel>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, repo_id, name, color FROM issue_labels WHERE repo_id = ?1 ORDER BY name"
        )?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok(IssueLabel { id: row.get(0)?, repo_id: row.get(1)?, name: row.get(2)?, color: row.get(3)? })
        })?;
        rows.collect()
    }

    pub async fn list_issue_labels(&self, issue_id: i64) -> Result<Vec<IssueLabel>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT l.id, l.repo_id, l.name, l.color
             FROM issue_labels l
             JOIN issue_label_map m ON m.label_id = l.id
             WHERE m.issue_id = ?1"
        )?;
        let rows = stmt.query_map(params![issue_id], |row| {
            Ok(IssueLabel { id: row.get(0)?, repo_id: row.get(1)?, name: row.get(2)?, color: row.get(3)? })
        })?;
        rows.collect()
    }

    pub async fn set_issue_labels(&self, issue_id: i64, label_ids: &[i64]) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM issue_label_map WHERE issue_id = ?1", params![issue_id])?;
        for &lid in label_ids {
            conn.execute(
                "INSERT OR IGNORE INTO issue_label_map (issue_id, label_id) VALUES (?1, ?2)",
                params![issue_id, lid],
            )?;
        }
        Ok(())
    }

    pub async fn delete_label(&self, id: i64, repo_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM issue_label_map WHERE label_id = ?1", params![id])?;
        let affected = conn.execute("DELETE FROM issue_labels WHERE id = ?1 AND repo_id = ?2", params![id, repo_id])?;
        Ok(affected > 0)
    }

    // ── Comments ──

    pub async fn add_comment(&self, issue_id: i64, author_id: i64, body: &str) -> Result<IssueComment, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO issue_comments (issue_id, author_id, body) VALUES (?1, ?2, ?3)",
            params![issue_id, author_id, body],
        )?;
        let id = conn.last_insert_rowid();
        let author_username: String = conn.query_row(
            "SELECT username FROM users WHERE id = ?1", params![author_id], |row| row.get(0),
        )?;
        let now = chrono::Utc::now().to_rfc3339();
        Ok(IssueComment { id, issue_id, author_id, author_username, body: body.to_string(), created_at: now.clone(), updated_at: now })
    }

    pub async fn list_comments(&self, issue_id: i64) -> Result<Vec<IssueComment>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT c.id, c.issue_id, c.author_id, u.username, c.body, c.created_at, c.updated_at
             FROM issue_comments c JOIN users u ON u.id = c.author_id
             WHERE c.issue_id = ?1 ORDER BY c.created_at ASC"
        )?;
        let rows = stmt.query_map(params![issue_id], |row| {
            Ok(IssueComment {
                id: row.get(0)?,
                issue_id: row.get(1)?,
                author_id: row.get(2)?,
                author_username: row.get(3)?,
                body: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    // ── Pull Request operations ──

    pub async fn next_pr_number(&self, repo_id: i64) -> Result<i64, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let max: Option<i64> = conn.query_row(
            "SELECT MAX(number) FROM pull_requests WHERE repo_id = ?1",
            params![repo_id],
            |row| row.get(0),
        ).ok();
        Ok(max.unwrap_or(0) + 1)
    }

    pub async fn create_pr(&self, repo_id: i64, number: i64, title: &str, body: &str, author_id: i64, head_repo_id: i64, head_ref: &str, base_ref: &str) -> Result<PullRequestWithAuthor, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO pull_requests (repo_id, number, title, body, author_id, head_repo_id, head_ref, base_ref) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![repo_id, number, title, body, author_id, head_repo_id, head_ref, base_ref],
        )?;
        let id = conn.last_insert_rowid();
        let author_username: String = conn.query_row(
            "SELECT username FROM users WHERE id = ?1", params![author_id], |row| row.get(0),
        )?;
        let head_repo_name: String = conn.query_row(
            "SELECT name FROM repositories WHERE id = ?1", params![head_repo_id], |row| row.get(0),
        )?;
        let head_repo_owner: String = conn.query_row(
            "SELECT COALESCE(u.username, o.name) FROM repositories r
             LEFT JOIN users u ON u.id = r.user_id AND r.owner_type = 'user'
             LEFT JOIN organizations o ON o.id = r.org_id AND r.owner_type = 'org'
             WHERE r.id = ?1",
            params![head_repo_id], |row| row.get(0),
        )?;
        let now = chrono::Utc::now().to_rfc3339();
        Ok(PullRequestWithAuthor {
            pr: PullRequest {
                id, repo_id, number,
                title: title.to_string(),
                body: Some(body.to_string()),
                state: "open".to_string(),
                author_id, head_repo_id,
                head_ref: head_ref.to_string(),
                base_ref: base_ref.to_string(),
                merge_commit_sha: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                closed_at: None,
                merged_at: None,
            },
            author_username,
            head_repo_name,
            head_repo_owner,
        })
    }

    pub async fn list_prs(&self, repo_id: i64, state: Option<&str>) -> Result<Vec<PullRequestWithAuthor>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let sql = match state {
            Some(s) if s == "open" || s == "closed" || s == "merged" => format!(
                "SELECT p.id, p.repo_id, p.number, p.title, p.body, p.state, p.author_id, p.head_repo_id, p.head_ref, p.base_ref, p.merge_commit_sha, p.created_at, p.updated_at, p.closed_at, p.merged_at,
                        u.username, hr.name, COALESCE(u2.username, o.name)
                 FROM pull_requests p
                 JOIN users u ON u.id = p.author_id
                 JOIN repositories hr ON hr.id = p.head_repo_id
                 LEFT JOIN users u2 ON u2.id = hr.user_id AND hr.owner_type = 'user'
                 LEFT JOIN organizations o ON o.id = hr.org_id AND hr.owner_type = 'org'
                 WHERE p.repo_id = ?1 AND p.state = ?2
                 ORDER BY p.number DESC"
            ),
            _ => format!(
                "SELECT p.id, p.repo_id, p.number, p.title, p.body, p.state, p.author_id, p.head_repo_id, p.head_ref, p.base_ref, p.merge_commit_sha, p.created_at, p.updated_at, p.closed_at, p.merged_at,
                        u.username, hr.name, COALESCE(u2.username, o.name)
                 FROM pull_requests p
                 JOIN users u ON u.id = p.author_id
                 JOIN repositories hr ON hr.id = p.head_repo_id
                 LEFT JOIN users u2 ON u2.id = hr.user_id AND hr.owner_type = 'user'
                 LEFT JOIN organizations o ON o.id = hr.org_id AND hr.owner_type = 'org'
                 WHERE p.repo_id = ?1
                 ORDER BY p.number DESC"
            ),
        };

        let map_fn = |row: &rusqlite::Row| -> rusqlite::Result<PullRequestWithAuthor> {
            Ok(PullRequestWithAuthor {
                pr: PullRequest {
                    id: row.get(0)?, repo_id: row.get(1)?, number: row.get(2)?,
                    title: row.get(3)?, body: row.get(4)?, state: row.get(5)?,
                    author_id: row.get(6)?, head_repo_id: row.get(7)?,
                    head_ref: row.get(8)?, base_ref: row.get(9)?,
                    merge_commit_sha: row.get(10)?,
                    created_at: row.get(11)?, updated_at: row.get(12)?,
                    closed_at: row.get(13)?, merged_at: row.get(14)?,
                },
                author_username: row.get(15)?,
                head_repo_name: row.get(16)?,
                head_repo_owner: row.get(17)?,
            })
        };

        if state == Some("open") || state == Some("closed") || state == Some("merged") {
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![repo_id, state], map_fn)?;
            rows.collect()
        } else {
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![repo_id], map_fn)?;
            rows.collect()
        }
    }

    pub async fn get_pr(&self, repo_id: i64, number: i64) -> Result<Option<PullRequestWithAuthor>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT p.id, p.repo_id, p.number, p.title, p.body, p.state, p.author_id, p.head_repo_id, p.head_ref, p.base_ref, p.merge_commit_sha, p.created_at, p.updated_at, p.closed_at, p.merged_at,
                    u.username, hr.name, COALESCE(u2.username, o.name)
             FROM pull_requests p
             JOIN users u ON u.id = p.author_id
             JOIN repositories hr ON hr.id = p.head_repo_id
             LEFT JOIN users u2 ON u2.id = hr.user_id AND hr.owner_type = 'user'
             LEFT JOIN organizations o ON o.id = hr.org_id AND hr.owner_type = 'org'
             WHERE p.repo_id = ?1 AND p.number = ?2"
        )?;
        let mut rows = stmt.query_map(params![repo_id, number], |row| {
            Ok(PullRequestWithAuthor {
                pr: PullRequest {
                    id: row.get(0)?, repo_id: row.get(1)?, number: row.get(2)?,
                    title: row.get(3)?, body: row.get(4)?, state: row.get(5)?,
                    author_id: row.get(6)?, head_repo_id: row.get(7)?,
                    head_ref: row.get(8)?, base_ref: row.get(9)?,
                    merge_commit_sha: row.get(10)?,
                    created_at: row.get(11)?, updated_at: row.get(12)?,
                    closed_at: row.get(13)?, merged_at: row.get(14)?,
                },
                author_username: row.get(15)?,
                head_repo_name: row.get(16)?,
                head_repo_owner: row.get(17)?,
            })
        })?;
        rows.next().transpose()
    }

    pub async fn update_pr(&self, id: i64, repo_id: i64, title: Option<&str>, body: Option<&str>, state: Option<&str>) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut parts = vec!["updated_at = datetime('now')".to_string()];
        let mut vals: Vec<String> = vec![];

        if let Some(t) = title { parts.push(format!("title = ?{}", vals.len() + 1)); vals.push(t.to_string()); }
        if let Some(b) = body { parts.push(format!("body = ?{}", vals.len() + 1)); vals.push(b.to_string()); }
        if let Some(s) = state {
            parts.push(format!("state = ?{}", vals.len() + 1)); vals.push(s.to_string());
            if s == "closed" { parts.push("closed_at = datetime('now')".to_string()); }
            else if s == "merged" { parts.push("merged_at = datetime('now')".to_string()); parts.push("state = 'merged'".to_string()); }
            else { parts.push("closed_at = NULL".to_string()); }
        }

        let sql = format!("UPDATE pull_requests SET {} WHERE id = ?{} AND repo_id = ?{}", parts.join(", "), vals.len() + 1, vals.len() + 2);
        vals.push(id.to_string());
        vals.push(repo_id.to_string());
        let params_refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|v| v as &dyn rusqlite::types::ToSql).collect();
        let affected = conn.execute(&sql, params_refs.as_slice())?;
        Ok(affected > 0)
    }

    pub async fn set_pr_merge_sha(&self, pr_id: i64, merge_sha: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE pull_requests SET merge_commit_sha = ?1, merged_at = datetime('now'), state = 'merged' WHERE id = ?2",
            params![merge_sha, pr_id],
        )?;
        Ok(())
    }

    // ── Fork operations ──

    pub async fn list_user_repos_all(&self, user_id: i64) -> Result<Vec<Repository>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, description, is_private, default_branch, owner_type, org_id, created_at, updated_at, forked_from, stars_count, forks_count, watch_count
             FROM repositories WHERE user_id = ?1 AND owner_type = 'user' ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map(params![user_id], map_repo_row)?;
        rows.collect()
    }

    pub async fn set_repo_forked_from(&self, repo_id: i64, source_repo_id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE repositories SET forked_from = ?1 WHERE id = ?2",
            params![source_repo_id, repo_id],
        )?;
        Ok(())
    }

    // ── v2.1 Settings ──

    pub async fn list_access_tokens(&self, user_id: i64) -> Result<Vec<AccessToken>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, token_prefix, scopes, last_used_at, created_at, expires_at
             FROM access_tokens WHERE user_id = ?1 ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(AccessToken {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                token_prefix: row.get(3)?,
                scopes: row.get(4)?,
                last_used_at: row.get(5)?,
                created_at: row.get(6)?,
                expires_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    pub async fn create_access_token(&self, user_id: i64, name: &str, token_hash: &str, token_prefix: &str, scopes: &str, expires_at: Option<&str>) -> Result<AccessToken, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let expires_at = expires_at.map(|s| s.to_string());
        conn.execute(
            "INSERT INTO access_tokens (user_id, name, token_prefix, token_hash, scopes, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![user_id, name, token_prefix, token_hash, scopes, expires_at],
        )?;
        let id = conn.last_insert_rowid();
        Ok(AccessToken {
            id,
            user_id,
            name: name.to_string(),
            token_prefix: token_prefix.to_string(),
            scopes: scopes.to_string(),
            last_used_at: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            expires_at,
        })
    }

    pub async fn delete_access_token(&self, token_id: i64, user_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let affected = conn.execute(
            "DELETE FROM access_tokens WHERE id = ?1 AND user_id = ?2",
            params![token_id, user_id],
        )?;
        Ok(affected > 0)
    }

    pub async fn add_collaborator(&self, repo_id: i64, user_id: i64, permission: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO repo_collaborators (repo_id, user_id, permission) VALUES (?1, ?2, ?3)",
            params![repo_id, user_id, permission],
        )?;
        Ok(())
    }

    pub async fn list_collaborators(&self, repo_id: i64) -> Result<Vec<RepoCollaborator>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT rc.repo_id, rc.user_id, rc.permission, u.username, u.avatar_url
             FROM repo_collaborators rc
             JOIN users u ON u.id = rc.user_id
             WHERE rc.repo_id = ?1"
        )?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok(RepoCollaborator {
                repo_id: row.get(0)?,
                user_id: row.get(1)?,
                permission: row.get(2)?,
                username: row.get(3)?,
                avatar_url: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub async fn remove_collaborator(&self, repo_id: i64, user_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let affected = conn.execute(
            "DELETE FROM repo_collaborators WHERE repo_id = ?1 AND user_id = ?2",
            params![repo_id, user_id],
        )?;
        Ok(affected > 0)
    }

    pub async fn create_secret(&self, repo_id: i64, name: &str, encrypted_value: &[u8]) -> Result<RepoSecret, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO repo_secrets (repo_id, name, encrypted_value) VALUES (?1, ?2, ?3)",
            params![repo_id, name, encrypted_value],
        )?;
        let id = conn.last_insert_rowid();
        Ok(RepoSecret {
            id,
            repo_id,
            name: name.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub async fn list_secrets(&self, repo_id: i64) -> Result<Vec<RepoSecret>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, repo_id, name, created_at FROM repo_secrets WHERE repo_id = ?1 ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok(RepoSecret {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub async fn delete_secret(&self, secret_id: i64, repo_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let affected = conn.execute(
            "DELETE FROM repo_secrets WHERE id = ?1 AND repo_id = ?2",
            params![secret_id, repo_id],
        )?;
        Ok(affected > 0)
    }

    pub async fn create_branch_protection(&self, repo_id: i64, pattern: &str, require_pr: bool, require_approvals: i64, dismiss_stale_reviews: bool) -> Result<BranchProtection, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO branch_protection (repo_id, pattern, require_pr, require_approvals, dismiss_stale_reviews)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![repo_id, pattern, require_pr as i32, require_approvals, dismiss_stale_reviews as i32],
        )?;
        let id = conn.last_insert_rowid();
        Ok(BranchProtection {
            id,
            repo_id,
            pattern: pattern.to_string(),
            require_pr,
            require_approvals,
            dismiss_stale_reviews,
        })
    }

    pub async fn list_branch_protections(&self, repo_id: i64) -> Result<Vec<BranchProtection>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, repo_id, pattern, require_pr, require_approvals, dismiss_stale_reviews
             FROM branch_protection WHERE repo_id = ?1"
        )?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok(BranchProtection {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                pattern: row.get(2)?,
                require_pr: row.get::<_, i32>(3)? != 0,
                require_approvals: row.get(4)?,
                dismiss_stale_reviews: row.get::<_, i32>(5)? != 0,
            })
        })?;
        rows.collect()
    }

    pub async fn delete_branch_protection(&self, protection_id: i64, repo_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let affected = conn.execute(
            "DELETE FROM branch_protection WHERE id = ?1 AND repo_id = ?2",
            params![protection_id, repo_id],
        )?;
        Ok(affected > 0)
    }

    // ── v2.2 Stars ──

    pub async fn star_repo(&self, user_id: i64, repo_id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR IGNORE INTO stars (user_id, repo_id) VALUES (?1, ?2)",
            params![user_id, repo_id],
        )?;
        conn.execute(
            "UPDATE repositories SET stars_count = (SELECT COUNT(*) FROM stars WHERE repo_id = ?1) WHERE id = ?1",
            params![repo_id],
        )?;
        Ok(())
    }

    pub async fn unstar_repo(&self, user_id: i64, repo_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let affected = conn.execute(
            "DELETE FROM stars WHERE user_id = ?1 AND repo_id = ?2",
            params![user_id, repo_id],
        )?;
        conn.execute(
            "UPDATE repositories SET stars_count = (SELECT COUNT(*) FROM stars WHERE repo_id = ?1) WHERE id = ?1",
            params![repo_id],
        )?;
        Ok(affected > 0)
    }

    pub async fn is_starred(&self, user_id: i64, repo_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM stars WHERE user_id = ?1 AND repo_id = ?2",
            params![user_id, repo_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub async fn list_stargazers(&self, repo_id: i64) -> Result<Vec<UserPublic>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT u.id, u.username, u.bio, u.avatar_url, u.created_at
             FROM stars s JOIN users u ON u.id = s.user_id
             WHERE s.repo_id = ?1 ORDER BY s.created_at DESC"
        )?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok(UserPublic {
                id: row.get(0)?,
                username: row.get(1)?,
                bio: row.get(2)?,
                avatar_url: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub async fn list_user_stars(&self, user_id: i64) -> Result<Vec<Repository>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT r.id, r.user_id, r.name, r.description, r.is_private, r.default_branch, r.owner_type, r.org_id, r.created_at, r.updated_at, r.forked_from, r.stars_count, r.forks_count, r.watch_count
             FROM stars s JOIN repositories r ON r.id = s.repo_id
             WHERE s.user_id = ?1 ORDER BY s.created_at DESC"
        )?;
        let rows = stmt.query_map(params![user_id], map_repo_row)?;
        rows.collect()
    }

    // ── v2.2 Watches ──

    pub async fn watch_repo(&self, user_id: i64, repo_id: i64, watch_type: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO watches (user_id, repo_id, watch_type) VALUES (?1, ?2, ?3)",
            params![user_id, repo_id, watch_type],
        )?;
        conn.execute(
            "UPDATE repositories SET watch_count = (SELECT COUNT(*) FROM watches WHERE repo_id = ?1) WHERE id = ?1",
            params![repo_id],
        )?;
        Ok(())
    }

    pub async fn unwatch_repo(&self, user_id: i64, repo_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let affected = conn.execute(
            "DELETE FROM watches WHERE user_id = ?1 AND repo_id = ?2",
            params![user_id, repo_id],
        )?;
        conn.execute(
            "UPDATE repositories SET watch_count = (SELECT COUNT(*) FROM watches WHERE repo_id = ?1) WHERE id = ?1",
            params![repo_id],
        )?;
        Ok(affected > 0)
    }

    pub async fn get_watch_type(&self, user_id: i64, repo_id: i64) -> Result<Option<String>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let result = conn.query_row(
            "SELECT watch_type FROM watches WHERE user_id = ?1 AND repo_id = ?2",
            params![user_id, repo_id],
            |row| row.get(0),
        );
        match result {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

fn map_repo_row(row: &rusqlite::Row) -> rusqlite::Result<Repository> {
    Ok(Repository {
        id: row.get(0)?,
        user_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        is_private: row.get::<_, i32>(4)? != 0,
        default_branch: row.get(5)?,
        owner_type: row.get(6)?,
        org_id: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        forked_from: row.get(10)?,
        stars_count: row.get(11)?,
        forks_count: row.get(12)?,
        watch_count: row.get(13)?,
    })
}
