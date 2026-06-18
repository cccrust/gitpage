#!/bin/bash
source test/lib.sh
echo "=== 12-ssh-keys ==="
TK=$(register_user "ssh12"); assert_not_empty "$TK"
RID=$(create_repo "$TK" "ssh-repo"); assert_gt "$RID" 0

# Add SSH key (response: { "success": true, "ssh_key": SshKey })
KEY_RESP=$(api POST "/api/repos/$RID/ssh-keys" "$TK" '{"name":"mykey","public_key":"ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQC..."}')
KEY_ID=$(echo "$KEY_RESP" | extract "['ssh_key']['id']" 2>/dev/null)
assert_gt "$KEY_ID" 0

# List SSH keys (response: { "ssh_keys": [...] })
KEYS=$(api GET "/api/repos/$RID/ssh-keys" "$TK")
KEY_COUNT=$(echo "$KEYS" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['ssh_keys']))" 2>/dev/null)
assert_gt "$KEY_COUNT" 0

# Delete SSH key (response: { "success": true })
DEL_RESP=$(api DELETE "/api/repos/$RID/ssh-keys/$KEY_ID" "$TK")
DEL_SUCC=$(echo "$DEL_RESP" | extract "['success']" 2>/dev/null)
assert_eq "$DEL_SUCC" "true"

# Verify deleted
KEYS_AFTER=$(api GET "/api/repos/$RID/ssh-keys" "$TK")
AFTER_COUNT=$(echo "$KEYS_AFTER" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['ssh_keys']))" 2>/dev/null)
assert_eq "$AFTER_COUNT" "0"

summary
