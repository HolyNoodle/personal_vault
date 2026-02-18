# Development Dockerfile with hot-reloading
FROM rust:1.93-slim

WORKDIR /app

# Install development dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    postgresql-client \
    libglib2.0-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libgstreamer-plugins-bad1.0-dev \
    gstreamer1.0-tools \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    mold \
    clang \
    git \
    nodejs \
    npm \
    xvfb \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-watch for hot-reloading
RUN cargo install cargo-watch

# Install Claude Code CLI
RUN npm install -g @anthropic-ai/claude-code pm2


# Set target directory for build cache
ENV CARGO_TARGET_DIR=/app/target

# Expose port
EXPOSE 8080




# Ensure /tmp/ipc exists and is world-writable for IPC server
RUN mkdir -p /tmp/ipc && chmod 1777 /tmp/ipc

# Use host user for permissions
ARG USER_ID=1000
ARG GROUP_ID=1000
RUN groupadd -g $GROUP_ID sandbox \
    && useradd -m -u $USER_ID -g $GROUP_ID sandbox
RUN chown -R $USER_ID:$GROUP_ID /usr/local/cargo
RUN mkdir -p /app/target /app/frontend/web/node_modules \
    && chown $USER_ID:$GROUP_ID /app/target /app/frontend/web/node_modules

USER sandbox

# Run with hot-reloading
CMD ["cargo", "watch", "-x", "run"]
