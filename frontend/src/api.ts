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
  username?: string
  name: string
  description: string
  is_private: boolean
  default_branch: string
  owner_type?: string
  org_id?: number | null
  stars_count: number
  forks_count: number
  watch_count: number
  created_at: string
  updated_at: string
  forked_from?: number | null
}

export interface SearchResult {
  repos: Repo[]
  total: number
  page: number
  page_size: number
  total_pages: number
  query: string
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

export interface Organization {
  id: number
  name: string
  display_name: string
  description: string
  owner_id: number
  created_at: string
}

export interface OrganizationWithRole extends Organization {
  role: string
}

export interface OrgMember {
  id: number
  user_id: number
  org_id: number
  role: string
  username: string
  bio: string
  created_at: string
}

// ── Organizations ──

export function listMyOrgs() {
  return request<{ orgs: OrganizationWithRole[] }>('GET', '/api/orgs')
}

export function createOrg(name: string, displayName: string, description: string) {
  return request<{ org: Organization }>('POST', '/api/orgs', { name, display_name: displayName, description })
}

export function getOrg(name: string) {
  return request<{ org: Organization; role?: string }>('GET', `/api/orgs/${name}`)
}

export function updateOrg(name: string, data: { display_name?: string; description?: string }) {
  return request<{ org: Organization }>('PUT', `/api/orgs/${name}`, data)
}

export function deleteOrg(name: string) {
  return request<{ deleted: boolean }>('DELETE', `/api/orgs/${name}`)
}

export function listOrgRepos(name: string) {
  return request<{ repos: Repo[]; org: Organization }>('GET', `/api/orgs/${name}/repos`)
}

export function listOrgMembers(name: string) {
  return request<{ members: OrgMember[]; org: Organization }>('GET', `/api/orgs/${name}/members`)
}

export function addOrgMember(name: string, username: string, role?: string) {
  return request<{ success: boolean; member: OrgMember }>('POST', `/api/orgs/${name}/members`, { username, role })
}

export function removeOrgMember(name: string, userId: number) {
  return request<{ success: boolean }>('DELETE', `/api/orgs/${name}/members/${userId}`)
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

export function createRepo(name: string, description?: string, is_private?: boolean, orgName?: string) {
  return request<{ repo: Repo }>('POST', '/api/repos', { name, description, is_private, org_name: orgName })
}

export function getRepo(id: number) {
  return request<{ repo: Repo; username: string; org_name?: string }>('GET', `/api/repos/${id}`)
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

// ── Working Tree (File Manager) ──

export interface FileEntry {
  name: string
  is_dir: boolean
  size: number | null
  updated_at: string
}

export interface WorkingTreeChange {
  path: string
  change_type: string
}

export function listWorkingTree(repoId: number, path?: string) {
  const params = new URLSearchParams()
  if (path) params.set('path', path)
  return request<{ entries: FileEntry[]; path: string }>('GET', `/api/repos/${repoId}/tree?${params}`)
}

export async function getRawFile(repoId: number, path: string): Promise<Response> {
  const headers: Record<string, string> = {}
  const token = getToken()
  if (token) headers['Authorization'] = `Bearer ${token}`
  return fetch(`${BASE}/api/repos/${repoId}/raw?path=${encodeURIComponent(path)}`, { headers })
}

export async function writeFile(repoId: number, path: string, content: string | Blob) {
  const headers: Record<string, string> = {}
  const token = getToken()
  if (token) headers['Authorization'] = `Bearer ${token}`
  const body = typeof content === 'string' ? content : content
  const res = await fetch(`${BASE}/api/repos/${repoId}/files?path=${encodeURIComponent(path)}`, {
    method: 'PUT',
    headers,
    body,
  })
  if (!res.ok) {
    const err = await res.text()
    throw new Error(err || `HTTP ${res.status}`)
  }
  return res.json()
}

export function deleteFile(repoId: number, path: string) {
  return request<{ success: boolean }>('DELETE', `/api/repos/${repoId}/files?path=${encodeURIComponent(path)}`)
}

export function mkdir(repoId: number, path: string) {
  return request<{ success: boolean }>('POST', `/api/repos/${repoId}/mkdir?path=${encodeURIComponent(path)}`)
}

export function moveFile(repoId: number, from: string, to: string) {
  const params = new URLSearchParams({ from, to })
  return request<{ success: boolean }>('POST', `/api/repos/${repoId}/move?${params}`)
}

export function getStatus(repoId: number) {
  return request<{ pending: boolean; changes: WorkingTreeChange[] }>('GET', `/api/repos/${repoId}/status`)
}

export function commitRepo(repoId: number, message: string) {
  return request<{ success: boolean }>('POST', `/api/repos/${repoId}/commit`, { message })
}

// ── Deploy Logs ──

export interface DeployLog {
  id: number
  repo_id: number
  status: string
  started_at: string
  finished_at: string | null
  log_output: string
}

export function listDeploys(repoId: number) {
  return request<{ deploy_logs: DeployLog[] }>('GET', `/api/apps/${repoId}/deploys`)
}

export function getDeployLog(repoId: number, deployId: number) {
  return request<{ deploy_log: DeployLog }>('GET', `/api/apps/${repoId}/deploys/${deployId}`)
}

// ── v2.0 Issues & PRs ──

export interface Issue {
  id: number
  repo_id: number
  number: number
  title: string
  body: string | null
  state: string
  author_id: number
  assignee_id: number | null
  created_at: string
  updated_at: string
  closed_at: string | null
}

export interface IssueWithAuthor {
  issue: Issue
  author_username: string
  labels: IssueLabel[]
}

export interface IssueLabel {
  id: number
  repo_id: number
  name: string
  color: string
}

export interface IssueComment {
  id: number
  issue_id: number
  author_id: number
  author_username: string
  body: string
  created_at: string
  updated_at: string
}

export interface PullRequest {
  id: number
  repo_id: number
  number: number
  title: string
  body: string | null
  state: string
  author_id: number
  head_repo_id: number
  head_ref: string
  base_ref: string
  merge_commit_sha: string | null
  created_at: string
  updated_at: string
  closed_at: string | null
  merged_at: string | null
}

export interface PullRequestWithAuthor {
  pr: PullRequest
  author_username: string
  head_repo_name: string
  head_repo_owner: string
}

export interface DiffEntry {
  status: string
  old_path: string | null
  new_path: string | null
}

// ── Issues ──

export function listIssues(repoId: number, state?: string) {
  const params = state ? `?state=${state}` : ''
  return request<{ issues: IssueWithAuthor[] }>('GET', `/api/repos/${repoId}/issues${params}`)
}

export function createIssue(repoId: number, title: string, body?: string) {
  return request<{ issue: IssueWithAuthor }>('POST', `/api/repos/${repoId}/issues`, { title, body })
}

export function getIssue(repoId: number, issueNumber: number) {
  return request<{ issue: IssueWithAuthor }>('GET', `/api/repos/${repoId}/issues/${issueNumber}`)
}

export function updateIssue(repoId: number, issueNumber: number, data: { title?: string; body?: string; state?: string }) {
  return request<{ issue: IssueWithAuthor }>('PUT', `/api/repos/${repoId}/issues/${issueNumber}`, data)
}

export function deleteIssue(repoId: number, issueNumber: number) {
  return request<{ deleted: boolean }>('DELETE', `/api/repos/${repoId}/issues/${issueNumber}`)
}

// ── Labels ──

export function listLabels(repoId: number) {
  return request<{ labels: IssueLabel[] }>('GET', `/api/repos/${repoId}/labels`)
}

export function createLabel(repoId: number, name: string, color: string) {
  return request<{ label: IssueLabel }>('POST', `/api/repos/${repoId}/labels`, { name, color })
}

export function deleteLabel(repoId: number, labelId: number) {
  return request<{ deleted: boolean }>('DELETE', `/api/repos/${repoId}/labels/${labelId}`)
}

// ── Comments ──

export function listComments(repoId: number, issueNumber: number) {
  return request<{ comments: IssueComment[] }>('GET', `/api/repos/${repoId}/issues/${issueNumber}/comments`)
}

export function addComment(repoId: number, issueNumber: number, body: string) {
  return request<{ comment: IssueComment }>('POST', `/api/repos/${repoId}/issues/${issueNumber}/comments`, { body })
}

// ── Pull Requests ──

export function listPRs(repoId: number, state?: string) {
  const params = state ? `?state=${state}` : ''
  return request<{ prs: PullRequestWithAuthor[] }>('GET', `/api/repos/${repoId}/pulls${params}`)
}

export function createPR(repoId: number, title: string, headRepoId: number, headRef: string, baseRef: string, body?: string) {
  return request<{ pr: PullRequestWithAuthor }>('POST', `/api/repos/${repoId}/pulls`, { title, body, head_repo_id: headRepoId, head_ref: headRef, base_ref: baseRef })
}

export function getPR(repoId: number, prNumber: number) {
  return request<{ pr: PullRequestWithAuthor }>('GET', `/api/repos/${repoId}/pulls/${prNumber}`)
}

export function updatePR(repoId: number, prNumber: number, data: { title?: string; body?: string; state?: string }) {
  return request<{ pr: PullRequestWithAuthor }>('PUT', `/api/repos/${repoId}/pulls/${prNumber}`, data)
}

export function mergePR(repoId: number, prNumber: number) {
  return request<{ merged: boolean; merge_commit_sha: string }>('POST', `/api/repos/${repoId}/pulls/${prNumber}/merge`)
}

export function getPRDiff(repoId: number, prNumber: number) {
  return request<{ diff: DiffEntry[] }>('GET', `/api/repos/${repoId}/pulls/${prNumber}/diff`)
}

// ── Fork ──

export function forkRepo(repoId: number, ownerName: string) {
  return request<{ repo: Repo }>('POST', `/api/repos/${repoId}/fork`, { owner_name: ownerName })
}

// ── Stars ──

export function starRepo(repoId: number) {
  return request<{ starred: boolean; stars_count: number }>('PUT', `/api/repos/${repoId}/star`)
}

export function unstarRepo(repoId: number) {
  return request<{ starred: boolean; stars_count: number }>('DELETE', `/api/repos/${repoId}/star`)
}

export function getStarStatus(repoId: number) {
  return request<{ starred: boolean }>('GET', `/api/repos/${repoId}/star`)
}

export function listStargazers(repoId: number) {
  return request<{ stargazers: { id: number; username: string; avatar_url: string }[]; count: number }>('GET', `/api/repos/${repoId}/stars`)
}

export function listUserStars(username: string) {
  return request<{ repos: Repo[] }>('GET', `/api/users/${username}/stars`)
}

// ── Watches ──

export function watchRepo(repoId: number) {
  return request<{ watching: boolean; watch_count: number }>('PUT', `/api/repos/${repoId}/watch`)
}

export function unwatchRepo(repoId: number) {
  return request<{ watching: boolean; watch_count: number }>('DELETE', `/api/repos/${repoId}/watch`)
}

export function getWatchStatus(repoId: number) {
  return request<{ watching: boolean; watch_type: string | null }>('GET', `/api/repos/${repoId}/watch`)
}

const TEXT_EXTENSIONS = new Set([
  'txt', 'md', 'markdown', 'html', 'htm', 'css', 'js', 'ts', 'jsx', 'tsx',
  'json', 'yaml', 'yml', 'toml', 'xml', 'svg', 'csv',
  'sh', 'bash', 'zsh', 'fish',
  'py', 'rb', 'rs', 'go', 'java', 'c', 'h', 'cpp', 'hpp', 'cs', 'swift', 'kt',
  'sql', 'r', 'vue', 'svelte', 'astro', 'php',
  'conf', 'ini', 'cfg', 'env', 'gitignore', 'dockerfile',
  'lock', 'sum',
])

export function isTextFile(name: string): boolean {
  const i = name.lastIndexOf('.')
  if (i === -1) return false
  return TEXT_EXTENSIONS.has(name.slice(i + 1).toLowerCase())
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

// ── SSH Keys ──

export interface SshKey {
  id: number
  user_id: number
  repo_id: number
  name: string
  public_key: string
  created_at: string
}

export function listSshKeys(repoId: number) {
  return request<{ ssh_keys: SshKey[] }>('GET', `/api/repos/${repoId}/ssh-keys`)
}

export function addSshKey(repoId: number, name: string, publicKey: string) {
  return request<{ success: boolean; ssh_key: SshKey }>('POST', `/api/repos/${repoId}/ssh-keys`, { name, public_key: publicKey })
}

export function deleteSshKey(repoId: number, keyId: number) {
  return request<{ success: boolean }>('DELETE', `/api/repos/${repoId}/ssh-keys/${keyId}`)
}

export function changePassword(currentPassword: string, newPassword: string) {
  return request<{ success: boolean }>('PUT', '/api/auth/password', { current_password: currentPassword, new_password: newPassword })
}

export function updateProfile(username: string, bio: string, avatarUrl: string) {
  return request<{ success: boolean }>('PUT', `/api/users/${username}/profile`, { bio, avatar_url: avatarUrl })
}

export interface SshInfo {
  mode: string
  ssh_port?: number
  ssh_password?: string
  container?: string
}

export function getSshInfo() {
  return request<SshInfo>('GET', '/api/user/ssh-info')
}

// ── v2.1 Settings ──

export interface AccessToken {
  id: number
  user_id: number
  name: string
  token_prefix: string
  scopes: string
  last_used_at: string | null
  created_at: string
  expires_at: string | null
}

export interface RepoCollaborator {
  repo_id: number
  user_id: number
  permission: string
  username: string
  avatar_url: string
}

export interface RepoSecret {
  id: number
  repo_id: number
  name: string
  created_at: string
}

export interface BranchProtection {
  id: number
  repo_id: number
  pattern: string
  require_pr: boolean
  require_approvals: number
  dismiss_stale_reviews: boolean
}

// Tokens
export function listTokens() {
  return request<{ tokens: AccessToken[] }>('GET', '/api/user/tokens')
}

export function createToken(name: string, scopes?: string, expiresAt?: string) {
  return request<{ token: AccessToken; raw_token: string }>('POST', '/api/user/tokens', { name, scopes, expires_at: expiresAt })
}

export function deleteToken(tokenId: number) {
  return request<{ success: boolean }>('DELETE', `/api/user/tokens/${tokenId}`)
}

// Collaborators
export function listCollaborators(repoId: number) {
  return request<{ collaborators: RepoCollaborator[] }>('GET', `/api/repos/${repoId}/collaborators`)
}

export function addCollaborator(repoId: number, username: string, permission?: string) {
  return request<{ success: boolean }>('POST', `/api/repos/${repoId}/collaborators`, { username, permission })
}

export function removeCollaborator(repoId: number, userId: number) {
  return request<{ success: boolean }>('DELETE', `/api/repos/${repoId}/collaborators/${userId}`)
}

// Secrets
export function listSecrets(repoId: number) {
  return request<{ secrets: RepoSecret[] }>('GET', `/api/repos/${repoId}/secrets`)
}

export function createSecret(repoId: number, name: string, value: string) {
  return request<{ secret: RepoSecret }>('POST', `/api/repos/${repoId}/secrets`, { name, value })
}

export function deleteSecret(repoId: number, secretId: number) {
  return request<{ success: boolean }>('DELETE', `/api/repos/${repoId}/secrets/${secretId}`)
}

// Branch Protection
export function listBranchProtections(repoId: number) {
  return request<{ branch_protections: BranchProtection[] }>('GET', `/api/repos/${repoId}/branch-protections`)
}

export function createBranchProtection(repoId: number, pattern: string, requirePr?: boolean, requireApprovals?: number, dismissStaleReviews?: boolean) {
  return request<{ branch_protection: BranchProtection }>('POST', `/api/repos/${repoId}/branch-protections`, {
    pattern, require_pr: requirePr, require_approvals: requireApprovals, dismiss_stale_reviews: dismissStaleReviews
  })
}

export function deleteBranchProtection(repoId: number, protectionId: number) {
  return request<{ success: boolean }>('DELETE', `/api/repos/${repoId}/branch-protections/${protectionId}`)
}
