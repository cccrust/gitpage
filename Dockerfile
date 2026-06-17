# ============================================================
# Stage 1: Frontend build (Node)
# ============================================================
FROM node:22-bookworm-slim AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ .
RUN npx vite build

# ============================================================
# Stage 2: Backend build (Rust)
# ============================================================
FROM rust:1-slim-bookworm AS backend
WORKDIR /app

# System deps (rarely changes)
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev cmake \
    && rm -rf /var/lib/apt/lists/*

# Cache Rust dependencies (only rerun when Cargo.toml/lock changes)
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo fetch && \
    cargo build --release 2>&1; \
    rm -rf src

# Copy real source + frontend (changes frequently)
COPY src/ src/
COPY --from=frontend /app/frontend/dist/ frontend/dist/
RUN touch src/main.rs && \
    cargo build --release 2>&1

# ============================================================
# Stage 3: Runtime
# ============================================================
# Build base image first: docker build -t gitpage-dev-base:latest -f Dockerfile.base .
FROM gitpage-dev-base:latest

COPY --from=backend /app/target/release/gitpage /usr/local/bin/gitpage
COPY --from=frontend /app/frontend/dist/ /app/frontend/dist/
COPY config.toml /app/config.toml
COPY entrypoint.sh /entrypoint.sh

WORKDIR /app

VOLUME ["/app/data"]

EXPOSE 22 8080

ENTRYPOINT ["/entrypoint.sh"]
CMD []
