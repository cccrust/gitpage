# Pages

## Overview

The `pages/` directory contains 37 React components, each representing a full-page view in the application. Every page follows a consistent pattern:

1. **`useState`** for local state: data, loading flag, error message, and any form inputs.
2. **`useEffect`** on mount (and on dependency change) to fetch data from `api.ts`.
3. **Conditional rendering** based on `loading` (show `<Spinner />`), `err` (show `<div className="error-box">`), and `data` (show the actual UI).
4. **Direct imports** from `api.ts` — there is no service layer or data hook abstraction.

Pages do not use React Context, Redux, or any global state store. Each page is self-contained and re-fetches its data on every mount.

---

## Routed Pages

### Dashboard (`/`)
- **File**: `Dashboard.tsx`
- **Purpose**: Landing page. For anonymous users, shows a welcome callout with Login/Register links. For authenticated users, shows "My Repos" and "Starred" tabs with a global search bar.
- **Key state**: `repos`, `starredRepos`, `tab`, `searchQ`, `searchResults`, `searchPage`, `searchTotalPages`
- **API calls**: `listRepos()`, `listUserStars('me')`, direct `fetch('/api/repos/search?...')`
- **Components used**: `Spinner`, `Pagination` (for search results)

### LoginPage (`/login`)
- **File**: `LoginPage.tsx`
- **Purpose**: Username + password login form. On success, stores JWT via `setToken()` and navigates to `/`.
- **Key state**: `username`, `password`, `err`
- **API calls**: `login(username, password)`

### RegisterPage (`/register`)
- **File**: `RegisterPage.tsx`
- **Purpose**: Username + email + password registration form. On success, stores JWT and navigates to `/`.
- **Key state**: `username`, `email`, `password`, `err`
- **API calls**: `register(username, email, password)`

### NewRepoPage (`/new`)
- **File**: `NewRepoPage.tsx`
- **Purpose**: Create a new repository. Includes optional org ownership selector.
- **Key state**: `name`, `desc`, `priv`, `orgs`, `org`, `err`
- **API calls**: `createRepo()`, `listMyOrgs()`

### RepoPage (`/repo/:id`)
- **File**: `RepoPage.tsx`
- **Purpose**: Repository landing page. Shows breadcrumb, star/fork buttons, file tree, rendered README, recent commits, and clone URL. Links to sub-pages (Commits, Files, Issues, PRs, Pages, App, SSH, Settings).
- **Key state**: `repo`, `entries`, `readmeHtml`, `commits`, `starred`, `forking`
- **API calls**: `getRepo()`, `listTree()`, `getReadme()`, `listCommits()`, `getStarStatus()`, `starRepo()`/`unstarRepo()`, `forkRepo()`, `me()`
- **Components used**: `MarkdownView`, `Spinner`

### FileViewPage (`/repo/:id/*`)
- **File**: `FileViewPage.tsx`
- **Purpose**: Catch-all for repo paths. Parses the `*` wildcard into `tree|blob`/`branch`/`path`. Renders directory listings or file content. Markdown files use `MarkdownView`; other text files use a `<pre>` block.
- **Key state**: `repo`, `content`, `rendered`, `entries`, `isDir`, `branch`, `path`
- **API calls**: `getRepo()`, `listTree()`, `getBlob()`
- **Components used**: `MarkdownView`, `Spinner`

### FileExplorerPage (`/repo/:id/files`)
- **File**: `FileExplorerPage.tsx`
- **Purpose**: Staging area file manager. Navigates the working tree (not the git tree), supports file upload, new file, new folder, delete, and commit ("Save Version") with a modal dialog showing pending changes.
- **Key state**: `repo`, `entries`, `currentPath`, `pending`, `changes`, `showCommit`, `commitMsg`, `showNewDir`, `newDirName`, `uploading`
- **API calls**: `getRepo()`, `listWorkingTree()`, `deleteFile()`, `mkdir()`, `writeFile()`, `getStatus()`, `commitRepo()`
- **Components used**: `Spinner`

### FileEditorPage (`/repo/:id/files/edit` and `/repo/:id/files/new`)
- **File**: `FileEditorPage.tsx`
- **Purpose**: Text file editor for the staging area. Creates new files or edits existing ones. Saves via `writeFile()`. Determines "new" vs "edit" mode by checking `location.pathname`.
- **Key state**: `repo`, `content`, `originalContent`, `saving`, `msg`, `err`, `filePath`
- **API calls**: `getRepo()`, `getRawFile()`, `writeFile()`
- **Components used**: `Spinner`

### CommitsPage (`/repo/:id/commits/:branch`)
- **File**: `CommitsPage.tsx`
- **Purpose**: Commit history for a specific branch. Lists SHA, commit message, author, and timestamp.
- **Key state**: `repo`, `commits`
- **API calls**: `getRepo()`, `listCommits()`
- **Components used**: `Spinner`

### PagesSettingsPage (`/repo/:id/pages`)
- **File**: `PagesSettingsPage.tsx`
- **Purpose**: Configure Gitpage Pages (static site hosting). Fields for branch, source directory, custom domain, enabled toggle. Shows site URL and redeploy button when enabled.
- **Key state**: `repo`, `username`, `branch`, `sourceDir`, `customDomain`, `enabled`, `saving`, `deploying`
- **API calls**: `getRepo()`, `getPagesConfig()`, `updatePagesConfig()`, `deployPages()`
- **Components used**: `Spinner`

### AppSettingsPage (`/repo/:id/app`)
- **File**: `AppSettingsPage.tsx`
- **Purpose**: Configure Gitpage App hosting. Fields for branch, source dir, build command, start command, environment variables (JSON), enabled toggle. Shows app status (running/deploying/failed) and URL.
- **Key state**: `repo`, `status`, `branch`, `sourceDir`, `buildCommand`, `startCommand`, `envVars`, `enabled`
- **API calls**: `getRepo()`, `getAppsConfig()`, `updateAppsConfig()`, `deployApps()`
- **Components used**: `Spinner`

### DeployLogsPage (`/repo/:id/deploys`)
- **File**: `DeployLogsPage.tsx`
- **Purpose**: List of deploy log entries with status indicators (running/success/failed) and timestamps. Links to individual log detail pages.
- **Key state**: `repo`, `logs`
- **API calls**: `getRepo()`, `listDeploys()`
- **Components used**: `Spinner`

### DeployLogDetailPage (`/repo/:id/deploys/:deployId`)
- **File**: `DeployLogDetailPage.tsx`
- **Purpose**: Single deploy log detail view — status badge, start/finish timestamps, full log output in a `<pre>` block.
- **Key state**: `repo`, `log`
- **API calls**: `getRepo()`, `getDeployLog()`
- **Components used**: `Spinner`

### RepoSettingsPage (`/repo/:id/settings`)
- **File**: `RepoSettingsPage.tsx`
- **Purpose**: Repository settings — rename, description, private toggle, and a danger zone for deletion (requires typing the repo name to confirm). Links to collaborators, secrets, and branch protection sub-pages.
- **Key state**: `repo`, `name`, `desc`, `isPrivate`, `saving`, `deleting`, `confirmDelete`
- **API calls**: `getRepo()`, `deleteRepo()`, direct `fetch()` for update

### RepoSSHKeysPage (`/repo/:id/ssh`)
- **File**: `RepoSSHKeysPage.tsx`
- **Purpose**: SSH key management for the repository. Add a public key (name + key text), list existing keys with delete, and show connection instructions.
- **Key state**: `repo`, `keys`, `keyName`, `publicKey`, `adding`
- **API calls**: `getRepo()`, `listSshKeys()`, `addSshKey()`, `deleteSshKey()`
- **Components used**: `Spinner`

### RepoSettingsCollaboratorsPage (`/repo/:id/collaborators`)
- **File**: `RepoSettingsCollaboratorsPage.tsx`
- **Purpose**: Collaborator management — add users by username, list current collaborators with permission level, remove collaborators.
- **Key state**: `repo`, `collabs`, `username`, `adding`
- **API calls**: `getRepo()`, `listCollaborators()`, `addCollaborator()`, `removeCollaborator()`
- **Components used**: `Spinner`

### RepoSettingsSecretsPage (`/repo/:id/secrets`)
- **File**: `RepoSettingsSecretsPage.tsx`
- **Purpose**: Repo-level secrets (environment variables for app deployment). Add name/value pairs (value is masked), list existing secrets, delete.
- **Key state**: `repo`, `secrets`, `name`, `value`, `creating`
- **API calls**: `getRepo()`, `listSecrets()`, `createSecret()`, `deleteSecret()`
- **Components used**: `Spinner`

### RepoSettingsBranchProtectionPage (`/repo/:id/branch-protection`)
- **File**: `RepoSettingsBranchProtectionPage.tsx`
- **Purpose**: Branch protection rules. Add a pattern (e.g., `main`, `release/*`), list existing rules with PR/approval/stale-review settings, delete rules.
- **Key state**: `repo`, `protections`, `pattern`, `creating`
- **API calls**: `getRepo()`, `listBranchProtections()`, `createBranchProtection()`, `deleteBranchProtection()`
- **Components used**: `Spinner`

### UserProfilePage (`/u/:username`)
- **File**: `UserProfilePage.tsx`
- **Purpose**: User profile page. Shows avatar (first letter), username, bio, join date, and list of public repositories.
- **Key state**: `profile`, `repos`
- **API calls**: direct `fetch('/api/users/:username/profile')` (not through `api.ts`)

### UserSettingsPage (`/settings`)
- **File**: `UserSettingsPage.tsx`
- **Purpose**: User account settings. Bio editor, avatar URL editor, password change form. Links to access tokens page.
- **Key state**: `user`, `bio`, `avatarUrl`, `curPw`, `newPw`
- **API calls**: `me()`, `updateProfile()`, `changePassword()`
- **Components used**: `Spinner`

### SettingsTokensPage (`/settings/tokens`)
- **File**: `SettingsTokensPage.tsx`
- **Purpose**: Personal access token management. Create new tokens (shows raw token once), list existing tokens with prefix and scopes, delete tokens.
- **Key state**: `tokens`, `name`, `rawToken`
- **API calls**: `listTokens()`, `createToken()`, `deleteToken()`
- **Components used**: `Spinner`

### OrgList (`/orgs`)
- **File**: `OrgList.tsx`
- **Purpose**: List organizations the current user belongs to. Shows display name, description, and user's role (admin/member).
- **Key state**: `orgs`
- **API calls**: `listMyOrgs()`
- **Components used**: `Spinner`

### OrgCreate (`/orgs/new`)
- **File**: `OrgCreate.tsx`
- **Purpose**: Create a new organization. Form with name, display name, and description fields.
- **Key state**: `name`, `displayName`, `description`, `submitting`, `err`
- **API calls**: `createOrg()`

### OrgDetail (`/org/:name`)
- **File**: `OrgDetail.tsx`
- **Purpose**: Organization home page. Shows display name, description, list of repos owned by the org. Links to members and settings pages.
- **Key state**: `org`, `repos`
- **API calls**: `getOrg()`, `listOrgRepos()`
- **Components used**: `Spinner`

### OrgSettings (`/org/:name/settings`)
- **File**: `OrgSettings.tsx`
- **Purpose**: Organization settings — edit display name and description.
- **Key state**: `displayName`, `description`, `saving`
- **API calls**: `getOrg()`, `updateOrg()`
- **Components used**: `Spinner`

### OrgMembers (`/org/:name/members`)
- **File**: `OrgMembers.tsx`
- **Purpose**: Organization member management. List members with roles (admin/member), add members by username, remove members.
- **Key state**: `org`, `members`, `addName`, `adding`
- **API calls**: `getOrg()`, `listOrgMembers()`, `addOrgMember()`, `removeOrgMember()`
- **Components used**: `Spinner`

### DockerStatusPage (`/docker-status`)
- **File**: `DockerStatusPage.tsx`
- **Purpose**: Display Docker runtime information. Shows execution mode (docker vs process), container name, SSH port, SSH password, and a sample SSH connection command.
- **Key state**: `info`, `error`
- **API calls**: `getSshInfo()`
- **Components used**: `Spinner`

---

## Unrouted Pages

These page components exist in the `pages/` directory but are **not registered** in `App.tsx` routes. They are fully implemented (API calls, state management, error handling) but unreachable through normal navigation. They represent features that were partially developed.

### IssueList (unrouted, expected `/repo/:id/issues`)
- **File**: `IssueList.tsx`
- **Purpose**: List issues for a repository with open/closed/all tab filter. Shows issue number, title, state indicator, author, date, and labels.
- **Key state**: `repo`, `issues`, `filter`
- **API calls**: `getRepo()`, `listIssues()`
- **Components used**: `Spinner`

### IssueDetail (unrouted, expected `/repo/:id/issues/:issueNumber`)
- **File**: `IssueDetail.tsx`
- **Purpose**: Single issue view. Shows title, state badge, body, labels, comment thread. Supports state toggle (open/close), delete, and adding comments.
- **Key state**: `data`, `comments`, `commentBody`, `submitting`
- **API calls**: `getIssue()`, `listComments()`, `addComment()`, `updateIssue()`, `deleteIssue()`
- **Components used**: `Spinner`

### IssueNew (unrouted, expected `/repo/:id/issues/new`)
- **File**: `IssueNew.tsx`
- **Purpose**: Create a new issue. Title and description form. Redirects to the new issue on success.
- **Key state**: `title`, `body`, `submitting`, `err`
- **API calls**: `getRepo()`, `createIssue()`

### PRList (unrouted, expected `/repo/:id/pulls`)
- **File**: `PRList.tsx`
- **Purpose**: List pull requests with open/closed/merged/all filter. Shows number, title, state indicator, author, date, and branch refs.
- **Key state**: `repo`, `prs`, `filter`
- **API calls**: `getRepo()`, `listPRs()`
- **Components used**: `Spinner`

### PRDetail (unrouted, expected `/repo/:id/pulls/:prNumber`)
- **File**: `PRDetail.tsx`
- **Purpose**: Single PR view. Shows title, state badge, body, branch merge info, merge button, close button, diff file list (collapsible), and comment thread.
- **Key state**: `data`, `comments`, `diff`, `commentBody`, `showDiff`, `merging`
- **API calls**: `getPR()`, `listComments()`, `addComment()`, `mergePR()`, `getPRDiff()`, `updatePR()`
- **Components used**: `Spinner`

### PRNew (unrouted, expected `/repo/:id/pulls/new`)
- **File**: `PRNew.tsx`
- **Purpose**: Create a new pull request. Fields for base branch, head branch, title, and description.
- **Key state**: `title`, `body`, `baseRef`, `headRef`, `submitting`, `err`
- **API calls**: `getRepo()`, `createPR()`

### SettingsBranches (unrouted, standalone)
- **File**: `SettingsBranches.tsx`
- **Purpose**: Branch protection management (richer UI than the routed version — table layout, PR requirement checkbox, approval count, stale review toggle).
- **Key state**: `protections`, `pattern`, `requirePr`, `requireApprovals`, `dismissStale`
- **API calls**: `listBranchProtections()`, `createBranchProtection()`, `deleteBranchProtection()`
- **Components used**: `Spinner`

### SettingsCollaborators (unrouted, standalone)
- **File**: `SettingsCollaborators.tsx`
- **Purpose**: Collaborator management (alternative UI — table layout with permission selector for read/write/admin).
- **Key state**: `collabs`, `username`, `permission`
- **API calls**: `listCollaborators()`, `addCollaborator()`, `removeCollaborator()`
- **Components used**: `Spinner`

### SettingsSecrets (unrouted, standalone)
- **File**: `SettingsSecrets.tsx`
- **Purpose**: Secret management (alternative UI — table layout with creation timestamps).
- **Key state**: `secrets`, `name`, `value`
- **API calls**: `listSecrets()`, `createSecret()`, `deleteSecret()`
- **Components used**: `Spinner`
