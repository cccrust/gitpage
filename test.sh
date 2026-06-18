#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== Gitpage Integration Test Suite (legacy wrapper) ==="
echo "Delegating to test/run_all.sh..."
echo ""

cd "$SCRIPT_DIR"
exec bash test/run_all.sh
