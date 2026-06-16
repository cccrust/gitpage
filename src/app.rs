use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::auth;
use crate::config::Config;
use crate::db::Database;
use crate::handlers;

const JWT_SECRET: &str = "gitpage-dev-secret-change-in-production";

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: Arc<Config>,
    pub jwt_secret: String,
    pub jwt_expires_hours: u64,
}

async fn auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    let is_cors_preflight = method == axum::http::Method::OPTIONS;
    let is_non_api = !path.starts_with("/api/");
    let is_public_auth = path == "/api/auth/login" || path == "/api/auth/register";
    let is_get_repo_by_id = path.starts_with("/api/repos/") && method == axum::http::Method::GET;
    let is_public_get = path.starts_with("/api/")
        && method == axum::http::Method::GET
        && path != "/api/auth/me"
        && path != "/api/repos";

    let is_public = is_non_api || is_cors_preflight || is_public_auth || is_get_repo_by_id || is_public_get;

    if is_public {
        // Try to extract auth if available (for user context on read-only calls)
        if let Some(header_value) = req.headers().get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
            if let Some(token) = header_value.strip_prefix("Bearer ") {
                if let Ok(claims) = auth::verify_token(token, JWT_SECRET) {
                    let mut req = req;
                    req.extensions_mut().insert(claims.sub);
                    req.extensions_mut().insert(claims.username.clone());
                    return Ok(next.run(req).await);
                }
            }
        }
        return Ok(next.run(req).await);
    }

    // Protected paths require auth
    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    if let Some(header_value) = auth_header {
        if let Some(token) = header_value.strip_prefix("Bearer ") {
            if let Ok(claims) = auth::verify_token(token, JWT_SECRET) {
                let mut req = req;
                req.extensions_mut().insert(claims.sub);
                req.extensions_mut().insert(claims.username.clone());
                return Ok(next.run(req).await);
            }
        }
    }

    Err((StatusCode::UNAUTHORIZED, "需要登入".to_string()))
}

pub fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/register", post(handlers::auth::register))
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/me", get(handlers::auth::me))
        .route("/api/repos", get(handlers::repos::list_user_repos))
        .route("/api/repos", post(handlers::repos::create_repo))
        .route("/api/repos/:id", get(handlers::repos::get_repo_by_id))
        .route("/api/repos/:id", delete(handlers::repos::delete_repo))
        .route("/api/users/:username/repos", get(handlers::repos::list_public_repos))
        .route("/api/:username/:repo_name/tree", get(handlers::content::list_directory))
        .route("/api/:username/:repo_name/blob", get(handlers::content::get_file_content))
        .route("/api/:username/:repo_name/readme", get(handlers::content::get_readme))
        .route("/api/:username/:repo_name/commits/:branch", get(handlers::content::list_commits))
        .route("/api/users/:username/profile", get(handlers::auth::get_user_profile))
        .route("/api/users/:username/profile", put(handlers::auth::update_profile))
        .route("/api/repos/search", get(handlers::repos::search_repos))
        .route("/api/repos/:id", put(handlers::repos::update_repo_handler))
        .route("/api/pages/:repo_id", get(handlers::pages::get_pages_config))
        .route("/api/pages/:repo_id", put(handlers::pages::update_pages_config))
        .route("/api/pages/:repo_id/deploy", post(handlers::pages::deploy_pages_handler))
        .fallback(fallback_handler)
        .layer(middleware::from_fn(auth_middleware))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn fallback_handler(
    State(state): State<AppState>,
    req: Request,
) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().clone();


    // Git HTTP Smart Protocol: /git/{username}/{repo_name}/{*path}
    if let Some(rest) = path.strip_prefix("/git/") {
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        if parts.len() >= 2 {
            let username = parts[0];
            let repo_name = parts[1];
            let subpath = parts.get(2).copied().unwrap_or("");
            let repo_path = state.config.repo_path(username, repo_name);

            if !crate::git::repo_exists(&repo_path) {
                return (StatusCode::NOT_FOUND, "Repository not found").into_response();
            }

            let path_info = format!("/{}/{}.git/{}", username, repo_name, subpath);
            let query = req.uri().query().map(|s| s.to_string());
            let git_root = state.config.storage.base_path.clone();
            let content_type = req.headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            let is_push = content_type.as_deref() == Some("application/x-git-receive-pack-request");

            let body_bytes = axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await
                .unwrap_or_default();

            let resp = crate::git::handle_git_backend(
                &method,
                &path_info,
                query.as_deref(),
                content_type.as_deref(),
                body_bytes,
                &git_root,
                username,
                repo_name,
            );

            // Auto-deploy pages on successful push
            if is_push && resp.status().is_success() {
                tokio::spawn(auto_deploy_pages(
                    state.clone(),
                    username.to_string(),
                    repo_name.to_string(),
                ));
            }

            return resp;
        }
    }

    // Pages serving: /pages/{username}/{repo_name}/{*path}
    if let Some(rest) = path.strip_prefix("/pages/") {
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        if parts.len() >= 2 {
            let username = parts[0];
            let repo_name = parts[1];
            let subpath = parts.get(2).copied().unwrap_or("");

            let pages_dir = format!("{}/{}/{}/pages", state.config.storage.base_path, username, repo_name);
            return handlers::git_smart::serve_pages(&pages_dir, subpath).await;
        }
    }

    // Serve static files from 'static/' or 'frontend/dist/'
    for base_dir in &["frontend/dist", "static"] {
        let dir = std::path::Path::new(base_dir);
        let file_path = dir.join(path.strip_prefix('/').unwrap_or(""));
        if file_path.exists() && file_path.is_file() {
            match tokio::fs::read(&file_path).await {
                Ok(content) => {
                    let mime = mime_guess::from_path(&file_path).first_or_octet_stream();
                    return Response::builder()
                        .header(header::CONTENT_TYPE, mime.as_ref())
                        .body(Body::from(content))
                        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
                }
                Err(_) => {}
            }
        }

        // SPA fallback: serve index.html for non-file requests
        if !path.contains('.') {
            let index_path = dir.join("index.html");
            if index_path.exists() {
                match tokio::fs::read(&index_path).await {
                    Ok(content) => {
                        return Response::builder()
                            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                            .body(Body::from(content))
                            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
                    }
                    Err(_) => {}
                }
            }
        }
    }

    (StatusCode::NOT_FOUND, "Not found").into_response()
}

async fn auto_deploy_pages(state: AppState, username: String, repo_name: String) {
    let repo_path = state.config.repo_path(&username, &repo_name);
    let pages_dir = state.config.pages_dir(&username, &repo_name);

    // Find the repo in DB
    let user = match state.db.find_user_by_username(&username).await {
        Ok(Some(u)) => u,
        _ => return,
    };
    let repo = match state.db.find_repo_by_name(user.id, &repo_name).await {
        Ok(Some(r)) => r,
        _ => return,
    };

    let cfg = match state.db.get_pages_config(repo.id).await {
        Ok(Some(c)) if c.enabled => c,
        _ => return,
    };

    let _ = crate::git::deploy_pages(&repo_path, &pages_dir, &cfg.branch, &cfg.source_dir);
    tracing::info!("Pages deployed for {}/{}", username, repo_name);
}
