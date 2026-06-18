#!/bin/bash
source test/lib.sh
echo "=== 04-files ==="
TK=$(register_user "files04"); assert_not_empty "$TK"
RID=$(create_repo "$TK" "files-repo"); assert_gt "$RID" 0

# Write file (response: { "success": true, "path": "hello.txt" })
WRITE_RESP=$(api PUT "/api/repos/$RID/files?path=hello.txt" "$TK" "Hello")
WRITE_SUCC=$(echo "$WRITE_RESP" | extract "['success']" 2>/dev/null)
assert_eq "$WRITE_SUCC" "true"

# List tree (response: { "entries": [...], "path": "/" })
TREE=$(api GET "/api/repos/$RID/tree" "$TK")
assert_not_empty "$TREE"
# Verify hello.txt appears in entries
HAS_FILE=$(echo "$TREE" | python3 -c "import sys,json;d=json.load(sys.stdin);any(e['name']=='hello.txt' for e in d['entries']) and print('yes')" 2>/dev/null)
assert_eq "$HAS_FILE" "yes"

# Create directory
MKDIR_RESP=$(api POST "/api/repos/$RID/mkdir?path=subdir" "$TK" "")
MKDIR_SUCC=$(echo "$MKDIR_RESP" | extract "['success']" 2>/dev/null)
assert_eq "$MKDIR_SUCC" "true"

# Move file
MOVE_RESP=$(api POST "/api/repos/$RID/move?from=hello.txt&to=hello2.txt" "$TK" "")
MOVE_SUCC=$(echo "$MOVE_RESP" | extract "['success']" 2>/dev/null)
assert_eq "$MOVE_SUCC" "true"

# Status (response: { "pending": boolean, "changes": [...] })
STATUS=$(api GET "/api/repos/$RID/status" "$TK")
IS_PENDING=$(echo "$STATUS" | extract "['pending']" 2>/dev/null)
assert_eq "$IS_PENDING" "true"
CHANGES=$(echo "$STATUS" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['changes']))" 2>/dev/null)
assert_gt "$CHANGES" 0

# Commit
COMMIT_RESP=$(api POST "/api/repos/$RID/commit" "$TK" '{"message":"test commit"}')
COMMIT_SUCC=$(echo "$COMMIT_RESP" | extract "['success']" 2>/dev/null)
assert_eq "$COMMIT_SUCC" "true"

# Path traversal protection
TRAV_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "$BASE/api/repos/$RID/files?path=../../../etc/passwd" \
    -H "Authorization: Bearer $TK" \
    -H "Content-Type: application/json" \
    -d "hack")
assert_eq "$TRAV_STATUS" "400"

summary
