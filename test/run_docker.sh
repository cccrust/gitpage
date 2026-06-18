#!/bin/bash
set -ex
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

IMAGE="gitpage:test"
CONTAINER="gptest-docker"
PORT=18080
DATA_DIR="/tmp/gptest-docker-data"

cleanup() {
    docker rm -f "$CONTAINER" 2>/dev/null
    rm -rf "$DATA_DIR"
}
trap cleanup EXIT
cleanup

echo "=== Build Docker images ==="
docker build -t gitpage-dev-base:latest -f Dockerfile.base .
docker build -t "$IMAGE" .

echo "=== Start container ==="
mkdir -p "$DATA_DIR/repos"
docker run -d --name "$CONTAINER" \
  -p "$PORT:8080" \
  -v "$DATA_DIR:/app/data" \
  -e RUST_LOG=info \
  "$IMAGE"
sleep 3

export TEST_PORT=$PORT

for script in test/0*.sh; do
    echo ""
    echo "=== $(basename "$script") ==="
    bash "$script" 2>&1
done

cleanup
echo "=== ALL DOCKER TESTS PASSED ==="
