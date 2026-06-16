pub mod models;

use rusqlite::{Connection, params};
use std::sync::Arc;
use tokio::sync::Mutex;
use models::{User, Repository, PagesConfig};

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
        let affected = conn.execute("DELETE FROM repositories WHERE id = ?1", params![id])?;
        Ok(affected > 0)
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
