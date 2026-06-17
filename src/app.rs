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
use crate::deploy::AppProcessManager;
use crate::handlers;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: Arc<Config>,
    pub jwt_expires_hours: u64,
    pub app_manager: AppProcessManager,
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
                if let Ok(claims) = auth::verify_token(token) {
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
            if let Ok(claims) = auth::verify_token(token) {
                let mut req = req;
                req.extensions_mut().insert(claims.sub);
                req.extensions_mut().insert(claims.username.clone());
                return Ok(next.run(req).await);
            }
        }
    }

    Err((StatusCode::UNAUTHORIZED, "需要登入".to_string()))
}

fn build_cors_layer(cfg: &crate::config::CorsConfig) -> CorsLayer {
    if cfg.allowed_origins.contains(&"*".to_string()) {
        return CorsLayer::permissive();
    }
    let origins: Vec<_> = cfg.allowed_origins.iter()
        .filter_map(|o| o.parse::<axum::http::HeaderValue>().ok())
        .collect();
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE, axum::http::Method::OPTIONS])
        .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION])
}

pub fn create_app(state: AppState) -> Router {
    let cors = build_cors_layer(&state.config.cors);
    Router::new()
        .route("/api/auth/register", post(handlers::auth::register))
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/me", get(handlers::auth::me))
        .route("/api/auth/password", put(handlers::auth::change_password))
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
        .route("/api/apps/:repo_id", get(handlers::apps::get_apps_config))
        .route("/api/apps/:repo_id", put(handlers::apps::update_apps_config))
        .route("/api/apps/:repo_id", delete(handlers::apps::delete_apps_handler))
        .route("/api/apps/:repo_id/deploy", post(handlers::apps::deploy_apps_handler))
        .route("/api/apps/:repo_id/deploys", get(handlers::apps::list_deploys))
        .route("/api/apps/:repo_id/deploys/:deploy_id", get(handlers::apps::get_deploy_log))
        .route("/api/repos/:repo_id/tree", get(handlers::files::tree))
        .route("/api/repos/:repo_id/raw", get(handlers::files::raw))
        .route("/api/repos/:repo_id/files", put(handlers::files::write_file))
        .route("/api/repos/:repo_id/files", delete(handlers::files::delete_file))
        .route("/api/repos/:repo_id/mkdir", post(handlers::files::mkdir))
        .route("/api/repos/:repo_id/move", post(handlers::files::move_file))
        .route("/api/repos/:repo_id/status", get(handlers::files::status))
        .route("/api/repos/:repo_id/commit", post(handlers::files::commit))
        .route("/api/repos/:repo_id/ssh-keys", get(handlers::ssh_keys::list_keys))
        .route("/api/repos/:repo_id/ssh-keys", post(handlers::ssh_keys::add_key))
        .route("/api/repos/:repo_id/ssh-keys/:key_id", delete(handlers::ssh_keys::delete_key))

        // Org routes
        .route("/api/orgs", get(handlers::orgs::list_my_orgs))
        .route("/api/orgs", post(handlers::orgs::create_org))
        .route("/api/orgs/:name", get(handlers::orgs::get_org))
        .route("/api/orgs/:name", put(handlers::orgs::update_org))
        .route("/api/orgs/:name", delete(handlers::orgs::delete_org))
        .route("/api/orgs/:name/repos", get(handlers::orgs::list_org_repos))
        .route("/api/orgs/:name/members", get(handlers::orgs::list_members))
        .route("/api/orgs/:name/members", post(handlers::orgs::add_member))
        .route("/api/orgs/:name/members/:user_id", delete(handlers::orgs::remove_member))

        .fallback(fallback_handler)
        .layer(middleware::from_fn(auth_middleware))
        .layer(cors)
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
                return (StatusCode::NOT_FOUND, "倉庫不存在").into_response();
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

            // Auto-deploy pages and apps on successful push
            if is_push && resp.status().is_success() {
                tokio::spawn(auto_deploy_pages(
                    state.clone(),
                    username.to_string(),
                    repo_name.to_string(),
                ));
                tokio::spawn(auto_deploy_app(
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

    // App proxy: /app/{username}/{repo_name}/{*path}
    if let Some(rest) = path.strip_prefix("/app/") {
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        if parts.len() >= 2 {
            let username = parts[0];
            let repo_name = parts[1];
            let subpath = parts.get(2).copied().unwrap_or("");

            let repo = match resolve_owner_and_repo(&state.db, username, repo_name).await {
                Some((r, _)) => r,
                _ => return (StatusCode::NOT_FOUND, "倉庫不存在").into_response(),
            };

            if let Some(proc) = state.app_manager.get(repo.id).await {
                if proc.status == crate::deploy::AppStatus::Running {
                    let url = format!("http://127.0.0.1:{}/{}", proc.port, subpath);

                    let client = reqwest::Client::new();
                    let body_bytes = axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await.unwrap_or_default();

                    let reqwest_req = client
                        .request(
                            reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET),
                            &url,
                        )
                        .body(body_bytes.to_vec())
                        .build()
                        .unwrap();

                    let reqwest_resp = match client.execute(reqwest_req).await {
                        Ok(r) => r,
                        Err(e) => return (StatusCode::BAD_GATEWAY, format!("代理錯誤: {}", e)).into_response(),
                    };

                    let status = reqwest_resp.status();
                    let content_type = reqwest_resp
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("text/html; charset=utf-8")
                        .to_string();
                    let resp_body = match reqwest_resp.bytes().await {
                        Ok(b) => b,
                        Err(_) => return (StatusCode::BAD_GATEWAY, "讀取上游回應失敗").into_response(),
                    };

                    let mut resp = Response::builder().status(status);
                    resp = resp.header(header::CONTENT_TYPE, content_type);
                    return resp
                        .body(Body::from(resp_body.to_vec()))
                        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
                }
            }
            return (StatusCode::BAD_GATEWAY, "App 未在執行").into_response();
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

    (StatusCode::NOT_FOUND, "找不到頁面").into_response()
}

async fn resolve_owner_and_repo(db: &crate::db::Database, owner_name: &str, repo_name: &str) -> Option<(crate::db::models::Repository, String)> {
    // Try user first
    if let Ok(Some(user)) = db.find_user_by_username(owner_name).await {
        if let Ok(Some(repo)) = db.find_repo_by_name(user.id, repo_name).await {
            return Some((repo, user.username));
        }
    }
    // Try org
    if let Ok(Some(org)) = db.find_org_by_name(owner_name).await {
        if let Ok(Some(repo)) = db.find_org_repo_by_name(org.id, repo_name).await {
            return Some((repo, org.name));
        }
    }
    None
}

pub(crate) async fn auto_deploy_pages(state: AppState, owner_name: String, repo_name: String) {
    let repo_path = state.config.repo_path(&owner_name, &repo_name);
    let pages_dir = state.config.pages_dir(&owner_name, &repo_name);

    let (repo, _) = match resolve_owner_and_repo(&state.db, &owner_name, &repo_name).await {
        Some(r) => r,
        _ => return,
    };

    let cfg = match state.db.get_pages_config(repo.id).await {
        Ok(Some(c)) if c.enabled => c,
        _ => return,
    };

    let _ = crate::git::deploy_pages(&repo_path, &pages_dir, &cfg.branch, &cfg.source_dir);
    tracing::info!("Pages deployed for {}/{}", owner_name, repo_name);
}

pub(crate) async fn auto_deploy_app(state: AppState, owner_name: String, repo_name: String) {
    let (repo, _) = match resolve_owner_and_repo(&state.db, &owner_name, &repo_name).await {
        Some(r) => r,
        _ => return,
    };

    let cfg = match state.db.get_apps_config(repo.id).await {
        Ok(Some(c)) if c.enabled => c,
        _ => return,
    };

    let repo_path = state.config.repo_path(&owner_name, &repo_name);
    let workspace = state.config.app_workspace_dir(&owner_name, &repo_name);

    let deploy_log = match state.db.create_deploy_log(repo.id).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to create deploy log: {}", e);
            return;
        }
    };

    match crate::deploy::deploy_app(
        &state.app_manager,
        &repo_path,
        &workspace,
        &cfg,
        &owner_name,
        &repo_name,
        repo.id,
    ).await {
        Ok((port, log)) => {
            let _ = state.db.update_deploy_log(deploy_log.id, "success", &log).await;
            tracing::info!("App deployed for {}/{} on port {}", owner_name, repo_name, port);
        }
        Err(e) => {
            let log = format!("Deploy failed: {}", e);
            let _ = state.db.update_deploy_log(deploy_log.id, "failed", &log).await;
            tracing::error!("App deploy failed for {}/{}: {}", owner_name, repo_name, e);
        }
    }
}
