#!/bin/bash
set -e

BASE="http://localhost:8080"
WORK="/tmp/gpseed"
rm -rf "$WORK"
mkdir -p "$WORK"

cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

# ── helpers ──

api() {
  curl -s -X "$1" "$BASE$2" ${3:+-H "Authorization: Bearer $3"} \
    ${4:+ -H "Content-Type: application/json" -d "$4"}
}

login_or_register() {
  local u=$1 p="${1}123"
  # try login first
  local resp
  resp=$(curl -s -X POST "$BASE/api/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"$u\",\"password\":\"$p\"}")
  local tk
  tk=$(echo "$resp" | python3 -c "import sys,json;print(json.load(sys.stdin).get('token',''))" 2>/dev/null)
  if [ -n "$tk" ]; then
    echo "$tk"
    return
  fi
  # register if not exists
  resp=$(curl -s -X POST "$BASE/api/auth/register" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"$u\",\"email\":\"$u@test.com\",\"password\":\"$p\"}")
  echo "$resp" | python3 -c "import sys,json;print(json.load(sys.stdin)['token'])"
}

repo() {
  api POST "/api/repos" "$TOKEN" \
    "{\"name\":\"$1\",\"description\":\"$2\",\"is_private\":$3}" > /dev/null
}

push() {
  local user=$1 repo=$2 fname=$3 msg=$4
  rm -rf "$WORK/$repo" && mkdir -p "$WORK/$repo"
  cd "$WORK/$repo"
  git init -q
  git config user.email "$user@test.com"
  git config user.name "$user"
  # write content passed via heredoc
  while IFS= read -r line; do
    echo "$line" >> "$fname"
  done
  git add -A 2>/dev/null
  git commit -q -m "$msg" --allow-empty
  git branch -m main
  git remote add origin "http://localhost:8080/git/$user/$repo"
  git push origin main 2>&1 | tail -1
  cd - > /dev/null
}

# ─────────────────────────────────────

echo "=== Seeding gitpage ==="
echo ""

# ensure server is running
if ! curl -sf "$BASE/api/auth/me" > /dev/null 2>&1; then
  echo "Starting server..."
  cd "$(dirname "$0")"
  # kill anything on port 8080 first
  lsof -ti tcp:8080 2>/dev/null | xargs kill -9 2>/dev/null || true
  sleep 1
  rm -rf data
  cargo run --release > /tmp/gpseed-server.log 2>&1 &
  sleep 4
fi
echo "Server ready at $BASE"
echo ""

# ── User: alice ──
echo "--- alice ---"
ALICE=$(login_or_register alice)
TOKEN=$ALICE

repo "blog" "Alice's personal blog" false
repo "dotfiles" "My dotfiles" false
repo "secret-project" "Top secret stuff" true

# Push blog content
cat > "$WORK/blog-content" << 'ENDCONTENT'
# Alice's Blog

Welcome to my blog.

## Posts

- [Getting Started with Rust](./rust-intro.md)
- [My Development Setup](./dev-setup.md)

## About Me

I'm a software engineer who loves Rust and building things.
ENDCONTENT
echo "" > "$WORK/blog-content"

push alice blog "README.md" "Initial commit" << 'ENDREADME'
# Alice's Blog

Welcome to my blog. Powered by gitpage.
ENDREADME

# Push a Rust project
rm -rf "$WORK/secret-project" && mkdir -p "$WORK/secret-project"
cd "$WORK/secret-project"
git init -q && git config user.email "alice@test.com" && git config user.name "alice"
mkdir -p src
cat > Cargo.toml << 'ENDTOML'
[package]
name = "secret-project"
version = "0.1.0"
edition = "2021"
ENDTOML
cat > src/main.rs << 'ENDRS'
fn main() {
    println!("Hello, world!");
}
ENDRS
cat > README.md << 'ENDREADME'
# Secret Project

This is a private repository.
ENDREADME
git add -A && git commit -q -m "Initial commit"
git branch -m main
git remote add origin http://localhost:8080/git/alice/secret-project
git push origin main 2>&1 | tail -1
cd - > /dev/null

echo "  alice token: $ALICE"

# ── User: bob ──
echo ""
echo "--- bob ---"
BOB=$(login_or_register bob)
TOKEN=$BOB

repo "my-notes" "Personal knowledge base" false
repo "portfolio" "My portfolio site" false

push bob my-notes "README.md" "Initial commit" << 'ENDNOTES'
# My Notes

## Linux
- `grep` cheatsheet
- Systemd service basics

## Git
- Rebase vs merge
- Signed commits
ENDNOTES

# Portfolio with pages
rm -rf "$WORK/portfolio" && mkdir -p "$WORK/portfolio"
cd "$WORK/portfolio"
git init -q && git config user.email "bob@test.com" && git config user.name "bob"
cat > index.html << 'ENDHTML'
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Bob's Portfolio</title>
<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: system-ui, sans-serif; background: #fafafa; color: #111; line-height: 1.6; padding: 40px 20px; max-width: 640px; margin: 0 auto; }
h1 { font-size: 2rem; margin-bottom: 4px; }
.sub { color: #666; margin-bottom: 24px; }
.project { background: #fff; border: 1px solid #ddd; border-radius: 8px; padding: 16px; margin-bottom: 12px; }
.project h3 { margin-bottom: 4px; }
.project p { font-size: 14px; color: #555; }
</style>
</head>
<body>
<h1>Bob</h1>
<p class="sub">Full-stack developer</p>
<div class="project"><h3>gitpage</h3><p>Self-hosted Git hosting platform</p></div>
<div class="project"><h3>My Notes</h3><p>Personal knowledge base</p></div>
</body>
</html>
ENDHTML
git add -A && git commit -q -m "Add portfolio site"
git branch -m main
git remote add origin http://localhost:8080/git/bob/portfolio
git push origin main 2>&1 | tail -1
cd - > /dev/null

# Enable Pages on portfolio
api PUT "/api/pages/5" "$BOB" \
  '{"branch":"main","source_dir":"/","enabled":true}' > /dev/null

# ── SSH key demo ──
echo ""
echo "--- SSH key demo ---"
SSH_KEY_FILE="/tmp/gpseed-id_ed25519"
rm -f "$SSH_KEY_FILE" "$SSH_KEY_FILE.pub"
ssh-keygen -t ed25519 -N "" -f "$SSH_KEY_FILE" -q
PUB_KEY=$(cat "$SSH_KEY_FILE.pub")
TOKEN=$ALICE
api POST "/api/repos/1/ssh-keys" "$ALICE" \
  "{\"name\":\"demo-laptop\",\"public_key\":\"$PUB_KEY\"}" > /dev/null
echo "  Added demo SSH key to alice/blog"
rm -f "$SSH_KEY_FILE" "$SSH_KEY_FILE.pub"

echo ""
echo "=== Seed complete ==="
echo ""
echo "Users:"
echo "  alice / alice123"
echo "  bob   / bob123"
echo ""
echo "Repos:"
echo "  alice/blog          (public, SSH key added)"
echo "  alice/dotfiles      (public)"
echo "  alice/secret-project (private)"
echo "  bob/my-notes        (public)"
echo "  bob/portfolio       (public, Pages enabled)"
echo ""
echo "Pages: http://localhost:8080/pages/bob/portfolio/"
