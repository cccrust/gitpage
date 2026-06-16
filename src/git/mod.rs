use axum::{
    body::Bytes,
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
};
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::Write;

use crate::utils::errors::AppError;

fn git_backend_path() -> &'static str {
    if cfg!(target_os = "windows") { "git.exe" } else { "git" }
}

pub fn init_bare_repo(path: &str) -> Result<(), AppError> {
    let repo_path = Path::new(path);
    if repo_path.exists() {
        return Ok(());
    }
    if let Some(parent) = repo_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let output = Command::new(git_backend_path())
        .args(["init", "--bare"])
        .arg(path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!("Failed to init bare repo: {}", stderr)));
    }

    let output = Command::new(git_backend_path())
        .args(["config", "http.receivepack", "true"])
        .current_dir(path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!("Failed to set http.receivepack: {}", stderr)));
    }

    Ok(())
}

pub fn repo_exists(path: &str) -> bool {
    Path::new(path).join("HEAD").exists()
}

pub fn handle_git_backend(
    method: &Method,
    path_info: &str,
    query_string: Option<&str>,
    content_type: Option<&str>,
    body: Bytes,
    git_root: &str,
    username: &str,
    _repo: &str,
) -> Response {
    let mut cmd = Command::new(git_backend_path());
    cmd.arg("http-backend");

    let full_path = if path_info.starts_with('/') {
        path_info.to_string()
    } else {
        format!("/{}", path_info)
    };

    cmd.env("GIT_PROJECT_ROOT", git_root);
    cmd.env("GIT_HTTP_EXPORT_ALL", "1");
    cmd.env("PATH_INFO", &full_path);
    cmd.env("REQUEST_METHOD", method.as_str());
    cmd.env("QUERY_STRING", query_string.unwrap_or(""));
    cmd.env("REMOTE_USER", username);

    if let Some(ct) = content_type {
        cmd.env("CONTENT_TYPE", ct);
    }

    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("git http-backend spawn error: {}", e)).into_response();
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(&body);
    }

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("git http-backend wait error: {}", e)).into_response();
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("git http-backend stderr: {}", stderr);
    }

    let stdout = output.stdout;
    let stdout_str = String::from_utf8_lossy(&stdout);

    let mut resp_headers = HeaderMap::new();
    let mut body_start = 0;
    let mut status = StatusCode::OK;

    if let Some(header_end) = stdout_str.find("\r\n\r\n") {
        let header_section = &stdout_str[..header_end];
        body_start = header_end + 4;

        for line in header_section.split("\r\n") {
            if line.is_empty() {
                continue;
            }
            if let Some(value) = line.strip_prefix("Status: ") {
                if let Some(code_str) = value.split_whitespace().next() {
                    if let Ok(code) = code_str.parse::<u16>() {
                        status = StatusCode::from_u16(code).unwrap_or(StatusCode::OK);
                    }
                }
            } else if let Some(pos) = line.find(':') {
                let key = line[..pos].trim();
                let val = line[pos + 1..].trim();
                if let (Ok(name), Ok(value)) = (
                    HeaderName::from_bytes(key.as_bytes()),
                    HeaderValue::from_str(val),
                ) {
                    resp_headers.insert(name, value);
                }
            }
        }
    }

    let body_bytes = stdout[body_start..].to_vec();

    let mut builder = Response::builder().status(status);
    for (k, v) in resp_headers.iter() {
        builder = builder.header(k, v);
    }

    builder
        .header("Access-Control-Allow-Origin", "*")
        .body(axum::body::Body::from(body_bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub fn list_refs(repo_path: &str) -> Result<Vec<(String, String)>, AppError> {
    let repo = git2::Repository::open_bare(repo_path)?;
    let mut refs = Vec::new();
    for r in repo.references()? {
        let r = r?;
        if let Some(name) = r.name() {
            if let Some(target) = r.target() {
                refs.push((name.to_string(), target.to_string()));
            }
        }
    }
    Ok(refs)
}

pub fn get_file_content(repo_path: &str, branch: &str, path: &str) -> Result<Option<(Vec<u8>, String)>, AppError> {
    let repo = git2::Repository::open_bare(repo_path)?;

    let branch_ref = format!("refs/heads/{}", branch);
    let oid = match repo.refname_to_id(&branch_ref) {
        Ok(oid) => oid,
        Err(_) => {
            match repo.refname_to_id("HEAD") {
                Ok(oid) => oid,
                Err(_) => return Ok(None),
            }
        }
    };

    let commit = repo.find_commit(oid)?;
    let tree = commit.tree()?;

    let clean_path = path.trim_start_matches('/');
    if clean_path.is_empty() {
        return Ok(None);
    }

    let entry = match tree.get_path(std::path::Path::new(clean_path)) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };

    let obj = entry.to_object(&repo)?;
    let blob = obj.peel_to_blob()?;
    let content = blob.content().to_vec();

    let kind = entry.name()
        .and_then(|n| std::path::Path::new(n).extension())
        .and_then(|ext| ext.to_str())
        .map(|ext| mime_guess::from_ext(ext).first_or_octet_stream().to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    Ok(Some((content, kind)))
}

pub fn list_directory(repo_path: &str, branch: &str, path: &str) -> Result<Vec<(String, bool)>, AppError> {
    let repo = git2::Repository::open_bare(repo_path)?;

    let branch_ref = format!("refs/heads/{}", branch);
    let oid = match repo.refname_to_id(&branch_ref) {
        Ok(oid) => oid,
        Err(_) => {
            match repo.refname_to_id("HEAD") {
                Ok(oid) => oid,
                Err(_) => return Ok(Vec::new()),
            }
        }
    };

    let commit = repo.find_commit(oid)?;
    let tree = commit.tree()?;

    let clean_path = path.trim_start_matches('/');
    let subtree = if clean_path.is_empty() {
        tree
    } else {
        let entry = match tree.get_path(std::path::Path::new(clean_path)) {
            Ok(e) => e,
            Err(_) => return Ok(Vec::new()),
        };
        let obj = entry.to_object(&repo)?;
        obj.peel_to_tree()?
    };

    let mut entries: Vec<(String, bool)> = subtree.iter()
        .filter_map(|entry| {
            let name = String::from_utf8_lossy(entry.name_bytes()).to_string();
            if name.starts_with('.') {
                return None;
            }
            let is_dir = entry.kind() == Some(git2::ObjectType::Tree);
            Some((name, is_dir))
        })
        .collect();

    entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(entries)
}

pub fn get_readme(repo_path: &str, branch: &str) -> Result<Option<String>, AppError> {
    for name in &["README.md", "readme.md", "Readme.md"] {
        if let Ok(Some((content, _))) = get_file_content(repo_path, branch, name) {
            if let Ok(text) = String::from_utf8(content) {
                return Ok(Some(text));
            }
        }
    }
    Ok(None)
}

pub fn deploy_pages(repo_path: &str, output_dir: &str, branch: &str, source_dir: &str) -> Result<(), AppError> {
    let repo = git2::Repository::open_bare(repo_path)?;

    let branch_ref = format!("refs/heads/{}", branch);
    let oid = match repo.refname_to_id(&branch_ref) {
        Ok(oid) => oid,
        Err(_) => return Err(AppError::NotFound(format!("Branch '{}' not found", branch))),
    };

    let commit = repo.find_commit(oid)?;
    let tree = commit.tree()?;

    let clean_source = source_dir.trim_start_matches('/').trim_end_matches('/');
    let subtree: git2::Tree = if clean_source.is_empty() {
        tree
    } else {
        let entry = tree.get_path(std::path::Path::new(clean_source))
            .map_err(|_| AppError::NotFound(format!("Source dir '{}' not found", source_dir)))?;
        let obj = entry.to_object(&repo)?;
        obj.peel_to_tree()?
    };

    // Clean output dir
    let out = std::path::Path::new(output_dir);
    if out.exists() {
        std::fs::remove_dir_all(out)?;
    }
    std::fs::create_dir_all(out)?;

    // Walk tree and write files
    walk_tree_for_pages(&repo, &subtree, out, "")?;

    Ok(())
}

fn walk_tree_for_pages(repo: &git2::Repository, tree: &git2::Tree, out_dir: &std::path::Path, prefix: &str) -> Result<(), AppError> {
    for entry in tree.iter() {
        let name = String::from_utf8_lossy(entry.name_bytes()).to_string();
        let entry_path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", prefix, name)
        };
        let dest = out_dir.join(&entry_path);

        match entry.kind() {
            Some(git2::ObjectType::Tree) => {
                let obj = entry.to_object(repo)?;
                let subtree = obj.peel_to_tree()?;
                std::fs::create_dir_all(&dest)?;
                walk_tree_for_pages(repo, &subtree, out_dir, &entry_path)?;
            }
            Some(git2::ObjectType::Blob) => {
                let obj = entry.to_object(repo)?;
                let blob = obj.peel_to_blob()?;
                std::fs::write(&dest, blob.content())?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Commit staging directory contents to bare repo using git2 TreeBuilder.
/// Commit staging directory contents to bare repo, overlaying on top of the
/// parent tree so existing files are preserved.
pub fn commit_staging(bare_path: &str, staging_path: &str, message: &str, author: &str, branch: &str) -> Result<(), AppError> {
    let repo = git2::Repository::open_bare(bare_path)?;
    let sig = git2::Signature::now(author, "gitpage@localhost")?;

    let branch_ref = format!("refs/heads/{}", branch);

    // Get parent tree if a commit already exists
    let parent_tree: Option<git2::Tree> = match repo.refname_to_id(&branch_ref) {
        Ok(oid) => {
            match repo.find_commit(oid) {
                Ok(c) => c.tree().ok(),
                Err(_) => None,
            }
        }
        Err(_) => None,
    };

    // Build tree starting from parent, overlaying staging files
    let tree_oid = build_tree_from_dir(&repo, staging_path, parent_tree.as_ref())?;
    let tree = repo.find_tree(tree_oid)?;

    // Parent commits
    let parents: Vec<git2::Commit> = match repo.refname_to_id(&branch_ref) {
        Ok(oid) => repo.find_commit(oid).ok().into_iter().collect(),
        Err(_) => Vec::new(),
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    repo.commit(
        Some(&branch_ref),
        &sig, &sig, message, &tree,
        &parent_refs,
    )?;

    // Clear staging directory
    let staging = std::path::Path::new(staging_path);
    if staging.exists() {
        std::fs::remove_dir_all(staging)?;
    }
    std::fs::create_dir_all(staging)?;

    Ok(())
}

/// Build a git tree from a directory. If `parent` is provided, the new tree
/// starts with all entries from the parent, then staging entries are
/// added/updated on top.
fn build_tree_from_dir(repo: &git2::Repository, dir: &str, parent: Option<&git2::Tree>) -> Result<git2::Oid, AppError> {
    let mut tb = repo.treebuilder(parent)?;

    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }
        let path = entry.path();
        if path.is_dir() {
            // Get parent's subtree for this directory so existing files inside are preserved
            let sub_parent = parent
                .and_then(|t| t.get_name(&name))
                .and_then(|e| e.to_object(repo).ok())
                .and_then(|o| o.peel_to_tree().ok());
            let sub_oid = build_tree_from_dir(repo, path.to_str().unwrap(), sub_parent.as_ref())?;
            tb.insert(&name, sub_oid, git2::FileMode::Tree.into())?;
        } else {
            let content = std::fs::read(&path)?;
            let blob_oid = repo.blob(&content)?;
            tb.insert(&name, blob_oid, git2::FileMode::Blob.into())?;
        }
    }
    Ok(tb.write()?)
}

pub fn get_commit_log(repo_path: &str, branch: &str, limit: usize) -> Result<Vec<(String, String, String, String)>, AppError> {
    let repo = git2::Repository::open_bare(repo_path)?;

    let branch_ref = format!("refs/heads/{}", branch);
    let oid = match repo.refname_to_id(&branch_ref) {
        Ok(oid) => oid,
        Err(_) => return Ok(Vec::new()),
    };

    let mut revwalk = repo.revwalk()?;
    revwalk.push(oid)?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let mut commits = Vec::new();
    for (i, oid) in revwalk.enumerate() {
        if i >= limit { break; }
        if let Ok(oid) = oid {
            if let Ok(commit) = repo.find_commit(oid) {
                let sha = oid.to_string();
                let short_sha = sha[..8].to_string();
                let message = commit.message().unwrap_or("").to_string();
                let author = commit.author().name().unwrap_or("unknown").to_string();
                let time = commit.time().seconds();
                let datetime = chrono::DateTime::from_timestamp(time, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default();
                commits.push((short_sha, message, author, datetime));
            }
        }
    }

    Ok(commits)
}
