#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

PORT=8080

# Kill anything already on the port
lsof -ti tcp:$PORT 2>/dev/null | xargs kill -9 2>/dev/null || true
sleep 1

echo "=== Build frontend ==="
cd frontend
npm install --silent 2>/dev/null
npx vite build
cd ..

echo ""
echo "=== Build backend ==="
cargo build --release 2>&1 | tail -3

echo ""
echo "=== Start server ==="
echo "Open http://localhost:$PORT"
echo ""

exec cargo run --release
