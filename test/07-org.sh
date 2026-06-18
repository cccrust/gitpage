#!/bin/bash
source test/lib.sh
echo "=== 07-org ==="
TK=$(register_user "org07"); assert_not_empty "$TK"

# Create org
ORG_RESP=$(api POST "/api/orgs" "$TK" "{\"name\":\"myorg07-$TIMESTAMP\",\"display_name\":\"My Org\",\"description\":\"test\"}")
ORG_NAME=$(echo "$ORG_RESP" | extract "['org']['name']")
assert_eq "$ORG_NAME" "myorg07-$TIMESTAMP"

# Duplicate org fails
DUP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/api/orgs" \
    -H "Authorization: Bearer $TK" \
    -H "Content-Type: application/json" \
    -d "{\"name\":\"myorg07-$TIMESTAMP\"}")
assert_eq "$DUP_STATUS" "409"

# Get org
ORG=$(api GET "/api/orgs/myorg07-$TIMESTAMP" "")
assert_not_empty "$ORG"

# List my orgs
MY_ORGS=$(api GET "/api/orgs" "$TK")
ORG_COUNT=$(echo "$MY_ORGS" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['orgs']))")
assert_gt "$ORG_COUNT" 0

# Create repo under org
ORG_REPO=$(api POST "/api/repos" "$TK" "{\"name\":\"orgrepo\",\"description\":\"org repo\",\"org_name\":\"myorg07-$TIMESTAMP\"}")
ORG_RID=$(echo "$ORG_REPO" | extract "['repo']['id']")
assert_gt "$ORG_RID" 0

# List org repos
ORG_REPOS=$(api GET "/api/orgs/myorg07-$TIMESTAMP/repos" "")
assert_not_empty "$ORG_REPOS"

# Register bob and add as member
BOB_TK=$(register_user "bob07")
BOB_ME=$(api GET "/api/auth/me" "$BOB_TK")
BOB_ID=$(echo "$BOB_ME" | extract "['user']['id']")

ADD_RESP=$(api POST "/api/orgs/myorg07-$TIMESTAMP/members" "$TK" "{\"username\":\"bob07-$TIMESTAMP\"}")
assert_not_empty "$ADD_RESP"

# List members
MEMBERS=$(api GET "/api/orgs/myorg07-$TIMESTAMP/members" "$TK")
MEMBER_COUNT=$(echo "$MEMBERS" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['members']))")
assert_gt "$MEMBER_COUNT" 0

# Delete repo
api DELETE "/api/repos/$ORG_RID" "$TK" > /dev/null

summary
