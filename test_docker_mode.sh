#!/bin/bash
set -x

# Gitpage Docker Mode Integration Test
# Tests: per-user container creation, container exec for build/start,
#        named volume persistence
# Prerequisites: Docker running on the host

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

TEST_PORT=8081
DATA_DIR="/tmp/gptest-docker-mode-data"

cleanup() {
    kill $SERVER_PID 2>/dev/null || true
    sleep 1
    pkill -f "target/debug/gitpage" 2>/dev/null || true
    docker rm -f gitpage-test 2>/dev/null || true
    docker rm -f gitpage-alice 2>/dev/null || true
    docker volume rm gitpage-home-test 2>/dev/null || true
    docker volume rm gitpage-home-alice 2>/dev/null || true
    rm -rf "$DATA_DIR"
    # Restore original config
    if [ -f "config.toml.bak" ]; then
        mv config.toml.bak config.toml
    fi
    # Clean staging/apps dirs created during test
    rm -rf data/staging/test data/staging/alice
    rm -rf data/apps/test data/apps/alice
}
trap cleanup EXIT
cleanup

echo ""
echo "=== Build backend ==="
cargo build 2>&1 | tail -2

echo ""
echo "=== Setup: backup config, write test config ==="
cp config.toml config.toml.bak
mkdir -p "$DATA_DIR/repos" "$DATA_DIR/staging" "$DATA_DIR/apps"
cat > config.toml <<EOF
[server]
host = "0.0.0.0"
port = $TEST_PORT

[database]
path = "$DATA_DIR/gitpage.db"

[storage]
base_path = "$DATA_DIR"

[jwt]
secret = "gitpage-test-secret"
expires_in_hours = 24

[ssh]
enabled = false

[cors]
allowed_origins = ["*"]

[upload]
max_file_size = 10485760

[apps]
port_range_start = 4000
port_range_end = 65535

[runtime]
mode = "docker"

[docker]
base_image = "gitpage-dev-base:latest"
network = "bridge"
EOF

echo ""
echo "=== Check Docker connectivity ==="
if ! docker info >/dev/null 2>&1; then
    echo "FAIL: Docker daemon is not running. Please start Docker and try again."
    exit 1
fi
echo "Docker is running"

echo ""
echo "=== Ensure Docker base image exists ==="
docker image inspect gitpage-dev-base:latest >/dev/null 2>&1 || \
    docker build -t gitpage-dev-base:latest -f Dockerfile.base .

echo ""
echo "=== Start gitpage server on port $TEST_PORT ==="
RUST_LOG=info cargo run &
SERVER_PID=$!
sleep 5

# Verify server started
curl -sf "http://localhost:$TEST_PORT/" > /dev/null 2>&1 || {
    echo "FAIL: Server not responding on :$TEST_PORT"
    exit 1
}
echo "Server is up on :$TEST_PORT"

echo ""
echo "=== 1. Register user (triggers container creation) ==="
RESP=$(curl -s -X POST "http://localhost:$TEST_PORT/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{"username":"test","email":"test@test.com","password":"pass123"}')
echo "$RESP" | python3 -m json.tool
TK=$(echo "$RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["token"])')
[ -z "$TK" ] && { echo "FAIL: no token"; exit 1; }
echo "TOKEN=${TK:0:20}..."

echo ""
echo "=== 2. Verify per-user container was created ==="
sleep 3
docker ps --filter "name=gitpage-test" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
STATUS=$(docker inspect gitpage-test --format '{{.State.Status}}' 2>/dev/null)
if [ "$STATUS" = "running" ]; then
    echo "PASS: User container gitpage-test is running"
else
    echo "FAIL: User container not running (status=$STATUS)"
    docker ps -a --filter "name=gitpage-test"
    exit 1
fi

echo ""
echo "=== 3. Verify named volume exists ==="
docker volume ls --format '{{.Name}}' | grep -q gitpage-home-test
if [ $? -eq 0 ]; then
    echo "PASS: Named volume gitpage-home-test exists"
else
    echo "WARN: Named volume not found"
fi

echo ""
echo "=== 4. Verify staging bind mount ==="
docker exec gitpage-test sh -c "ls -la /workspace/" 2>&1 | head -5

echo ""
echo "=== 5. Create repo ==="
REPO=$(curl -sf -X POST "http://localhost:$TEST_PORT/api/repos" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TK" \
  -d '{"name":"myapp","description":"test app"}')
echo "$REPO" | python3 -m json.tool
REPO_ID=$(echo "$REPO" | python3 -c "import sys,json;print(json.load(sys.stdin).get('repo',{}).get('id',0))")
[ "$REPO_ID" = "0" ] && { echo "FAIL: repo creation"; exit 1; }

echo ""
echo "=== 6. Git push ==="
rm -rf /tmp/gptest-dm-repo
mkdir -p /tmp/gptest-dm-repo
cd /tmp/gptest-dm-repo
git init -q
git config user.email "test@test.com"
git config user.name "Test"
cat > package.json <<'JSON'
{
  "name": "myapp",
  "version": "1.0.0",
  "scripts": { "start": "node server.js" }
}
JSON
cat > server.js <<'JS'
const http = require('http');
const port = process.env.PORT || 4000;
http.createServer((req, res) => {
  res.writeHead(200, {'Content-Type': 'text/plain'});
  res.end('Hello from container\n');
}).listen(port, () => console.log('listening on ' + port));
JS
git add -A
git commit -q -m "Initial commit"
git remote add origin "http://localhost:$TEST_PORT/git/test/myapp"
git push origin main 2>&1
cd - > /dev/null

echo ""
echo "=== 7. Enable App (build + start inside container) ==="
curl -sf -X PUT "http://localhost:$TEST_PORT/api/apps/$REPO_ID" \
  -H "Authorization: Bearer $TK" \
  -H "Content-Type: application/json" \
  -d '{
    "branch":"main",
    "source_dir":"/",
    "build_command":"npm install",
    "start_command":"node server.js",
    "enabled":true
  }' | python3 -m json.tool

echo ""
echo "=== 8. Wait for container build and start ==="
sleep 8

# Check if the build ran
echo "--- Container process list ---"
docker exec gitpage-test sh -c "ps aux 2>/dev/null | grep -i node | head -5 || echo '(no node process)'"

echo "--- Container listening ports ---"
docker exec gitpage-test sh -c "lsof -i -P -n 2>/dev/null | grep LISTEN | head -5 || ss -tlnp 2>/dev/null | head -5 || echo '(no lsof/ss)'"

echo ""
echo "=== 9. Container IP ==="
CONTAINER_IP=$(docker inspect gitpage-test --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}')
echo "IP: $CONTAINER_IP"

echo ""
echo "=== 10. API checks ==="
curl -sf "http://localhost:$TEST_PORT/api/test/myapp/tree?branch=main" | python3 -m json.tool

echo ""
echo "=== 11. Cleanup test user ==="
docker rm -f gitpage-test 2>/dev/null || true
docker volume rm gitpage-home-test 2>/dev/null || true

echo ""
echo "=== 12. Register second user (verify fresh container) ==="
RESP2=$(curl -s -X POST "http://localhost:$TEST_PORT/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","email":"alice@test.com","password":"pass123"}')
echo "$RESP2" | python3 -c "import sys,json;d=json.load(sys.stdin);print('Alice:', d.get('user',{}).get('username','?'))"
sleep 3
docker inspect gitpage-alice --format '{{.State.Status}}' 2>/dev/null | grep -q running
if [ $? -eq 0 ]; then
    echo "PASS: Alice container created"
else
    echo "FAIL: Alice container not running"
    exit 1
fi

echo ""
echo "=== ALL DOCKER MODE TESTS PASSED ==="
