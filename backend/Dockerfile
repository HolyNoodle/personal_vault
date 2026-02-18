# Build frontend
FROM node:24-alpine AS frontend-builder

WORKDIR /app/frontend/web

COPY frontend/web/package.json frontend/web/package-lock.json ./
RUN npm ci

COPY frontend/web/ ./
RUN npm run build

# Build backend
FROM rust:1.93-slim AS backend-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libx11-dev \
    libxrandr-dev \
    libxi-dev \
    libxrender-dev \
    libxcomposite-dev \
    libxfixes-dev \
    libxdamage-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared/ ./shared/
COPY apps/file-explorer/ ./apps/file-explorer/

# Copy backend manifests and source
COPY backend/ ./backend/

# Build release binaries (backend server + file-explorer native app)
RUN cargo build --release --bin sandbox-server --bin file_explorer

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
    xvfb \
    libx11-6 \
    libxrandr2 \
    libxi6 \
    libxrender1 \
    libxcomposite1 \
    libxfixes3 \
    libxdamage1 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 sandbox && \
    mkdir -p /var/lib/sandbox/storage /var/log/sandbox /app/.app/file_explorer && \
    chown -R sandbox:sandbox /var/lib/sandbox /var/log/sandbox /app/.app

WORKDIR /app

# Copy binaries from backend builder
COPY --from=backend-builder /app/target/release/sandbox-server /usr/local/bin/sandbox-server
COPY --from=backend-builder /app/target/release/file_explorer /app/.app/file_explorer/file_explorer

# Copy frontend static files from frontend builder
COPY --from=frontend-builder /app/frontend/web/dist /app/static

# Set apps root directory
ENV APPS_ROOT=/app/.app

# Switch to non-root user
USER sandbox

EXPOSE 8080

CMD ["sandbox-server"]
