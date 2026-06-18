# build_docker.sh — Docker Build Helper

## Overview

`build_docker.sh` is a minimal convenience script for building the Docker images. It documents (both in comments and in actual execution) the two-step build process required for Gitpage's Docker setup.

## The Two-Step Build

Gitpage uses two Docker images:

1. **`gitpage-dev-base:latest`** (from `Dockerfile.base`) — Base image with all development toolchains: Python, Rust, Node.js, OpenSSH server, git, and the `opencode` CLI. This is **large and slow to build** (~5–15 min first time) but changes rarely.

2. **`gitpage:latest`** (from `Dockerfile`) — The application image. Uses a multi-stage build that copies only the compiled binary, frontend assets, and config into the base image. This is **fast** (seconds) once the base image exists.

## Script Execution

```bash
set -x
docker build -t gitpage-dev-base:latest -f Dockerfile.base .
./run_docker.sh --build
```

1. Builds the base image explicitly (in case it doesn't exist).
2. Delegates app image build + container start to `run_docker.sh --build`.

The comments document the normal iterative workflow:

```bash
# After the first slow build, subsequent runs are fast:
# ./run_docker.sh          # Uses existing image (no rebuild)
```

## Why a Separate Script?

- **Documentation**: The script's comments explain the build sequence and expected timing.
- **Speed**: Running `docker build` for the base image separately lets you watch its progress without the app build output mixed in.
- **Manual control**: Some users may want to customize the base image (add packages) and rebuild only that layer.
- **CI integration**: CI pipelines can build and push the base image once, then rebuild the app image on every commit.

## Usage

```bash
# Full build (base + app):
./build_docker.sh

# After the first time, just rebuild the app:
./run_docker.sh --build

# Or skip build entirely and use cached image:
./run_docker.sh
```

## References

- `run_docker.sh` — Full runner (build + start container with mounts). The `--build` flag forces app image rebuild.
- `Dockerfile` — Multi-stage app image build.
- `Dockerfile.base` — Base image with toolchains.
