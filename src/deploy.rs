use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::db::models::AppsConfig;
use crate::docker::DockerManager;
use crate::utils::errors::AppError;

#[derive(Debug, Clone, PartialEq)]
pub enum AppStatus {
    Deploying,
    Running,
    Stopped,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct AppProcess {
    pub repo_id: i64,
    pub port: u16,
    pub status: AppStatus,
    pub pid: u32,
}

#[derive(Clone)]
pub struct AppProcessManager {
    processes: Arc<Mutex<HashMap<i64, AppProcess>>>,
    port_allocator: Arc<AtomicU16>,
    port_range_end: u16,
}

impl AppProcessManager {
    pub fn new(port_start: u16, port_end: u16) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            port_allocator: Arc::new(AtomicU16::new(port_start)),
            port_range_end: port_end,
        }
    }

    pub async fn allocate_port(&self) -> Result<u16, AppError> {
        let procs = self.processes.lock().await;
        let used_ports: std::collections::HashSet<u16> =
            procs.values().map(|p| p.port).collect();
        let mut port = self.port_allocator.load(Ordering::Relaxed);
        for _ in 0..(self.port_range_end - port) {
            if !used_ports.contains(&port) {
                return Ok(port);
            }
            port += 1;
        }
        Err(AppError::Internal("沒有可用的埠號".into()))
    }

    pub async fn register(&self, proc: AppProcess) {
        let mut procs = self.processes.lock().await;
        procs.insert(proc.repo_id, proc);
    }

    pub async fn unregister(&self, repo_id: i64) {
        let mut procs = self.processes.lock().await;
        procs.remove(&repo_id);
    }

    pub async fn get(&self, repo_id: i64) -> Option<AppProcess> {
        let procs = self.processes.lock().await;
        procs.get(&repo_id).cloned()
    }

    pub async fn update_status(&self, repo_id: i64, status: AppStatus) {
        let mut procs = self.processes.lock().await;
        if let Some(p) = procs.get_mut(&repo_id) {
            p.status = status;
        }
    }

    pub async fn list(&self) -> Vec<AppProcess> {
        let procs = self.processes.lock().await;
        procs.values().cloned().collect()
    }
}

pub fn detect_project_type(workspace_dir: &str) -> Result<ProjectType, AppError> {
    let source_dir = std::path::Path::new(workspace_dir);

    let package_json = source_dir.join("package.json");
    if package_json.exists() {
        return Ok(ProjectType::NodeJs);
    }

    let cargo_toml = source_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        return Ok(ProjectType::Rust);
    }

    Err(AppError::BadRequest(
        "不支援的專案類型，僅支援 Node.js (package.json) 與 Rust (Cargo.toml)".into()
    ))
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    NodeJs,
    Rust,
}

pub fn resolve_commands(project_type: &ProjectType, config: &AppsConfig, workspace_dir: &str) -> (String, String) {
    let build = if !config.build_command.is_empty() {
        config.build_command.clone()
    } else {
        match project_type {
            ProjectType::NodeJs => "npm install".to_string(),
            ProjectType::Rust => "cargo build --release".to_string(),
        }
    };

    let start = if !config.start_command.is_empty() {
        config.start_command.clone()
    } else {
        match project_type {
            ProjectType::NodeJs => "npm start".to_string(),
            ProjectType::Rust => {
                let cargo_path = std::path::Path::new(workspace_dir).join("Cargo.toml");
                match std::fs::read_to_string(&cargo_path) {
                    Ok(content) => {
                        let name = content.lines()
                            .find(|l| l.trim().starts_with("name"))
                            .and_then(|l| l.split('=').nth(1))
                            .map(|v| v.trim().trim_matches('"').trim().to_string())
                            .unwrap_or_else(|| "app".to_string());
                        format!("./target/release/{}", name)
                    }
                    Err(_) => {
                        "./target/release/app".to_string()
                    }
                }
            }
        }
    };

    (build, start)
}

pub async fn checkout_source(bare_repo_path: &str, workspace_dir: &str, branch: &str, source_dir: &str) -> Result<(), AppError> {
    let checkout_path = std::path::Path::new(workspace_dir).join("source");
    std::fs::create_dir_all(&checkout_path)
        .map_err(|e| AppError::Internal(format!("Failed to create checkout dir: {}", e)))?;

    // Use git CLI to checkout from bare repo
    let status = tokio::process::Command::new("git")
        .args([
            "--work-tree", checkout_path.to_str().unwrap(),
            "--git-dir", bare_repo_path,
            "checkout", "-f", branch, "--",
        ])
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to run git checkout: {}", e)))?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        tracing::warn!("git checkout stderr: {}", stderr);
    }

    // If source_dir is not root, move contents
    let effective_source = checkout_path.join(source_dir.trim_start_matches('/'));
    if effective_source != checkout_path {
        let tmp = std::path::Path::new(workspace_dir).join("_src");
        if tmp.exists() {
            let _ = std::fs::remove_dir_all(&tmp);
        }
        std::fs::rename(&checkout_path, &tmp)
            .map_err(|e| AppError::Internal(format!("Failed to move source: {}", e)))?;
        std::fs::rename(&tmp.join(source_dir.trim_start_matches('/')), &checkout_path)
            .map_err(|e| AppError::Internal(format!("Failed to move subdir: {}", e)))?;
        let _ = std::fs::remove_dir_all(&tmp);
    }

    Ok(())
}

pub async fn run_build(workspace_dir: &str, build_cmd: &str, docker: Option<&DockerManager>, username: &str, repo_name: &str) -> Result<String, AppError> {
    if let Some(docker) = docker {
        return docker.exec_build(username, repo_name, build_cmd).await
            .map_err(|e| AppError::Internal(format!("Build failed: {}", e)));
    }

    let source_dir = std::path::Path::new(workspace_dir).join("source");
    tracing::info!("Running build: {} in {:?}", build_cmd, source_dir);

    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(build_cmd)
        .current_dir(&source_dir)
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to run build: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let log = if stdout.is_empty() && stderr.is_empty() {
        format!("$ {}\n", build_cmd)
    } else {
        format!("$ {}\n{}\n{}", build_cmd, stdout, stderr)
    };

    if !output.status.success() {
        return Err(AppError::Internal(format!("Build failed:\n{}", log)));
    }

    Ok(log)
}

pub async fn start_app(manager: &AppProcessManager, repo_id: i64, workspace_dir: &str, start_cmd: &str, port: u16, env_vars: &str, docker_opt: Option<&DockerManager>, username: &str, repo_name: &str) -> Result<(), AppError> {
    if let Some(docker) = docker_opt {
        // Stop existing if any
        let _ = docker.exec_stop_app(username, port).await;

        let mut env: Vec<String> = Vec::new();
        if let Ok(vars) = serde_json::from_str::<HashMap<String, String>>(env_vars) {
            for (k, v) in vars {
                env.push(format!("{}={}", k, v));
            }
        }

        docker.exec_start_detached(username, repo_name, start_cmd, port, Some(env)).await
            .map_err(|e| AppError::Internal(format!("Failed to start app in container: {}", e)))?;

        let proc = AppProcess {
            repo_id,
            port,
            status: AppStatus::Running,
            pid: 0,
        };
        manager.register(proc).await;

        // Health check via container
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        match docker.exec_check_status(username, repo_name, port).await {
            Ok(true) => tracing::info!("App {} is running on port {} in container", repo_id, port),
            _ => tracing::warn!("App {} started but health check failed", repo_id),
        }

        return Ok(());
    }

    let source_dir = std::path::Path::new(workspace_dir).join("source");
    tracing::info!("Starting app {} on port {}", repo_id, port);

    // Stop existing process if any
    stop_app(manager, repo_id, docker_opt).await;

    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c").arg(start_cmd)
        .current_dir(&source_dir)
        .env("PORT", port.to_string())
        .env("HOST", "127.0.0.1");

    // Parse env_vars JSON
    if let Ok(vars) = serde_json::from_str::<HashMap<String, String>>(env_vars) {
        for (k, v) in vars {
            cmd.env(&k, &v);
        }
    }

    let child = cmd.spawn()
        .map_err(|e| AppError::Internal(format!("Failed to start app: {}", e)))?;

    let pid = child.id().unwrap_or(0);

    let proc = AppProcess {
        repo_id,
        port,
        status: AppStatus::Running,
        pid,
    };
    manager.register(proc).await;

    // Monitor child in background
    let manager_clone = manager.clone();
    let repo_id_clone = repo_id;
    tokio::spawn(async move {
        let output = child.wait_with_output().await;
        match output {
            Ok(status) => {
                let msg = if status.status.success() {
                    "exited".to_string()
                } else {
                    format!("exited with code {:?}", status.status.code())
                };
                tracing::warn!("App {} {}: {}", repo_id_clone, msg, String::from_utf8_lossy(&status.stderr));
                manager_clone.update_status(repo_id_clone, AppStatus::Stopped).await;
            }
            Err(e) => {
                tracing::error!("App {} monitor error: {}", repo_id_clone, e);
                manager_clone.update_status(repo_id_clone, AppStatus::Error(e.to_string())).await;
            }
        }
    });

    // Give the process a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Quick health check
    let health_url = format!("http://127.0.0.1:{}/", port);
    match reqwest::get(&health_url).await {
        Ok(_) => tracing::info!("App {} is running on port {}", repo_id, port),
        Err(_) => tracing::warn!("App {} started but health check failed (may need more time)", repo_id),
    }

    Ok(())
}

pub async fn stop_app(manager: &AppProcessManager, repo_id: i64, docker: Option<&DockerManager>) {
    if let Some(proc) = manager.get(repo_id).await {
        if proc.status == AppStatus::Running || proc.status == AppStatus::Deploying {
            tracing::info!("Stopping app {}", repo_id);

            if let Some(docker) = docker {
                let _ = docker.exec_stop_app("", proc.port).await;
            } else {
                let _ = tokio::process::Command::new("kill")
                    .args([&proc.pid.to_string()])
                    .output().await;
                let _ = tokio::process::Command::new("lsof")
                    .args(["-ti", &format!("tcp:{}", proc.port)])
                    .output().await
                    .and_then(|o| {
                        if o.status.success() {
                            let pids = String::from_utf8_lossy(&o.stdout);
                            for pid in pids.lines() {
                                let _ = std::process::Command::new("kill").args(["-9", pid]).output();
                            }
                        }
                        Ok(())
                    });
            }
        }
        manager.update_status(repo_id, AppStatus::Stopped).await;
    }
}

pub async fn deploy_app(
    manager: &AppProcessManager,
    bare_repo_path: &str,
    workspace_dir: &str,
    config: &AppsConfig,
    username: &str,
    repo_name: &str,
    repo_id: i64,
    docker: Option<&DockerManager>,
) -> Result<(u16, String), AppError> {
    manager.update_status(repo_id, AppStatus::Deploying).await;
    let mut log = String::new();

    // Checkout source
    log.push_str(&format!("$ git checkout -f {} -- {}\n---\n", config.branch, config.source_dir));
    let _ = checkout_source(bare_repo_path, workspace_dir, &config.branch, &config.source_dir).await
        .map_err(|e| {
            log.push_str(&format!("Checkout failed: {}\n", e));
            AppError::Internal(format!("Checkout failed: {}", e))
        })?;
    log.push_str("Checkout OK\n\n");

    // Detect project type
    let source_dir = std::path::Path::new(workspace_dir).join("source");
    let project_type = detect_project_type(source_dir.to_str().unwrap())?;
    log.push_str(&format!("Detected: {:?}\n\n", project_type));

    // Resolve commands
    let (build_cmd, start_cmd) = resolve_commands(&project_type, config, source_dir.to_str().unwrap());
    log.push_str(&format!("Build: {}\nStart: {}\n\n", build_cmd, start_cmd));

    // Build
    if !build_cmd.is_empty() {
        match run_build(workspace_dir, &build_cmd, docker, username, repo_name).await {
            Ok(build_log) => log.push_str(&build_log),
            Err(e) => {
                log.push_str(&format!("Build error: {}\n", e));
                return Err(AppError::Internal(format!("Build failed")));
            }
        }
    }

    // Allocate port
    let port = manager.allocate_port().await?;
    log.push_str(&format!("\nPort: {}\n\n", port));

    // Start
    match start_app(manager, repo_id, workspace_dir, &start_cmd, port, &config.env_vars, docker, username, repo_name).await {
        Ok(()) => {
            log.push_str(&format!("App started on port {}\n", port));
            Ok((port, log))
        }
        Err(e) => {
            log.push_str(&format!("Start failed: {}\n", e));
            Err(AppError::Internal(format!("Start failed: {}", e)))
        }
    }
}
