use std::fs;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
    pub jwt: JwtConfig,
    #[serde(default)]
    pub ssh: SshConfig,
    #[serde(default)]
    pub cors: CorsConfig,
    #[serde(default)]
    pub upload: UploadConfig,
    #[serde(default)]
    pub apps: AppsConfig,
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub docker: DockerConfig,
    #[serde(default)]
    pub secrets: SecretsConfig,
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

impl JwtConfig {
    pub fn effective_secret(&self) -> String {
        std::env::var("JWT_SECRET").unwrap_or_else(|_| self.secret.clone())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppsConfig {
    pub port_range_start: u16,
    pub port_range_end: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SshConfig {
    pub enabled: bool,
}

impl Default for SshConfig {
    fn default() -> Self {
        SshConfig { enabled: true }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RuntimeConfig {
    pub mode: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig { mode: "process".to_string() }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        CorsConfig { allowed_origins: vec!["*".to_string()] }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadConfig {
    pub max_file_size: usize,
}

impl Default for UploadConfig {
    fn default() -> Self {
        UploadConfig { max_file_size: 10 * 1024 * 1024 }
    }
}

impl Default for AppsConfig {
    fn default() -> Self {
        AppsConfig { port_range_start: 4000, port_range_end: 65535 }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DockerConfig {
    pub base_image: String,
    pub network: String,
    pub memory_limit: String,
    pub cpu_shares: i64,
    pub ssh_port_range_start: u16,
    pub ssh_port_range_end: u16,
}

impl Default for DockerConfig {
    fn default() -> Self {
        DockerConfig {
            base_image: "gitpage-dev-base:latest".to_string(),
            network: "bridge".to_string(),
            memory_limit: "1g".to_string(),
            cpu_shares: 512,
            ssh_port_range_start: 22001,
            ssh_port_range_end: 22999,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SecretsConfig {
    pub encryption_key: String,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        SecretsConfig {
            encryption_key: String::new(),
        }
    }
}

impl Config {
    pub fn from_file(path: &str) -> Self {
        let content = fs::read_to_string(path).expect("Failed to read config file");
        toml::from_str(&content).expect("Failed to parse config file")
    }

    pub fn repo_path(&self, username: &str, repo: &str) -> String {
        format!("{}/repos/{}/{}.git", self.storage.base_path, username, repo)
    }

    pub fn user_repos_path(&self, username: &str) -> String {
        format!("{}/repos/{}", self.storage.base_path, username)
    }

    pub fn pages_dir(&self, username: &str, repo: &str) -> String {
        format!("{}/repos/{}/{}/pages", self.storage.base_path, username, repo)
    }

    pub fn app_workspace_dir(&self, username: &str, repo: &str) -> String {
        format!("{}/apps/{}/{}", self.storage.base_path, username, repo)
    }

    pub fn working_tree_path(&self, username: &str, repo: &str) -> String {
        format!("{}/{}", self.storage.base_path, username)
    }

    pub fn staging_path(&self, username: &str, repo: &str) -> String {
        format!("{}/staging/{}/{}", self.storage.base_path, username, repo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            server: ServerConfig { host: "0.0.0.0".to_string(), port: 8080 },
            database: DatabaseConfig { path: "data/test.db".to_string() },
            storage: StorageConfig { base_path: "data".to_string() },
            jwt: JwtConfig { secret: "test-secret".to_string(), expires_in_hours: 24 },
            ssh: SshConfig { enabled: false },
            cors: CorsConfig { allowed_origins: vec!["*".to_string()] },
            upload: UploadConfig { max_file_size: 5 * 1024 * 1024 },
            apps: AppsConfig { port_range_start: 4000, port_range_end: 5000 },
            runtime: RuntimeConfig { mode: "process".to_string() },
            docker: DockerConfig {
                base_image: "test-img".to_string(),
                network: "bridge".to_string(),
                memory_limit: "512m".to_string(),
                cpu_shares: 256,
                ssh_port_range_start: 22001,
                ssh_port_range_end: 22010,
            },
            secrets: SecretsConfig { encryption_key: "test-key".to_string() },
        }
    }

    #[test]
    fn test_config_paths() {
        let cfg = test_config();
        assert_eq!(cfg.repo_path("alice", "myrepo"), "data/repos/alice/myrepo.git");
        assert_eq!(cfg.staging_path("alice", "myrepo"), "data/staging/alice/myrepo");
        assert_eq!(cfg.app_workspace_dir("alice", "myrepo"), "data/apps/alice/myrepo");
        assert_eq!(cfg.pages_dir("alice", "myrepo"), "data/repos/alice/myrepo/pages");
        assert_eq!(cfg.user_repos_path("alice"), "data/repos/alice");
    }

    #[test]
    fn test_config_defaults() {
        let ssh = SshConfig::default();
        assert!(ssh.enabled);

        let cors = CorsConfig::default();
        assert_eq!(cors.allowed_origins, vec!["*"]);

        let upload = UploadConfig::default();
        assert_eq!(upload.max_file_size, 10 * 1024 * 1024);

        let apps = AppsConfig::default();
        assert_eq!(apps.port_range_start, 4000);
        assert_eq!(apps.port_range_end, 65535);

        let runtime = RuntimeConfig::default();
        assert_eq!(runtime.mode, "process");

        let docker = DockerConfig::default();
        assert_eq!(docker.base_image, "gitpage-dev-base:latest");
        assert_eq!(docker.network, "bridge");
        assert_eq!(docker.memory_limit, "1g");
        assert_eq!(docker.cpu_shares, 512);
        assert_eq!(docker.ssh_port_range_start, 22001);
        assert_eq!(docker.ssh_port_range_end, 22999);

        let secrets = SecretsConfig::default();
        assert_eq!(secrets.encryption_key, "");
    }

    #[test]
    fn test_effective_secret_prefers_env() {
        let cfg = JwtConfig { secret: "config-secret".to_string(), expires_in_hours: 24 };
        std::env::set_var("JWT_SECRET", "env-secret");
        assert_eq!(cfg.effective_secret(), "env-secret");
        std::env::remove_var("JWT_SECRET");
        assert_eq!(cfg.effective_secret(), "config-secret");
    }

    #[test]
    fn test_config_ssh_default_enabled() {
        let cfg = test_config();
        assert!(!cfg.ssh.enabled);
    }

    #[test]
    fn test_config_custom_base_path() {
        let mut cfg = test_config();
        cfg.storage.base_path = "/custom/data".to_string();
        assert_eq!(cfg.repo_path("bob", "repo"), "/custom/data/repos/bob/repo.git");
        assert_eq!(cfg.staging_path("bob", "repo"), "/custom/data/staging/bob/repo");
    }
}
