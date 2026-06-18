#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJ_DIR="$SCRIPT_DIR/.."
HURL="$PROJ_DIR/hurl-bin"

if [ ! -x "$HURL" ]; then
    echo "hurl-bin not found. Run: brew install hurl or download from https://hurl.dev"
    exit 1
fi

CLEANUP=${CLEANUP:-1}
PORT=${PORT:-8080}
HOST="http://localhost:$PORT"
TIMESTAMP=$(date +%s)
PASS=0
FAIL=0

cleanup() {
    [ "$CLEANUP" = "1" ] && pkill -f gitpage 2>/dev/null || true
}
trap cleanup EXIT

# Start server if not running
if ! curl -sf "$HOST" > /dev/null 2>&1; then
    echo "Starting Gitpage server on :$PORT..."
    cd "$PROJ_DIR"
    cargo build 2>&1 | tail -1
    cargo run &
    SERVER_PID=$!
    sleep 3
fi

echo "=== Hurl API Tests ==="
echo "Host: $HOST  Timestamp: $TIMESTAMP"

# Register user via curl (more reliable than hurl capture extraction)
USERNAME="hurl-$TIMESTAMP"
REG=$(curl -s -X POST "$HOST/api/auth/register" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"$USERNAME\",\"email\":\"$USERNAME@test.com\",\"password\":\"pass123\"}")

AUTH_TOKEN=$(echo "$REG" | python3 -c "import sys,json;print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

if [ -z "$AUTH_TOKEN" ]; then
    echo "ERROR: Could not register user"
    exit 1
fi
echo "Auth token obtained"

# Run all hurl files
for f in "$SCRIPT_DIR/api/"*.hurl; do
    base=$(basename "$f")
    echo ""
    echo "--- $base ---"
    # Run auth.hurl with its own register (uses different timestamp but that's fine)
    if [ "$base" = "auth.hurl" ]; then
        # Run auth.hurl separately since it creates its own user
        if $HURL --test --variable host="$HOST" --variable timestamp="$TIMESTAMP" \
            --file-root "$SCRIPT_DIR/api" "$f" 2>&1; then
            PASS=$((PASS + 1))
        else
            FAIL=$((FAIL + 1))
        fi
    else
        if $HURL --test --variable host="$HOST" --variable timestamp="$TIMESTAMP" \
            --variable auth_token="$AUTH_TOKEN" \
            --file-root "$SCRIPT_DIR/api" "$f" 2>&1; then
            PASS=$((PASS + 1))
        else
            FAIL=$((FAIL + 1))
        fi
    fi
done

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
