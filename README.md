# gitpage

A lightweight, self-hosted Git platform with Pages hosting, App hosting, file manager, deploy logs, and SSH shell access — like a minimal GitHub/GitLab you run yourself.

## Features

- **Git hosting** — Push/pull/clone via HTTP Smart Protocol
- **Pages** — Static site hosting (auto-deploy on push)
- **App hosting** — Run web apps as subprocesses (port per app)
- **File manager** — Dropbox-style staging area with batch commits
- **Deploy logs** — Track deploy history per app
- **SSH shell** — `ssh` into your repo's staging directory
- **Search** — Public repo search with pagination
- **User profiles** — Bio, avatar, public repo listing

## Quick Start

### Prerequisites

- Rust (edition 2021)
- Node.js 18+
- Git (for `git http-backend`)

### Setup

```bash
# Clone and build
git clone <repo> && cd gitpage
cargo build

# Install frontend deps
cd frontend && npm install && cd ..

# Run seed script (creates data/ with demo users + repos)
./seed.sh

# Or start fresh (data/ will be created on first run)
cargo run
```

Open http://localhost:8080

### Demo accounts (after `./seed.sh`)

| User  | Password |
|-------|----------|
| alice | alice123 |
| bob   | bob123   |

## Configuration

Edit `config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
path = "data/gitpage.db"

[jwt]
secret = "change-me-in-production"          # or set env JWT_SECRET
expires_in_hours = 24

[ssh]
enabled = true                               # write ~/.ssh/authorized_keys

[cors]
allowed_origins = ["*"]                      # or ["http://localhost:5173"]

[upload]
max_file_size = 10485760                     # 10 MB

[apps]
port_range_start = 4000
port_range_end = 65535
```

## Development

```bash
# Terminal 1 — backend
cargo run

# Terminal 2 — frontend (HMR at localhost:5173)
cd frontend && npm run dev
```

The Vite dev server proxies `/api`, `/git`, `/pages` to the Rust backend.

### Build for production

```bash
./run.sh     # Builds frontend → dist/, then backend release, starts on :8080
```

## Architecture

```
frontend/                     React 19 + TypeScript + Vite SPA
src/                          Rust backend (Axum)
├── main.rs                   Entrypoint — config, DB, SSH setup
├── app.rs                    Routes + fallback handler (Git/Pages/App proxy)
├── config.rs                 Config structs (from config.toml)
├── auth/mod.rs               JWT create/verify
├── db/
│   ├── mod.rs                SQLite operations (rusqlite, tokio::sync::Mutex)
│   └── models.rs             Data structs
├── git/mod.rs                libgit2 (tree/blob/log) + git http-backend (push/pull)
├── handlers/                 One file per domain
│   ├── auth.rs               Register, login, password change, profile
│   ├── repos.rs              CRUD, search, rename
│   ├── content.rs            Tree, blob, readme, commits
│   ├── files.rs              Staging file manager
│   ├── pages.rs              Pages config + deploy
│   ├── apps.rs               Apps config + deploy + logs
│   ├── ssh_keys.rs           SSH key management
│   └── git_smart.rs          Pages serving
├── deploy.rs                 App subprocess lifecycle
└── ssh.rs                    authorized_keys generation

data/                         Runtime data (auto-created)
├── gitpage.db                SQLite database
├── repos/                    Bare git repos
├── staging/                  Working directory (file manager)
└── apps/                     App workspaces
```

## Running Tests

```bash
./test.sh       # Integrated API test (starts fresh server)
./seed.sh       # Creates demo data (requires running server)
```

## API Overview

| Method | Path | Description |
|--------|------|-------------|
| POST | /api/auth/register | Register |
| POST | /api/auth/login | Login, returns JWT |
| GET | /api/auth/me | Current user |
| PUT | /api/auth/password | Change password |
| GET/POST | /api/repos | List / Create repos |
| GET/PUT/DELETE | /api/repos/:id | Get / Update / Delete |
| GET | /api/repos/search?q= | Search public repos |
| GET | /api/users/:username/repos | Public repos |
| GET/PUT | /api/users/:username/profile | Profile |
| GET | /api/:user/:repo/tree | List directory |
| GET | /api/:user/:repo/blob | File content |
| GET | /api/:user/:repo/readme | README |
| GET | /api/:user/:repo/commits/:branch | Commits |
| GET/PUT | /api/pages/:repo_id | Pages config |
| POST | /api/pages/:repo_id/deploy | Deploy pages |
| GET/PUT/DELETE | /api/apps/:repo_id | App config |
| POST | /api/apps/:repo_id/deploy | Deploy app |
| GET | /api/apps/:repo_id/deploys[/:id] | Deploy logs |
| GET/PUT/DELETE | /api/repos/:repo_id/files | Staging files |
| POST | /api/repos/:repo_id/commit | Commit staging |
| GET/POST/DELETE | /api/repos/:repo_id/ssh-keys[/:id] | SSH keys |

### Prefix notes

- **Git**: `/git/{user}/{repo}/*` — HTTP Smart Protocol (push/pull)
- **Pages**: `/pages/{user}/{repo}/*` — Served static sites
- **Apps**: `/app/{user}/{repo}/*` — Proxied to running app
- **SPA**: `/*` — Frontend (React) fallback

## Data Storage

All data lives under `data/`:
- `data/gitpage.db` — SQLite (users, repos, configs, deploy logs, SSH keys)
- `data/repos/{user}/{repo}.git` — Bare git repos
- `data/staging/{user}/{repo}/` — File manager working tree
- `data/apps/{user}/{repo}/` — App build workspace

## License

MIT
