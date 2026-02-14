# Build frontend
FROM node:20-alpine AS frontend-builder

WORKDIR /app/frontend/web

COPY frontend/web/package.json frontend/web/package-lock.json ./
RUN npm ci

COPY frontend/web/ ./
RUN npm run build

# Build backend
FROM rust:1.75-slim AS backend-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy backend manifests
COPY backend/Cargo.toml backend/Cargo.lock ./

# Cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy backend source
COPY backend/ ./

# Build release binary
RUN cargo build --release --bin sandbox-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    postgresql-client \
    xvfb \
    ffmpeg \
    xterm \
    thunar \
    dbus-x11 \
    wmctrl \
    openbox \
    xdotool \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 sandbox && \
    mkdir -p /var/lib/sandbox/storage /var/log/sandbox && \
    chown -R sandbox:sandbox /var/lib/sandbox /var/log/sandbox

WORKDIR /app

# Copy binary from backend builder
COPY --from=backend-builder /app/target/release/sandbox-server /usr/local/bin/sandbox-server

# Copy frontend static files from frontend builder
COPY --from=frontend-builder /app/frontend/web/dist /app/static

# Switch to non-root user
USER sandbox

EXPOSE 8080

CMD ["sandbox-server"]
