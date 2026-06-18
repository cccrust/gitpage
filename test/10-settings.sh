#!/bin/bash
source test/lib.sh
echo "=== 10-settings ==="
TK=$(register_user "set10"); assert_not_empty "$TK"
RID=$(create_repo "$TK" "settings-repo"); assert_gt "$RID" 0

# Access Token (response: { "raw_token": "gpt_...", "token": AccessToken })
TOKEN_RESP=$(api POST "/api/user/tokens" "$TK" '{"name":"dev","scopes":["repo:read"]}')
RAW_TK=$(echo "$TOKEN_RESP" | extract "['raw_token']" 2>/dev/null)
if echo "$RAW_TK" | grep -q "^gpt_"; then
    PASS_COUNT=$((PASS_COUNT + 1))
else
    FAIL_COUNT=$((FAIL_COUNT + 1))
    echo "FAIL: token prefix not gpt_"
fi

# Collaborator (response: { "success": true })
BOB_TK=$(register_user "bob10")
BOB_ME=$(api GET "/api/auth/me" "$BOB_TK")
BOB_NAME=$(echo "$BOB_ME" | extract "['user']['username']")
COLLAB=$(api POST "/api/repos/$RID/collaborators" "$TK" "{\"username\":\"$BOB_NAME\",\"permission\":\"read\"}")
COLLAB_SUCC=$(echo "$COLLAB" | extract "['success']" 2>/dev/null)
assert_eq "$COLLAB_SUCC" "true"

# List collaborators (response: { "collaborators": [...] })
COLLABS=$(api GET "/api/repos/$RID/collaborators" "$TK")
COLLAB_COUNT=$(echo "$COLLABS" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['collaborators']))" 2>/dev/null)
assert_gt "$COLLAB_COUNT" 0

# Secret (response: { "secret": RepoSecret })
SECRET_RESP=$(api POST "/api/repos/$RID/secrets" "$TK" '{"name":"DB_PASS","value":"supersecret"}')
SECRET_NAME=$(echo "$SECRET_RESP" | extract "['secret']['name']" 2>/dev/null)
assert_eq "$SECRET_NAME" "DB_PASS"

# List secrets (response: { "secrets": [...] })
SECRETS=$(api GET "/api/repos/$RID/secrets" "$TK")
SECRET_COUNT=$(echo "$SECRETS" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['secrets']))" 2>/dev/null)
assert_gt "$SECRET_COUNT" 0

# Branch protection (response: { "branch_protection": BranchProtection })
BP=$(api POST "/api/repos/$RID/branch-protections" "$TK" '{"pattern":"main","require_pull_request":true,"require_approvals":1}')
assert_not_empty "$BP"

# List branch protections (response: { "branch_protections": [...] })
BPS=$(api GET "/api/repos/$RID/branch-protections" "$TK")
BP_COUNT=$(echo "$BPS" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['branch_protections']))" 2>/dev/null)
assert_gt "$BP_COUNT" 0

summary
