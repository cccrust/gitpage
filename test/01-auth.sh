#!/bin/bash
source test/lib.sh
echo "=== 01-auth ==="
START_TK=$(register_user "auth01")
assert_not_empty "$START_TK"
TK=$START_TK

# Login
RESP=$(api_raw POST "/api/auth/login" "" "{\"username\":\"auth01-$TIMESTAMP\",\"password\":\"pass123\"}")
TK2=$(echo "$RESP" | extract "['token']")
assert_not_empty "$TK2"

# Me
ME=$(api GET "/api/auth/me" "$TK")
USERNAME=$(echo "$ME" | extract "['user']['username']")
assert_eq "$USERNAME" "auth01-$TIMESTAMP"

# Duplicate register fails
DUP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/api/auth/register" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"auth01-$TIMESTAMP\",\"email\":\"dup@test.com\",\"password\":\"pass123\"}")
assert_eq "$DUP_STATUS" "409"

# No-token access rejected
UNAUTH_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X GET "$BASE/api/auth/me")
assert_eq "$UNAUTH_STATUS" "401"

# Bad password login fails
BAD_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/api/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"auth01-$TIMESTAMP\",\"password\":\"wrongpass\"}")
assert_eq "$BAD_STATUS" "401"

summary
