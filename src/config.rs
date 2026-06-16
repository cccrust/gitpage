use std::fs;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
    pub jwt: JwtConfig,
    #[serde(default)]
    pub apps: AppsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub base_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expires_in_hours: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppsConfig {
    pub port_range_start: u16,
    pub port_range_end: u16,
}

impl Default for AppsConfig {
    fn default() -> Self {
        AppsConfig { port_range_start: 4000, port_range_end: 65535 }
    }
}

impl Config {
    pub fn from_file(path: &str) -> Self {
        let content = fs::read_to_string(path).expect("Failed to read config file");
        toml::from_str(&content).expect("Failed to parse config file")
    }

    pub fn repo_path(&self, username: &str, repo: &str) -> String {
        format!("{}/{}/{}.git", self.storage.base_path, username, repo)
    }

    pub fn user_repos_path(&self, username: &str) -> String {
        format!("{}/{}", self.storage.base_path, username)
    }

    pub fn pages_dir(&self, username: &str, repo: &str) -> String {
        format!("{}/{}/{}/pages", self.storage.base_path, username, repo)
    }

    pub fn app_workspace_dir(&self, username: &str, repo: &str) -> String {
        format!("data/apps/{}/{}", username, repo)
    }

    pub fn working_tree_path(&self, username: &str, repo: &str) -> String {
        format!("{}/{}/{}", self.storage.base_path, username, repo)
    }

    pub fn staging_path(&self, username: &str, repo: &str) -> String {
        format!("data/staging/{}/{}", username, repo)
    }
}
