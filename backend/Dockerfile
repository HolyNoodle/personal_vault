# Build frontend
FROM node:24-alpine AS frontend-builder

WORKDIR /app/frontend/web

COPY frontend/web/package.json frontend/web/package-lock.json ./
RUN npm ci

COPY frontend/web/ ./
RUN npm run build

# Build WASM app
FROM rust:1.93-slim AS wasm-builder

WORKDIR /app

RUN rustup target add wasm32-unknown-unknown

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared/ ./shared/
COPY apps/file-explorer/ ./apps/file-explorer/

# Build the file-explorer WASM module
RUN cargo build --target wasm32-unknown-unknown --release -p file-explorer

# Build backend
FROM rust:1.93-slim AS backend-builder

WORKDIR /app

# Install build dependencies (including GStreamer dev libs)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared/ ./shared/
COPY apps/file-explorer/ ./apps/file-explorer/

# Copy backend manifests and source
COPY backend/ ./backend/

# Build release binary
RUN cargo build --release --bin sandbox-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    postgresql-client \
    libgstreamer1.0-0 \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-libav \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 sandbox && \
    mkdir -p /var/lib/sandbox/storage /var/log/sandbox /app/wasm && \
    chown -R sandbox:sandbox /var/lib/sandbox /var/log/sandbox /app/wasm

WORKDIR /app

# Copy binary from backend builder
COPY --from=backend-builder /app/target/release/sandbox-server /usr/local/bin/sandbox-server

# Copy WASM binary from wasm builder
COPY --from=wasm-builder /app/target/wasm32-unknown-unknown/release/file_explorer.wasm /app/wasm/file_explorer.wasm

# Copy frontend static files from frontend builder
COPY --from=frontend-builder /app/frontend/web/dist /app/static

# Set WASM directory
ENV WASM_DIR=/app/wasm

# Switch to non-root user
USER sandbox

EXPOSE 8080

CMD ["sandbox-server"]
