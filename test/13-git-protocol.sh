#!/bin/bash
source test/lib.sh
echo "=== 13-git-protocol ==="
TK=$(register_user "git13"); assert_not_empty "$TK"
RID=$(create_repo "$TK" "git-repo"); assert_gt "$RID" 0

USERNAME="git13-$TIMESTAMP"

# Git push
rm -rf /tmp/gptest-git
mkdir -p /tmp/gptest-git
cd /tmp/gptest-git
git init -q
git config user.email "test@test.com"
git config user.name "Test"
echo "# Git Repo" > README.md
echo "console.log('hello');" > app.js
git add -A
git commit -q -m "Initial commit"
git remote add origin "$BASE/git/$USERNAME/git-repo"
PUSH_OUT=$(git push origin main 2>&1)
if echo "$PUSH_OUT" | grep -qiE "error|fatal"; then
    FAIL_COUNT=$((FAIL_COUNT + 1))
    echo "FAIL: git push failed: $PUSH_OUT"
else
    PASS_COUNT=$((PASS_COUNT + 1))
fi

# Clone
rm -rf /tmp/gptest-git-clone
CLONE_OUT=$(git clone "$BASE/git/$USERNAME/git-repo" /tmp/gptest-git-clone 2>&1)
if [ -f /tmp/gptest-git-clone/README.md ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
else
    FAIL_COUNT=$((FAIL_COUNT + 1))
    echo "FAIL: clone failed"
fi

# Push second commit
cd /tmp/gptest-git-clone
echo "// new" > new.txt
git add new.txt
git commit -q -m "Second commit"
PUSH2=$(git push origin main 2>&1)
if echo "$PUSH2" | grep -qiE "error|fatal"; then
    FAIL_COUNT=$((FAIL_COUNT + 1))
    echo "FAIL: second push: $PUSH2"
else
    PASS_COUNT=$((PASS_COUNT + 1))
fi
cd - > /dev/null

# Verify second commit via API
COMMITS=$(api GET "/api/$USERNAME/git-repo/commits/main" "$TK")
COMMIT_COUNT=$(echo "$COMMITS" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['commits']))")
assert_eq "$COMMIT_COUNT" "2"

summary
