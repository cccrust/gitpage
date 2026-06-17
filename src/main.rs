mod app;
mod auth;
mod config;
mod db;
mod deploy;
mod docker;
mod git;
mod handlers;
mod ssh;
mod utils;

use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config_path = std::env::args().nth(1).unwrap_or_else(|| "config.toml".to_string());
    let cfg = config::Config::from_file(&config_path);
    let cfg = Arc::new(cfg);

    std::fs::create_dir_all(&cfg.storage.base_path).expect("Failed to create storage directory");
    std::fs::create_dir_all(format!("{}/repos", cfg.storage.base_path)).expect("Failed to create repos directory");
    std::fs::create_dir_all(format!("{}/apps", cfg.storage.base_path)).expect("Failed to create apps directory");
    std::fs::create_dir_all(format!("{}/staging", cfg.storage.base_path)).expect("Failed to create staging directory");

    // Setup SSH: write gitpage-shell handler script to ~/.ssh/
    let staging_root = std::env::current_dir().expect("Failed to get current dir").join("data/staging");
    if cfg.ssh.enabled {
        let ssh_dir_path = crate::ssh::ssh_dir();
        std::fs::create_dir_all(&ssh_dir_path).expect("Failed to create ~/.ssh directory");

        let shell_script = format!(
            r#"#!/bin/bash
USERNAME="$1"
REPO_NAME="$2"
STAGING_DIR="{root}/$USERNAME/$REPO_NAME"
if [ ! -d "$STAGING_DIR" ]; then
    echo "ERROR: Staging directory not found: $STAGING_DIR"
    exit 1
fi
cd "$STAGING_DIR" || exit 1
if [ -n "$SSH_ORIGINAL_COMMAND" ]; then
    exec bash -c "$SSH_ORIGINAL_COMMAND"
else
    exec bash -i
fi
"#,
            root = staging_root.display()
        );

        let shell_script_path = std::path::Path::new(&ssh_dir_path).join("gitpage-shell");
        if !shell_script_path.exists() {
            std::fs::write(&shell_script_path, &shell_script).expect("Failed to write gitpage-shell script");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&shell_script_path, std::fs::Permissions::from_mode(0o755))
                    .expect("Failed to make gitpage-shell executable");
            }
            tracing::info!("Wrote SSH handler script to {:?}", shell_script_path);
        }
    }

    std::fs::create_dir_all("static").expect("Failed to create static directory");

    let db = db::Database::new(&cfg.database.path).expect("Failed to open database");
    db.run_migrations().await.expect("Failed to run migrations");

    let jwt_secret = cfg.jwt.effective_secret();
    auth::init_jwt_secret(jwt_secret);

    let app_manager = deploy::AppProcessManager::new(
        cfg.apps.port_range_start,
        cfg.apps.port_range_end,
    );

    // Initialize Docker manager if runtime mode is "docker"
    let docker = if cfg.runtime.mode == "docker" {
        match crate::docker::DockerManager::connect(&cfg).await {
            Ok(m) => {
                tracing::info!("Docker manager initialized");
                Some(m)
            }
            Err(e) => {
                tracing::warn!("Failed to initialize Docker manager, falling back to process mode: {}", e);
                None
            }
        }
    } else {
        None
    };

    let state = app::AppState {
        db,
        config: cfg.clone(),
        jwt_expires_hours: cfg.jwt.expires_in_hours,
        app_manager,
        docker,
    };

    let app = app::create_app(state);

    let addr = format!("{}:{}", cfg.server.host, cfg.server.port);
    tracing::info!("gitpage server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
