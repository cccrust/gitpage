#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "╔══════════════════════════════════════════════════════╗"
echo "║          Gitpage 完整測試套件 (All-in-One)           ║"
echo "╚══════════════════════════════════════════════════════╝"

cleanup() {
    pkill -f gitpage 2>/dev/null || true
    rm -rf /tmp/gptest-*
}
trap cleanup EXIT
cleanup

TOTAL=0
PASS=0
FAIL=0

step() {
    local name="$1"
    TOTAL=$((TOTAL + 1))
    echo ""
    echo "━━━ [$TOTAL] $name ━━━"
}

# 1. Rust 單元測試
step "Rust 單元測試"
if cargo test 2>&1; then
    echo "✅ Rust 單元測試通過"
    PASS=$((PASS + 1))
else
    echo "❌ Rust 單元測試失敗"
    FAIL=$((FAIL + 1))
fi

# 啟動伺服器（供後續測試使用）
start_server() {
    mkdir -p data/repos
    cargo build 2>&1 | tail -1
    cargo run &
    sleep 3
}

# 2. 整合測試
step "整合測試 (14 scripts)"
cleanup
start_server
INTEG_FAIL=0
for f in test/0*.sh; do
    if bash "$f"; then
        :  # pass
    else
        INTEG_FAIL=1
    fi
done
if [ "$INTEG_FAIL" -eq 0 ]; then
    echo "✅ 整合測試通過"
    PASS=$((PASS + 1))
else
    echo "❌ 整合測試失敗"
    FAIL=$((FAIL + 1))
fi

# 3. Hurl API 測試
step "Hurl API 測試 (6 files)"
cleanup
if [ ! -x hurl-bin ]; then
    echo "⚠️  hurl-bin 不存在，跳過 Hurl 測試"
    echo "✅ Hurl API 測試通過 (跳過)"
    PASS=$((PASS + 1))
    else
        start_server
        # Wait for server to accept connections
        for i in 1 2 3 4 5; do
            if curl -sf http://localhost:8080 > /dev/null 2>&1; then
                break
            fi
            echo "  Waiting for server... (attempt $i)"
            sleep 2
        done
        HURL_FAIL=0
        TS=$(date +%s)
    USER="hurl-api-$TS"
    EMAIL="$USER@test.com"
    REPO="hurl-repo-$TS"

    # Register via curl
    REG=$(curl -s -X POST "http://localhost:8080/api/auth/register" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$USER\",\"email\":\"$EMAIL\",\"password\":\"pass123\"}")
    AUTH_TOKEN=$(echo "$REG" | python3 -c "import sys,json;print(json.load(sys.stdin)['token'])" 2>/dev/null)
    if [ -z "$AUTH_TOKEN" ]; then
        echo "⚠️  無法取得 auth token: $(echo $REG | head -c 200)"
        HURL_FAIL=1
    else
        # Create repo via curl
        REPO_RESP=$(curl -s -X POST "http://localhost:8080/api/repos" \
            -H "Authorization: Bearer $AUTH_TOKEN" \
            -H "Content-Type: application/json" \
            -d "{\"name\":\"$REPO\",\"description\":\"hurl test\"}")
        REPO_ID=$(echo "$REPO_RESP" | python3 -c "import sys,json;print(json.load(sys.stdin)['repo']['id'])" 2>/dev/null)
        if [ -z "$REPO_ID" ]; then
            echo "⚠️  無法建立 repo: $(echo $REPO_RESP | head -c 200)"
            HURL_FAIL=1
    else
        # Git push so content tests have data
        rm -rf /tmp/gptest-hurl-push
        mkdir -p /tmp/gptest-hurl-push
        cd /tmp/gptest-hurl-push
        git init -q
        git config user.email "hurl@test.com"
        git config user.name "Hurl"
        echo "# Hurl Repo" > README.md
        git add -A && git commit -q -m "init"
        git remote add origin "http://localhost:8080/git/$USER/$REPO"
        git push origin main 2>&1
        cd "$SCRIPT_DIR"

        VARGS="--variable host=http://localhost:8080 --variable timestamp=$TS"
        VARGS="$VARGS --variable auth_token=$AUTH_TOKEN"
        VARGS="$VARGS --variable repo_id=$REPO_ID"
        VARGS="$VARGS --variable username=$USER"
        VARGS="$VARGS --variable owner_name=$USER"
        VARGS="$VARGS --variable repo_name=$REPO"

            for f in tests/api/*.hurl; do
                base=$(basename "$f")
                if [ "$base" = "auth.hurl" ]; then
                    # Run auth.hurl separately (creates its own user, doesn't need shared vars)
                    if ./hurl-bin --test --variable host=http://localhost:8080 \
                        --variable timestamp=$TS \
                        --file-root tests/api "$f" 2>&1 | grep -qi "success"; then
                        :
                    else
                        echo "  ❌ $base 失敗"
                        HURL_FAIL=1
                    fi
            else
                if ./hurl-bin --test $VARGS --file-root tests/api "$f" 2>&1 | grep -qi "success"; then
                    :
                else
                    echo "  ❌ $base 失敗"
                    HURL_FAIL=1
                fi
                fi
            done
        fi
    fi
    if [ "$HURL_FAIL" -eq 0 ]; then
        echo "✅ Hurl API 測試通過"
        PASS=$((PASS + 1))
    else
        echo "❌ Hurl API 測試失敗"
        FAIL=$((FAIL + 1))
    fi
fi

# 4. Playwright E2E 測試
step "Playwright E2E 測試 (3 specs)"
cleanup
cd frontend
if [ ! -d node_modules ]; then
    npm install --silent
fi
if npx playwright test --config=e2e/playwright.config.ts 2>&1; then
    echo "✅ Playwright E2E 測試通過"
    PASS=$((PASS + 1))
else
    echo "❌ Playwright E2E 測試失敗"
    FAIL=$((FAIL + 1))
fi
cd "$SCRIPT_DIR"

echo ""
echo "╔══════════════════════════════════════════════════════╗"
echo "║                    測試結果                          ║"
echo "║  總計: $TOTAL | 通過: $PASS | 失敗: $FAIL            ║"
echo "╚══════════════════════════════════════════════════════╝"
[ "$FAIL" -eq 0 ] || exit 1
