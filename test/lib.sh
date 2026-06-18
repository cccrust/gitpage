# Shared test library for Gitpage integration tests
# Usage: source test/lib.sh

BASE="http://localhost:${TEST_PORT:-8080}"
TIMESTAMP=$(date +%s)
PASS_COUNT=0
FAIL_COUNT=0

api() {
    curl -s -X "$1" "$BASE$2" \
        ${3:+-H "Authorization: Bearer $3"} \
        ${4:+-H "Content-Type: application/json" -d "$4"}
}

api_raw() {
    curl -s -X "$1" "$BASE$2" \
        ${3:+-H "Authorization: Bearer $3"} \
        ${4:+-H "Content-Type: application/json" -d "$4"}
}

extract() {
    python3 -c "import sys,json;v=json.load(sys.stdin)${1};print(v if isinstance(v,str) else json.dumps(v))"
}

register_user() {
    local u=$1
    api_raw POST "/api/auth/register" "" \
        "{\"username\":\"$u-$TIMESTAMP\",\"email\":\"$u@$TIMESTAMP.com\",\"password\":\"pass123\"}" \
        | extract "['token']"
}

register_user_fixed() {
    api_raw POST "/api/auth/register" "" \
        "{\"username\":\"$1\",\"email\":\"$1@test.com\",\"password\":\"pass123\"}" \
        | extract "['token']"
}

create_repo() {
    api POST "/api/repos" "$1" "{\"name\":\"$2\",\"description\":\"test\"}" \
        | extract "['repo']['id']"
}

assert_eq() {
    if [ "$1" = "$2" ]; then
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        FAIL_COUNT=$((FAIL_COUNT + 1))
        echo "FAIL (line $BASH_LINENO): '$1' != '$2'"
    fi
}

assert_neq() {
    if [ "$1" != "$2" ]; then
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        FAIL_COUNT=$((FAIL_COUNT + 1))
        echo "FAIL (line $BASH_LINENO): '$1' == '$2' (should differ)"
    fi
}

assert_status() {
    local s
    s=$(curl -s -o /dev/null -w "%{http_code}" -X "$1" "$BASE$2" \
        ${3:+-H "Authorization: Bearer $3"} \
        ${4:+-H "Content-Type: application/json" -d "$4"})
    assert_eq "$s" "${5:-200}"
}

assert_gt() {
    if [ "$1" -gt "$2" ] 2>/dev/null; then
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        FAIL_COUNT=$((FAIL_COUNT + 1))
        echo "FAIL (line $BASH_LINENO): $1 <= $2"
    fi
}

assert_not_empty() {
    if [ -n "$1" ]; then
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        FAIL_COUNT=$((FAIL_COUNT + 1))
        echo "FAIL (line $BASH_LINENO): empty value"
    fi
}

summary() {
    echo ""
    echo "=== RESULTS: $PASS_COUNT passed, $FAIL_COUNT failed ==="
    [ "$FAIL_COUNT" -eq 0 ] || exit 1
}

start_server() {
    mkdir -p data/repos
    cargo build 2>&1 | tail -1
    cargo run &
    SERVER_PID=$!
    sleep 3
}

stop_server() {
    kill $SERVER_PID 2>/dev/null || true
    pkill -f gitpage 2>/dev/null || true
}
