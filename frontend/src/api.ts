const BASE = ''

function getToken(): string | null {
  return localStorage.getItem('token')
}

export function setToken(t: string) {
  localStorage.setItem('token', t)
}

export function clearToken() {
  localStorage.removeItem('token')
}

export function isLoggedIn(): boolean {
  return !!getToken()
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' }
  const token = getToken()
  if (token) headers['Authorization'] = `Bearer ${token}`

  const res = await fetch(`${BASE}${path}`, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  })

  if (!res.ok) {
    const err = await res.text()
    let msg: string
    try {
      msg = JSON.parse(err).error || err
    } catch {
      msg = err || `HTTP ${res.status}`
    }
    throw new Error(msg)
  }

  return res.json()
}

export interface User {
  id: number
  username: string
  bio: string
  avatar_url: string
  created_at: string
}

export interface Repo {
  id: number
  user_id: number
  name: string
  description: string
  is_private: boolean
  default_branch: string
  created_at: string
  updated_at: string
}

export interface TreeEntry {
  name: string
  is_dir: boolean
}

export interface BlobData {
  content: string
  mime_type: string
  is_markdown: boolean
  rendered: string | null
  repo: Repo
  branch: string
  path: string
}

export interface CommitInfo {
  sha: string
  message: string
  author: string
  time: string
}

// ── Auth ──

export function register(username: string, email: string, password: string) {
  return request<{ token: string; user: User }>('POST', '/api/auth/register', { username, email, password })
}

export function login(username: string, password: string) {
  return request<{ token: string; user: User }>('POST', '/api/auth/login', { username, password })
}

export function me() {
  return request<{ user: User }>('GET', '/api/auth/me')
}

// ── Repos ──

export function listRepos() {
  return request<{ repos: Repo[] }>('GET', '/api/repos')
}

export function createRepo(name: string, description?: string, is_private?: boolean) {
  return request<{ repo: Repo }>('POST', '/api/repos', { name, description, is_private })
}

export function getRepo(id: number) {
  return request<{ repo: Repo; username: string }>('GET', `/api/repos/${id}`)
}

export function deleteRepo(id: number) {
  return request<{ deleted: boolean }>('DELETE', `/api/repos/${id}`)
}

export function listPublicRepos(username: string) {
  return request<{ repos: Repo[]; user: string }>('GET', `/api/users/${username}/repos`)
}

// ── Content ──

export function listTree(username: string, repo: string, branch?: string, path?: string) {
  const params = new URLSearchParams()
  if (branch) params.set('branch', branch)
  if (path) params.set('path', path)
  return request<{ entries: TreeEntry[]; repo: Repo; branch: string; path: string }>(
    'GET', `/api/${username}/${repo}/tree?${params}`,
  )
}

export function getBlob(username: string, repo: string, branch: string, path: string) {
  const params = new URLSearchParams({ branch, path })
  return request<BlobData>('GET', `/api/${username}/${repo}/blob?${params}`)
}

export function getReadme(username: string, repo: string, branch?: string) {
  const params = new URLSearchParams()
  if (branch) params.set('branch', branch)
  return request<{ content?: string; rendered?: string; has_readme: boolean }>(
    'GET', `/api/${username}/${repo}/readme?${params}`,
  )
}

export function listCommits(username: string, repo: string, branch: string) {
  return request<{ commits: CommitInfo[]; repo: Repo; branch: string }>(
    'GET', `/api/${username}/${repo}/commits/${branch}`,
  )
}

// ── Pages ──

export interface PagesConfig {
  id: number
  repo_id: number
  branch: string
  source_dir: string
  custom_domain: string
  enabled: boolean
}

export function getPagesConfig(repoId: number) {
  return request<{ pages_config: PagesConfig | null }>('GET', `/api/pages/${repoId}`)
}

export function updatePagesConfig(repoId: number, data: { branch?: string; source_dir?: string; custom_domain?: string; enabled?: boolean }) {
  return request<{ success: boolean; deploy_error?: string }>('PUT', `/api/pages/${repoId}`, data)
}

export function deployPages(repoId: number) {
  return request<{ success: boolean; pages_dir: string }>('POST', `/api/pages/${repoId}/deploy`)
}

// ── Apps ──

export interface AppsConfig {
  id: number
  repo_id: number
  branch: string
  source_dir: string
  build_command: string
  start_command: string
  env_vars: string
  enabled: boolean
}

export interface AppStatusResponse {
  apps_config: AppsConfig | null
  status: string | null
  port: number | null
  url: string | null
}

export function getAppsConfig(repoId: number) {
  return request<AppStatusResponse>('GET', `/api/apps/${repoId}`)
}

export function updateAppsConfig(repoId: number, data: {
  branch?: string; source_dir?: string; build_command?: string;
  start_command?: string; env_vars?: string; enabled?: boolean
}) {
  return request<{ success: boolean; port?: number; deploy_error?: string }>('PUT', `/api/apps/${repoId}`, data)
}

export function deployApps(repoId: number) {
  return request<{ success: boolean; port: number; url: string }>('POST', `/api/apps/${repoId}/deploy`)
}

export function deleteAppsConfig(repoId: number) {
  return request<{ success: boolean }>('DELETE', `/api/apps/${repoId}`)
}
