# Gitpage API Reference

Base URL: `http://localhost:8080`

Auth: `Authorization: Bearer <jwt-token>`

## Auth

### `POST /api/auth/register`
```json
{"username": "alice", "email": "alice@test.com", "password": "pass123"}
```
→ `201` `{"token": "<jwt>", "user": {id, username, bio, avatar_url, created_at}}`

### `POST /api/auth/login`
```json
{"username": "alice", "password": "pass123"}
```
→ `200` `{"token": "<jwt>", "user": {...}}`

### `GET /api/auth/me`
→ `200` `{"user": {...}}`

### `PUT /api/auth/password`
```json
{"current_password": "...", "new_password": "..."}
```
→ `200` `{"success": true}`

## Repos

### `GET /api/repos` (auth)
→ `200` `{"repos": [{id, user_id, name, description, is_private, default_branch, created_at, updated_at}]}`

### `POST /api/repos` (auth)
```json
{"name": "myrepo", "description": "...", "is_private": false}
```
→ `201` `{"repo": {...}}`

### `GET /api/repos/:id`
→ `200` `{"repo": {...}, "username": "..."}`

### `PUT /api/repos/:id` (auth)
```json
{"name": "newname", "description": "...", "is_private": true}
```
Renames bare repo + staging dirs on disk. → `200` `{"success": true}`

### `DELETE /api/repos/:id` (auth)
→ `200` `{"deleted": true}`

### `GET /api/repos/search?q=<query>&page=1&page_size=10`
→ `200` `{"repos": [{..., username}], "total": N, "page": 1, "page_size": 10, "total_pages": N, "query": "..."}`

## Users

### `GET /api/users/:username/repos`
→ `200` `{"repos": [...], "user": "..."}`

### `GET /api/users/:username/profile`
→ `200` `{"user": {id, username, bio, avatar_url, created_at}, "repos": [...]}`

### `PUT /api/users/:username/profile` (auth)
```json
{"bio": "...", "avatar_url": "..."}
```
→ `200` `{"success": true}`

## Repo Content

### `GET /api/:username/:repo_name/tree?branch=main&path=src`
→ `200` `{"entries": [{name, is_dir}], "repo": {...}, "branch": "main", "path": "src"}`

### `GET /api/:username/:repo_name/blob?branch=main&path=README.md`
→ `200` `{"content": "..." , "mime_type": "...", "is_markdown": bool, "rendered": "<html>", "repo": {...}, "branch": "main", "path": "README.md"}`

### `GET /api/:username/:repo_name/readme?branch=main`
→ `200` `{"has_readme": bool, "content": "...", "rendered": "<html>"}`

### `GET /api/:username/:repo_name/commits/:branch`
→ `200` `{"commits": [{sha, message, author, time}], "repo": {...}, "branch": "main"}`

## Pages

### `GET /api/pages/:repo_id`
→ `200` `{"pages_config": {id, repo_id, branch, source_dir, custom_domain, enabled} | null}`

### `PUT /api/pages/:repo_id` (auth)
```json
{"branch": "main", "source_dir": "/", "custom_domain": "", "enabled": true}
```
→ `200` `{"success": true, "deploy_error": "..."}`

### `POST /api/pages/:repo_id/deploy` (auth)
→ `200` `{"success": true, "pages_dir": "..."}`

## Apps

### `GET /api/apps/:repo_id`
→ `200` `{"apps_config": {...} | null, "status": "running"|null, "port": 4000|null, "url": "..."|null}`

### `PUT /api/apps/:repo_id` (auth)
```json
{"branch": "main", "source_dir": "/", "build_command": "npm run build", "start_command": "npm start", "env_vars": "{}", "enabled": true}
```
→ `200` `{"success": true, "port": 4000, "deploy_error": "..."}`

### `DELETE /api/apps/:repo_id` (auth)
→ `200` `{"success": true}`

### `POST /api/apps/:repo_id/deploy` (auth)
→ `200` `{"success": true, "port": 4000, "url": "http://..."}`

### `GET /api/apps/:repo_id/deploys`
→ `200` `{"deploy_logs": [{id, repo_id, status, started_at, finished_at, log_output}]}`

### `GET /api/apps/:repo_id/deploys/:deploy_id`
→ `200` `{"deploy_log": {...}}`

## Staging Files (File Manager)

### `GET /api/repos/:repo_id/tree?path=src`
→ `200` `{"entries": [{name, is_dir, size, updated_at}], "path": "src"}`

### `GET /api/repos/:repo_id/raw?path=README.md`
Raw file content (response is the file body, not JSON)

### `PUT /api/repos/:repo_id/files?path=README.md`
Body: file content (raw). → `200` `{"success": true, "path": "README.md"}`

### `DELETE /api/repos/:repo_id/files?path=file.txt`
→ `200` `{"success": true}`

### `POST /api/repos/:repo_id/mkdir?path=newdir`
→ `200` `{"success": true}`

### `POST /api/repos/:repo_id/move?from=old&to=new`
→ `200` `{"success": true}`

### `GET /api/repos/:repo_id/status`
→ `200` `{"pending": bool, "changes": [{path, change_type}]}`

### `POST /api/repos/:repo_id/commit` (auth)
```json
{"message": "commit message"}
```
→ `200` `{"success": true}`

## SSH Keys

### `GET /api/repos/:repo_id/ssh-keys` (auth)
→ `200` `{"ssh_keys": [{id, user_id, repo_id, name, public_key, created_at}]}`

### `POST /api/repos/:repo_id/ssh-keys` (auth)
```json
{"name": "my-laptop", "public_key": "ssh-ed25519 ..."}
```
→ `200` `{"success": true, "ssh_key": {...}}`

### `DELETE /api/repos/:repo_id/ssh-keys/:key_id` (auth)
→ `200` `{"success": true}`

## Non-API Endpoints

| Path | Description |
|------|-------------|
| `GET/POST /git/{user}/{repo}/{*path}` | Git HTTP Smart Protocol |
| `GET /pages/{user}/{repo}/{*path}` | Static Pages hosting |
| `GET /app/{user}/{repo}/{*path}` | App reverse proxy |
| `/*` | SPA fallback (frontend/dist or static/) |

## Errors

All errors return JSON: `{"error": "<中文訊息>"}`

| Status | Meaning |
|--------|---------|
| 400 | Bad request |
| 401 | Unauthorized |
| 404 | Not found |
| 409 | Conflict (e.g. duplicate) |
| 500 | Internal error |
