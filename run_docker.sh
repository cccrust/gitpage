#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

BASE_IMAGE="gitpage-dev-base:latest"
IMAGE="gitpage:latest"
PORT=${PORT:-8080}
SSH_PORT=${SSH_PORT:-2222}
DATA_DIR="$SCRIPT_DIR/data"

# Build base image if missing (dev tools: uv+python, rustup+rust, node, etc.)
if ! docker image inspect "$BASE_IMAGE" >/dev/null 2>&1; then
  echo "=== Build base image (dev tools) — slow, once ==="
  docker build -t "$BASE_IMAGE" -f Dockerfile.base .
fi

# Build app image only if missing, or force with --build
if [ "$1" = "--build" ] || ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  echo "=== Build app image ==="
  docker build -t "$IMAGE" .
else
  echo "=== Using existing image $IMAGE (pass --build to rebuild) ==="
fi

echo ""
echo "=== Prepare data directory ==="
mkdir -p "$DATA_DIR/repos" "$DATA_DIR/staging" "$DATA_DIR/apps"

# Kill existing container if any
docker rm -f gitpage 2>/dev/null || true

echo ""
echo "=== Start container ==="
echo "Open  http://localhost:$PORT"
echo "SSH   ssh alice@localhost -p $SSH_PORT  (password: alice123)"
echo "SSH   ssh bob@localhost -p $SSH_PORT   (password: bob123)"
echo "SSH   ssh root@localhost -p $SSH_PORT  (password: gitpage)"
echo ""

exec docker run --rm --name gitpage \
  -p "$PORT:8080" \
  -p "$SSH_PORT:22" \
  -v "$DATA_DIR:/app/data" \
  -v "gitpage-ssh-keys:/etc/ssh" \
  -e RUST_LOG=info \
  -e SSH_USERS="alice:alice123,bob:bob123" \
  "$IMAGE"
