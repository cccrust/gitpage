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

# Start server (preserves existing data — use seed.sh for fresh state)
mkdir -p data/repos
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
REPO_CREATE=$(curl -s -X POST http://localhost:8080/api/repos \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TK" \
  -d '{"name":"myproject","description":"test project"}')
echo "$REPO_CREATE" | python3 -m json.tool
MY_REPO_ID=$(echo "$REPO_CREATE" | python3 -c "import sys,json;print(json.load(sys.stdin).get('repo',{}).get('id',0))" 2>/dev/null)

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

# 17. Push an index.html for pages test
cd /tmp/gptest-clone
echo '<!DOCTYPE html><html><body><h1>Hello Pages</h1></body></html>' > index.html
git add index.html
git commit -q -m "Add index.html"
git push origin main 2>&1
cd - > /dev/null

# 18. Enable Pages
curl -s -X PUT "http://localhost:8080/api/pages/$MY_REPO_ID" \
  -H "Authorization: Bearer $TK" \
  -H "Content-Type: application/json" \
  -d '{"branch":"main","source_dir":"/","enabled":true}' | python3 -m json.tool

# 19. Check pages config
curl -s "http://localhost:8080/api/pages/$MY_REPO_ID" | python3 -m json.tool

# 20. Verify pages are served
curl -s http://localhost:8080/pages/test/myproject/ | head -3

# 21. Redeploy via API
curl -s -X POST "http://localhost:8080/api/pages/$MY_REPO_ID/deploy" \
  -H "Authorization: Bearer $TK" | python3 -m json.tool

# Verify pages still served
curl -s http://localhost:8080/pages/test/myproject/ | head -3

# ── Org Tests ──

# 22. Create org
ORG_RESP=$(curl -s -X POST http://localhost:8080/api/orgs \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TK" \
  -d '{"name":"myorg","display_name":"My Org","description":"test org"}')
echo "Create org:" && echo "$ORG_RESP" | python3 -m json.tool

# 23. Duplicate org name must fail
curl -s -X POST http://localhost:8080/api/orgs \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TK" \
  -d '{"name":"myorg"}' | python3 -c "import sys,json;d=json.load(sys.stdin);print('Duplicate org:', d.get('error','?'))"

# 24. Get org
curl -s http://localhost:8080/api/orgs/myorg | python3 -m json.tool

# 25. List my orgs
curl -s http://localhost:8080/api/orgs -H "Authorization: Bearer $TK" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('My orgs:', len(d['orgs']))
for o in d['orgs']:
    print(' -', o['name'], o['role'])
"

# 26. Create repo under org
ORG_REPO=$(curl -s -X POST http://localhost:8080/api/repos \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TK" \
  -d '{"name":"orgproject","description":"org repo","org_name":"myorg"}')
echo "Org repo:" && echo "$ORG_REPO" | python3 -m json.tool
ORG_REPO_ID=$(echo "$ORG_REPO" | python3 -c "import sys,json;print(json.load(sys.stdin)['repo']['id'])")

# 27. Duplicate name in org must fail
curl -s -X POST http://localhost:8080/api/repos \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TK" \
  -d '{"name":"orgproject","org_name":"myorg"}' | python3 -c "import sys,json;d=json.load(sys.stdin);print('Duplicate org repo:', d.get('error','?'))"

# 28. Git push to org repo
rm -rf /tmp/gptest-orgrepo
mkdir -p /tmp/gptest-orgrepo
cd /tmp/gptest-orgrepo
git init -q
git config user.email "test@test.com"
git config user.name "Test"
echo "# Org Project" > README.md
git add -A
git commit -q -m "Initial commit"
git remote add origin http://localhost:8080/git/myorg/orgproject
git push origin main 2>&1
cd - > /dev/null

# 29. Tree listing for org repo
curl -s "http://localhost:8080/api/myorg/orgproject/tree?branch=main" | python3 -m json.tool

# 30. Readme for org repo
curl -s "http://localhost:8080/api/myorg/orgproject/readme?branch=main" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('Org repo has_readme:', d['has_readme'])
"

# 31. List org repos
curl -s http://localhost:8080/api/orgs/myorg/repos | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('Org repos:', len(d['repos']))
for r in d['repos']:
    print(' -', r['name'])
"

# 32. Clone org repo
rm -rf /tmp/gptest-orgclone
git clone http://localhost:8080/git/myorg/orgproject /tmp/gptest-orgclone 2>&1
ls -la /tmp/gptest-orgclone/

# 33. List org members
curl -s http://localhost:8080/api/orgs/myorg/members -H "Authorization: Bearer $TK" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('Org members:', len(d['members']))
for m in d['members']:
    print(' -', m['username'], m['role'])
"

# Register bob for member tests
BOB_RESP=$(curl -s -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"bob","email":"bob@test.com","password":"pass123"}')
BOB_TK=$(echo "$BOB_RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["token"])')
BOB_ID=$(curl -s http://localhost:8080/api/auth/me -H "Authorization: Bearer $BOB_TK" | python3 -c 'import sys,json;print(json.load(sys.stdin)["user"]["id"])')
echo "Bob token: ${BOB_TK:0:20}... bob_id=$BOB_ID"

# 34. Add bob as member
curl -s -X POST http://localhost:8080/api/orgs/myorg/members \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TK" \
  -d "{\"username\":\"bob\"}" | python3 -c "import sys,json;d=json.load(sys.stdin);print('Add bob:', d.get('success','?'))"

# 35. Verify bob listed
curl -s http://localhost:8080/api/orgs/myorg/members -H "Authorization: Bearer $TK" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('Members after add:', len(d['members']))
for m in d['members']:
    print(' -', m['username'], m['role'])
"

# 36. Remove bob
curl -s -X DELETE "http://localhost:8080/api/orgs/myorg/members/$BOB_ID" \
  -H "Authorization: Bearer $TK" | python3 -c "import sys,json;d=json.load(sys.stdin);print('Remove bob:', d.get('success','?'))"

# 37. Create org with name matching existing user must fail
curl -s -X POST http://localhost:8080/api/orgs \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TK" \
  -d '{"name":"test"}' | python3 -c "import sys,json;d=json.load(sys.stdin);print('Conflict check:', d.get('error','?'))"

# 38. Delete org repo
curl -s -X DELETE "http://localhost:8080/api/repos/$ORG_REPO_ID" \
  -H "Authorization: Bearer $TK" | python3 -c "import sys,json;d=json.load(sys.stdin);print('Delete org repo:', d.get('deleted','?'))"

# ── End Org Tests ──

# 39. Delete user repo
curl -s -X DELETE "http://localhost:8080/api/repos/$MY_REPO_ID" \
  -H "Authorization: Bearer $TK" | python3 -m json.tool

# 40. Verify deletion
curl -s http://localhost:8080/api/repos -H "Authorization: Bearer $TK" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print('Remaining repos:', len(d['repos']))
"

# 41. Auth test (no token)
curl -s -X POST http://localhost:8080/api/repos \
  -H "Content-Type: application/json" \
  -d '{"name":"shouldfail"}' | python3 -m json.tool

cleanup
echo ""
echo "=== ALL TESTS PASSED ==="
