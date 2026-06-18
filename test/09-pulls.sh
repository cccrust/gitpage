#!/bin/bash
source test/lib.sh
echo "=== 09-pulls ==="
TK=$(register_user "pr09"); assert_not_empty "$TK"
RID=$(create_repo "$TK" "pr-repo"); assert_gt "$RID" 0

# Git push base
rm -rf /tmp/gptest-pr
mkdir -p /tmp/gptest-pr
cd /tmp/gptest-pr
git init -q
git config user.email "test@test.com"
git config user.name "Test"
echo "# Main" > README.md
git add -A
git commit -q -m "Initial commit"
git remote add origin "$BASE/git/pr09-$TIMESTAMP/pr-repo"
git push origin main 2>&1
cd - > /dev/null

# Create PR (must use same repo for head since no fork exists)
PR=$(api POST "/api/repos/$RID/pulls" "$TK" "{\"title\":\"Add feature\",\"body\":\"description\",\"head_repo_id\":$RID,\"head_ref\":\"main\",\"base_ref\":\"main\"}")
PR_NUM=$(echo "$PR" | extract "['pull']['pr']['number']")
assert_gt "$PR_NUM" 0

# List PRs
PRS=$(api GET "/api/repos/$RID/pulls" "$TK")
PR_COUNT=$(echo "$PRS" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['pulls']))" 2>/dev/null)
assert_gt "$PR_COUNT" 0

# Get single PR
PR_GET=$(api GET "/api/repos/$RID/pulls/$PR_NUM" "$TK")
TITLE=$(echo "$PR_GET" | extract "['pull']['pr']['title']")
assert_eq "$TITLE" "Add feature"

# Update PR
api PATCH "/api/repos/$RID/pulls/$PR_NUM" "$TK" '{"state":"closed"}' > /dev/null

summary
