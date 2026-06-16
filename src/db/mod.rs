pub mod models;

use rusqlite::{Connection, params};
use std::sync::Arc;
use tokio::sync::Mutex;
use models::{User, Repository, PagesConfig, AppsConfig, DeployLog, SshKey};

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
            "CREATE TABLE IF NOT EXISTS users (
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
                created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, name)
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
                enabled         INTEGER DEFAULT 0
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

    pub async fn create_repo(&self, user_id: i64, name: &str, description: &str, is_private: bool) -> Result<Repository, rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO repositories (user_id, name, description, is_private) VALUES (?1, ?2, ?3, ?4)",
            params![user_id, name, description, is_private as i32],
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
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn list_user_repos(&self, user_id: i64) -> Result<Vec<Repository>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, description, is_private, default_branch, created_at, updated_at
             FROM repositories WHERE user_id = ?1 ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(Repository {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                is_private: row.get::<_, i32>(4)? != 0,
                default_branch: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    pub async fn list_public_user_repos(&self, user_id: i64) -> Result<Vec<Repository>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, description, is_private, default_branch, created_at, updated_at
             FROM repositories WHERE user_id = ?1 AND is_private = 0 ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(Repository {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                is_private: row.get::<_, i32>(4)? != 0,
                default_branch: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    pub async fn find_repo_by_name(&self, user_id: i64, name: &str) -> Result<Option<Repository>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, description, is_private, default_branch, created_at, updated_at
             FROM repositories WHERE user_id = ?1 AND name = ?2"
        )?;
        let mut rows = stmt.query_map(params![user_id, name], |row| {
            Ok(Repository {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                is_private: row.get::<_, i32>(4)? != 0,
                default_branch: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(Ok(repo)) => Ok(Some(repo)),
            _ => Ok(None),
        }
    }

    pub async fn find_repo_by_id(&self, id: i64) -> Result<Option<Repository>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, description, is_private, default_branch, created_at, updated_at
             FROM repositories WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Repository {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                is_private: row.get::<_, i32>(4)? != 0,
                default_branch: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
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

    pub async fn search_repos(&self, query: &str, limit: i64) -> Result<Vec<Repository>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, description, is_private, default_branch, created_at, updated_at
             FROM repositories WHERE (name LIKE ?1 OR description LIKE ?1) AND is_private = 0
             ORDER BY updated_at DESC LIMIT ?2"
        )?;
        let rows = stmt.query_map(params![pattern, limit], |row| {
            Ok(Repository {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                is_private: row.get::<_, i32>(4)? != 0,
                default_branch: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    pub async fn update_repo(&self, id: i64, description: &str, is_private: bool) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE repositories SET description = ?1, is_private = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
            params![description, is_private as i32, id],
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

    pub async fn get_apps_config(&self, repo_id: i64) -> Result<Option<AppsConfig>, rusqlite::Error> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, repo_id, branch, source_dir, build_command, start_command, env_vars, enabled FROM apps_config WHERE repo_id = ?1"
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
            "SELECT id, repo_id, branch, source_dir, build_command, start_command, env_vars, enabled FROM apps_config WHERE enabled = 1"
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
                    r.id, r.user_id, r.name, r.description, r.is_private, r.default_branch, r.created_at, r.updated_at
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
                    created_at: row.get(19)?,
                    updated_at: row.get(20)?,
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
}
