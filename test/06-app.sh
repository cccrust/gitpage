#!/bin/bash
source test/lib.sh
echo "=== 06-app ==="
TK=$(register_user "app06"); assert_not_empty "$TK"
RID=$(create_repo "$TK" "app-repo"); assert_gt "$RID" 0

# Git push with Node app
rm -rf /tmp/gptest-app
mkdir -p /tmp/gptest-app
cd /tmp/gptest-app
git init -q
git config user.email "test@test.com"
git config user.name "Test"
cat > package.json <<'JSON'
{"name":"myapp","version":"1.0.0","scripts":{"start":"echo hello"}}
JSON
git add -A
git commit -q -m "Initial commit"
git remote add origin "$BASE/git/app06-$TIMESTAMP/app-repo"
git push origin main 2>&1
cd - > /dev/null

# Enable App
api PUT "/api/apps/$RID" "$TK" '{
    "branch":"main","source_dir":"/",
    "build_command":"npm install","start_command":"npm start",
    "enabled":true
}' > /dev/null

# Check config
CFG=$(api GET "/api/apps/$RID" "$TK")
assert_not_empty "$CFG"

# Deploy logs
LOGS=$(api GET "/api/apps/$RID/logs" "$TK")
assert_not_empty "$LOGS"

summary
