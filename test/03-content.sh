#!/bin/bash
source test/lib.sh
echo "=== 03-content ==="
TK=$(register_user "cont03"); assert_not_empty "$TK"
RID=$(create_repo "$TK" "content-repo"); assert_gt "$RID" 0

# Git push first
rm -rf /tmp/gptest-content
mkdir -p /tmp/gptest-content
cd /tmp/gptest-content
git init -q
git config user.email "test@test.com"
git config user.name "Test"
echo "# My Project" > README.md
echo "fn main() {}" > main.rs
mkdir -p src
echo "pub mod utils;" > src/lib.rs
git add -A
git commit -q -m "Initial commit"
git remote add origin "$BASE/git/cont03-$TIMESTAMP/content-repo"
git push origin main 2>&1
cd - > /dev/null

# Tree
TREE=$(api GET "/api/cont03-$TIMESTAMP/content-repo/tree?branch=main" "$TK")
assert_not_empty "$TREE"

# Subdirectory
SUBTREE=$(api GET "/api/cont03-$TIMESTAMP/content-repo/tree?branch=main&path=src" "$TK")
assert_not_empty "$SUBTREE"

# Blob (markdown)
BLOB_MD=$(api GET "/api/cont03-$TIMESTAMP/content-repo/blob?branch=main&path=README.md" "$TK")
IS_MD=$(echo "$BLOB_MD" | extract "['is_markdown']")
assert_eq "$IS_MD" "true"

# Blob (raw)
BLOB_RAW=$(api GET "/api/cont03-$TIMESTAMP/content-repo/blob?branch=main&path=main.rs" "$TK")
CONTENT=$(echo "$BLOB_RAW" | extract "['content']")
assert_not_empty "$CONTENT"

# README
README=$(api GET "/api/cont03-$TIMESTAMP/content-repo/readme?branch=main" "$TK")
HAS_RM=$(echo "$README" | extract "['has_readme']")
assert_eq "$HAS_RM" "true"

# Commits
COMMITS=$(api GET "/api/cont03-$TIMESTAMP/content-repo/commits/main" "$TK")
COMMIT_COUNT=$(echo "$COMMITS" | extract "['commits']" | python3 -c "import sys,json;print(len(json.load(sys.stdin)))")
assert_gt "$COMMIT_COUNT" 0

summary
