use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};

pub async fn serve_pages(pages_dir: &str, path: &str) -> Response {
    let file_path = std::path::Path::new(pages_dir).join(path);

    let target = if file_path.exists() && file_path.is_file() {
        file_path
    } else if path.is_empty() || path.ends_with('/') {
        let idx = std::path::Path::new(pages_dir).join("index.html");
        if idx.exists() { idx } else { file_path }
    } else {
        file_path
    };

    if !target.exists() || !target.is_file() {
        return (StatusCode::NOT_FOUND, "頁面不存在").into_response();
    }

    match tokio::fs::read(&target).await {
        Ok(content) => {
            let mime = mime_guess::from_path(&target).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(axum::body::Body::from(content))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Err(_) => (StatusCode::NOT_FOUND, "頁面不存在").into_response(),
    }
}
