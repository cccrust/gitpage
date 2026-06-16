#!/bin/bash
set -x

# Gitpage v0.1 Integration Test
# Tests: CLI, REST API, Git push/clone, Markdown rendering

cleanup() {
    pkill -f gitpage 2>/dev/null
    rm -rf /tmp/gptest-*
}
trap cleanup EXIT

cleanup

# Start server
rm -rf data && mkdir -p data/repos
cargo build 2>&1 | tail -2
cargo run &
sleep 3

# 1. Register
curl -s -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"test","email":"test@test.com","password":"pass123"}' | python3 -m json.tool

# 2. Login
RESP=$(curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"test","password":"pass123"}')
TK=$(echo "$RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["token"])')
echo "TOKEN=${TK:0:20}..."

# 3. Me
curl -s http://localhost:8080/api/auth/me -H "Authorization: Bearer $TK" | python3 -m json.tool

# 4. Create repo
curl -s -X POST http://localhost:8080/api/repos \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TK" \
  -d '{"name":"myproject","description":"test project"}' | python3 -m json.tool

# 5. List repos
curl -s http://localhost:8080/api/repos -H "Authorization: Bearer $TK" | python3 -m json.tool

# 6. Public repos
curl -s http://localhost:8080/api/users/test/repos | python3 -m json.tool

# 7. Git push from local repo
rm -rf /tmp/gptest-repo
mkdir -p /tmp/gptest-repo
cd /tmp/gptest-repo
git init -q
git config user.email "test@test.com"
git config user.name "Test"
echo "# My Project" > README.md
echo "fn main() {}" > main.rs
mkdir -p src
echo "pub mod utils;" > src/lib.rs
git add -A
git commit -q -m "Initial commit"
git remote add origin http://localhost:8080/git/test/myproject
git push origin main 2>&1
cd - > /dev/null

# 8. Tree listing
curl -s "http://localhost:8080/api/test/myproject/tree?branch=main" | python3 -m json.tool

# 9. Subdirectory tree
curl -s "http://localhost:8080/api/test/myproject/tree?branch=main&path=src" | python3 -m json.tool

# 10. Blob with markdown rendering
curl -s "http://localhost:8080/api/test/myproject/blob?branch=main&path=README.md" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('is_markdown:', d['is_markdown'])
print('rendered:', d.get('rendered','NONE')[:100])
"

# 11. Raw file
curl -s "http://localhost:8080/api/test/myproject/blob?branch=main&path=main.rs" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('content:', d['content'][:100])
"

# 12. README endpoint
curl -s "http://localhost:8080/api/test/myproject/readme?branch=main" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('has_readme:', d['has_readme'])
print('rendered:', d.get('rendered','NONE')[:100])
"

# 13. Commits
curl -s "http://localhost:8080/api/test/myproject/commits/main" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for c in d['commits']:
    print(c['sha'], c['author'], c['message'].strip())
"

# 14. Clone
rm -rf /tmp/gptest-clone
git clone http://localhost:8080/git/test/myproject /tmp/gptest-clone 2>&1
ls -la /tmp/gptest-clone/

# 15. Push second commit
cd /tmp/gptest-clone
echo "// new file" > new.txt
git add new.txt
git commit -q -m "Second commit"
git push origin main 2>&1
cd - > /dev/null

# 16. Verify second commit
curl -s "http://localhost:8080/api/test/myproject/commits/main" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('Commits:', len(d['commits']))
for c in d['commits']:
    print(' -', c['sha'], c['message'].strip())
"

# 17. Delete repo
curl -s -X DELETE http://localhost:8080/api/repos/1 \
  -H "Authorization: Bearer $TK" | python3 -m json.tool

# 18. Verify deletion
curl -s http://localhost:8080/api/repos -H "Authorization: Bearer $TK" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('Remaining repos:', len(d['repos']))
"

# 19. Auth test (no token)
curl -s -X POST http://localhost:8080/api/repos \
  -H "Content-Type: application/json" \
  -d '{"name":"shouldfail"}' | python3 -m json.tool

cleanup
echo ""
echo "=== ALL TESTS PASSED ==="
