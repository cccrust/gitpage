#!/bin/bash
source test/lib.sh
echo "=== 02-repo ==="
TK=$(register_user "repo02"); assert_not_empty "$TK"

# Create
RID=$(create_repo "$TK" "myrepo"); assert_gt "$RID" 0

# List
REPOS=$(api GET "/api/repos" "$TK")
COUNT=$(echo "$REPOS" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['repos']))")
assert_gt "$COUNT" 0

# Public list
PUB=$(api GET "/api/users/repo02-$TIMESTAMP/repos" "")
PUB_COUNT=$(echo "$PUB" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['repos']))")
assert_gt "$PUB_COUNT" 0

# Duplicate name fails (returns 400 or 409)
DUP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/api/repos" \
    -H "Authorization: Bearer $TK" \
    -H "Content-Type: application/json" \
    -d "{\"name\":\"myrepo\"}")
# Accept either 400 or 409 depending on error handler
if [ "$DUP_STATUS" = "409" ] || [ "$DUP_STATUS" = "400" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
else
    FAIL_COUNT=$((FAIL_COUNT + 1))
    echo "FAIL (line $BASH_LINENO): duplicate status $DUP_STATUS, expected 400 or 409"
fi

# Get single repo
REPO=$(api GET "/api/repos/$RID" "$TK")
RNAME=$(echo "$REPO" | extract "['repo']['name']")
assert_eq "$RNAME" "myrepo"

# Search
SEARCH=$(api GET "/api/repos/search?q=myrepo" "$TK")
SEARCH_TOTAL=$(echo "$SEARCH" | extract "['total']")
assert_gt "$SEARCH_TOTAL" 0

# Update repo
api PUT "/api/repos/$RID" "$TK" '{"name":"myrepo-renamed"}' > /dev/null
UPDATED=$(api GET "/api/repos/$RID" "$TK")
NEW_NAME=$(echo "$UPDATED" | extract "['repo']['name']")
assert_eq "$NEW_NAME" "myrepo-renamed"

# Delete
DEL_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE "$BASE/api/repos/$RID" -H "Authorization: Bearer $TK")
assert_eq "$DEL_STATUS" "200"

summary
