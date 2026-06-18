#!/bin/bash
source test/lib.sh
echo "=== 14-error-paths ==="
TK=$(register_user "err14"); assert_not_empty "$TK"

# 401 - no token
assert_status GET "/api/repos" "" "" "" "401"

# 404 - nonexistent repo
assert_status GET "/api/repos/999999" "$TK" "" "" "404"

# 404 - nonexistent endpoint
assert_status GET "/api/nonexistent" "$TK" "" "" "404"

# 404 - nonexistent user content
assert_status GET "/api/nobody99/norepo/tree?branch=main" "$TK" "" "" "404"

# 400 - empty repo name on create
assert_status POST "/api/repos" "$TK" '{"name":""}' "400"

# 400 - too short password
assert_status POST "/api/auth/register" "" '{"username":"badpwuser","email":"bad@test.com","password":"ab"}' "400"

# 400 - missing username on register
assert_status POST "/api/auth/register" "" '{"email":"x@test.com","password":"pass123"}' "400"

# 409 - duplicate registration
REG_RESP=$(api_raw POST "/api/auth/register" "" '{"username":"duperr","email":"dup@test.com","password":"pass123"}')
assert_status POST "/api/auth/register" "" '{"username":"duperr","email":"dup2@test.com","password":"pass123"}' "409"

summary
