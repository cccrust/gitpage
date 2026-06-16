mod app;
mod auth;
mod config;
mod db;
mod git;
mod handlers;
mod utils;

use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = config::Config::from_file("config.toml");
    let cfg = Arc::new(cfg);

    std::fs::create_dir_all(&cfg.storage.base_path).expect("Failed to create storage directory");
    std::fs::create_dir_all("static").expect("Failed to create static directory");

    let db = db::Database::new(&cfg.database.path).expect("Failed to open database");
    db.run_migrations().await.expect("Failed to run migrations");

    let jwt_secret = cfg.jwt.secret.clone();
    let jwt_expires_hours = cfg.jwt.expires_in_hours;

    let state = app::AppState {
        db,
        config: cfg.clone(),
        jwt_secret,
        jwt_expires_hours,
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
