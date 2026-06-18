#!/bin/bash
source test/lib.sh
echo "=== 05-pages ==="
TK=$(register_user "pages05"); assert_not_empty "$TK"
RID=$(create_repo "$TK" "pages-repo"); assert_gt "$RID" 0

# Git push with index.html
rm -rf /tmp/gptest-pages
mkdir -p /tmp/gptest-pages
cd /tmp/gptest-pages
git init -q
git config user.email "test@test.com"
git config user.name "Test"
echo '<!DOCTYPE html><html><body><h1>Hello Pages</h1></body></html>' > index.html
git add -A
git commit -q -m "Add index.html"
git remote add origin "$BASE/git/pages05-$TIMESTAMP/pages-repo"
git push origin main 2>&1
cd - > /dev/null

# Enable Pages
api PUT "/api/pages/$RID" "$TK" '{"branch":"main","source_dir":"/","enabled":true}' > /dev/null

# Check config
CFG=$(api GET "/api/pages/$RID" "$TK")
ENABLED=$(echo "$CFG" | extract "['pages_config']['enabled']")
assert_eq "$ENABLED" "true"

# Verify pages served
PAGE_RESP=$(curl -s "$BASE/pages/pages05-$TIMESTAMP/pages-repo/")
assert_not_empty "$PAGE_RESP"

# Redeploy
api POST "/api/pages/$RID/deploy" "$TK" > /dev/null

# Verify still served
PAGE2=$(curl -s "$BASE/pages/pages05-$TIMESTAMP/pages-repo/")
assert_not_empty "$PAGE2"

summary
