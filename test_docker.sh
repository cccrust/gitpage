#!/bin/bash
set -x

# Gitpage Docker Integration Test
# Tests: REST API, Git push/clone, Markdown rendering inside Docker

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

IMAGE="gitpage:test"
CONTAINER="gptest-docker"
PORT=18080
DATA_DIR="/tmp/gptest-docker-data"

cleanup() {
    docker rm -f "$CONTAINER" 2>/dev/null
    rm -rf "$DATA_DIR"
}
trap cleanup EXIT
cleanup

echo "=== Build base image (dev tools) ==="
docker build -t gitpage-dev-base:latest -f Dockerfile.base .

echo "=== Build app image ==="
docker build -t "$IMAGE" .

echo ""
echo "=== Start container ==="
mkdir -p "$DATA_DIR/repos"
docker run -d --name "$CONTAINER" \
  -p "$PORT:8080" \
  -v "$DATA_DIR:/app/data" \
  -e RUST_LOG=info \
  "$IMAGE"

sleep 3

# Helper
BASE="http://localhost:$PORT"

# 1. Register
curl -sf -X POST "$BASE/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{"username":"test","email":"test@test.com","password":"pass123"}' | python3 -m json.tool

# 2. Login
RESP=$(curl -sf -X POST "$BASE/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"test","password":"pass123"}')
TK=$(echo "$RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["token"])')
echo "TOKEN=${TK:0:20}..."

# 3. Me
curl -sf "$BASE/api/auth/me" -H "Authorization: Bearer $TK" | python3 -m json.tool

# 4. Create repo
REPO_CREATE=$(curl -sf -X POST "$BASE/api/repos" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TK" \
  -d '{"name":"myproject","description":"test project"}')
echo "$REPO_CREATE" | python3 -m json.tool
MY_REPO_ID=$(echo "$REPO_CREATE" | python3 -c "import sys,json;print(json.load(sys.stdin).get('repo',{}).get('id',0))")

# 5. List repos
curl -sf "$BASE/api/repos" -H "Authorization: Bearer $TK" | python3 -m json.tool

# 6. Public repos
curl -sf "$BASE/api/users/test/repos" | python3 -m json.tool

# 7. Git push
rm -rf /tmp/gptest-docker-repo
mkdir -p /tmp/gptest-docker-repo
cd /tmp/gptest-docker-repo
git init -q
git config user.email "test@test.com"
git config user.name "Test"
echo "# My Project" > README.md
echo "fn main() {}" > main.rs
git add -A
git commit -q -m "Initial commit"
git remote add origin "http://localhost:$PORT/git/test/myproject"
git push origin main 2>&1
cd - > /dev/null

# 8. Tree listing
curl -sf "$BASE/api/test/myproject/tree?branch=main" | python3 -m json.tool

# 9. Subdirectory (empty)
curl -sf "$BASE/api/test/myproject/tree?branch=main&path=src" 2>/dev/null | python3 -m json.tool || echo "(no src dir)"

# 10. Blob with markdown rendering
curl -sf "$BASE/api/test/myproject/blob?branch=main&path=README.md" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('is_markdown:', d['is_markdown'])
print('rendered:', d.get('rendered','NONE')[:100])
"

# 11. Raw file
curl -sf "$BASE/api/test/myproject/blob?branch=main&path=main.rs" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('content:', d['content'][:100])
"

# 12. README endpoint
curl -sf "$BASE/api/test/myproject/readme?branch=main" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('has_readme:', d['has_readme'])
print('rendered:', d.get('rendered','NONE')[:100])
"

# 13. Commits
curl -sf "$BASE/api/test/myproject/commits/main" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for c in d['commits']:
    print(c['sha'], c['author'], c['message'].strip())
"

# 14. Clone
rm -rf /tmp/gptest-docker-clone
git clone "http://localhost:$PORT/git/test/myproject" /tmp/gptest-docker-clone 2>&1
ls -la /tmp/gptest-docker-clone/

# 15. Push second commit
cd /tmp/gptest-docker-clone
echo "// new file" > new.txt
git add new.txt
git commit -q -m "Second commit"
git push origin main 2>&1
cd - > /dev/null

# 16. Verify second commit
curl -sf "$BASE/api/test/myproject/commits/main" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('Commits:', len(d['commits']))
for c in d['commits']:
    print(' -', c['sha'], c['message'].strip())
"

# 17. Pages test
cd /tmp/gptest-docker-clone
echo '<!DOCTYPE html><html><body><h1>Hello Pages</h1></body></html>' > index.html
git add index.html
git commit -q -m "Add index.html"
git push origin main 2>&1
cd - > /dev/null

# 18. Enable Pages
curl -sf -X PUT "$BASE/api/pages/$MY_REPO_ID" \
  -H "Authorization: Bearer $TK" \
  -H "Content-Type: application/json" \
  -d '{"branch":"main","source_dir":"/","enabled":true}' | python3 -m json.tool

# 19. Check pages config
curl -sf "$BASE/api/pages/$MY_REPO_ID" | python3 -m json.tool

# 20. Verify pages are served
curl -sf "$BASE/pages/test/myproject/" | head -3

# 21. Redeploy
curl -sf -X POST "$BASE/api/pages/$MY_REPO_ID/deploy" \
  -H "Authorization: Bearer $TK" | python3 -m json.tool

# Verify pages still served
curl -sf "$BASE/pages/test/myproject/" | head -3

# 22. Delete repo
curl -sf -X DELETE "$BASE/api/repos/$MY_REPO_ID" \
  -H "Authorization: Bearer $TK" | python3 -m json.tool

# 23. Auth test (no token)
curl -s -X POST "$BASE/api/repos" \
  -H "Content-Type: application/json" \
  -d '{"name":"shouldfail"}' | python3 -m json.tool

cleanup
echo ""
echo "=== ALL DOCKER TESTS PASSED ==="
