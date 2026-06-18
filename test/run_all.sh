#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

cleanup() {
    pkill -f gitpage 2>/dev/null || true
    rm -rf /tmp/gptest-*
}
trap cleanup EXIT
cleanup

export TEST_PORT=${TEST_PORT:-8080}

echo "=== Integration Test Suite ==="
echo "Port: $TEST_PORT"
echo ""

start_server

for script in test/0*.sh; do
    echo ""
    echo "============================================"
    bash "$script"
    echo "============================================"
done

stop_server
cleanup
echo ""
echo "=== ALL INTEGRATION TESTS COMPLETED ==="
