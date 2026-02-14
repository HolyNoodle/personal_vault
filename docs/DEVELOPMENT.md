# Development Guide

## Prerequisites

### System Requirements

**Operating System:**
- Linux kernel 5.13+ (required for Landlock LSM)
- Ubuntu 22.04+ or Debian 12+ recommended
- Fedora 36+ or Arch Linux also supported

**Check Kernel Version:**
```bash
uname -r  # Should be >= 5.13
```

**Enable User Namespaces (if disabled):**
```bash
# Check current setting
cat /proc/sys/kernel/unprivileged_userns_clone

# Enable if it shows 0
sudo sysctl -w kernel.unprivileged_userns_clone=1

# Persist across reboots
echo "kernel.unprivileged_userns_clone=1" | sudo tee -a /etc/sysctl.conf
```

**Required Packages:**
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    postgresql \
    postgresql-contrib \
    ffmpeg \
    xvfb \
    x11-utils \
    xdotool \
    openbox \
    fonts-liberation

# Fedora
sudo dnf install -y \
    gcc \
    openssl-devel \
    postgresql-server \
    postgresql-contrib \
    ffmpeg \
    xorg-x11-server-Xvfb \
    xdotool \
    openbox \
    liberation-fonts
```

### Rust Toolchain

**Install Rust (via rustup):**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Version Requirements:**
- Rust 1.75+ (for latest async features)
- Edition 2021

**Verify Installation:**
```bash
rustc --version  # Should be >= 1.75.0
cargo --version
```

**Recommended Tools:**
```bash
# Code formatter
rustup component add rustfmt

# Linter
rustup component add clippy

# Security auditing
cargo install cargo-audit

# Dependency tree viewer
cargo install cargo-tree

# Watch and rebuild
cargo install cargo-watch
```

## Project Setup

### Clone Repository

```bash
git clone https://github.com/yourorg/secure-sandbox-server.git
cd secure-sandbox-server
```

### Database Setup

**Start PostgreSQL:**
```bash
# Ubuntu/Debian
sudo systemctl start postgresql
sudo systemctl enable postgresql

# Fedora
sudo postgresql-setup --initdb
sudo systemctl start postgresql
sudo systemctl enable postgresql
```

**Create Database and User:**
```bash
sudo -u postgres psql
```

```sql
CREATE DATABASE sandbox_server;
CREATE USER sandbox_user WITH ENCRYPTED PASSWORD 'change_me_in_production';
GRANT ALL PRIVILEGES ON DATABASE sandbox_server TO sandbox_user;
\q
```

**Run Migrations:**
```bash
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations
sqlx migrate run
```

### Environment Configuration

**Create `.env` file:**
```bash
cp .env.example .env
```

**Edit `.env`:**
```bash
# Database
DATABASE_URL=postgresql://sandbox_user:change_me_in_production@localhost/sandbox_server

# Server
HOST=127.0.0.1
PORT=8080

# JWT Secret (generate with: openssl rand -base64 32)
JWT_SECRET=your_256_bit_secret_here

# JWT Expiry (seconds)
JWT_ACCESS_EXPIRY=900        # 15 minutes
JWT_REFRESH_EXPIRY=604800    # 7 days

# Sandbox Limits
SANDBOX_MEMORY_LIMIT_MB=512
SANDBOX_CPU_PERCENT=50
SANDBOX_PID_LIMIT=100
SANDBOX_SESSION_TIMEOUT_SECS=1800  # 30 minutes

# File Storage
FILE_STORAGE_PATH=/data/users
FILE_ENCRYPTION_ENABLED=true

# Video Encoding
VIDEO_FRAMERATE=30
VIDEO_BITRATE_KBPS=2000
VIDEO_CODEC=libx264
VIDEO_PRESET=ultrafast
VIDEO_TUNE=zerolatency

# Logging
RUST_LOG=info,sandbox_server=debug
LOG_FORMAT=json

# STUN Server
STUN_SERVER=stun:stun.l.google.com:19302
```

### Build Project

```bash
# Debug build (faster compilation, slower runtime)
cargo build

# Release build (optimized)
cargo build --release

# Check for errors without building
cargo check
```

## Development Workflow

### Running the Server

**Development Mode:**
```bash
# With auto-reload on file changes
cargo watch -x run

# Or manually
cargo run
```

**Production Mode:**
```bash
cargo run --release
```

**Access Server:**
- API: http://localhost:8080/api
- WebSocket: ws://localhost:8080/ws
- Health Check: http://localhost:8080/health

### Testing

**Run All Tests:**
```bash
cargo test
```

**Run Specific Test:**
```bash
cargo test test_namespace_creation
```

**Run with Output:**
```bash
cargo test -- --nocapture
```

**Run Integration Tests:**
```bash
cargo test --test '*'
```

**Run Tests in Single Thread (for namespace tests):**
```bash
cargo test -- --test-threads=1
```

**Coverage (using tarpaulin):**
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

### Linting and Formatting

**Format Code:**
```bash
cargo fmt
```

**Check Formatting:**
```bash
cargo fmt -- --check
```

**Run Clippy (linter):**
```bash
cargo clippy
```

**Clippy with Strict Mode:**
```bash
cargo clippy -- -D warnings
```

**Security Audit:**
```bash
cargo audit
```

### Database Migrations

**Create New Migration:**
```bash
sqlx migrate add create_users_table
```

**Edit Migration File:**
```sql
-- migrations/XXXXXX_create_users_table.up.sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

**Run Migrations:**
```bash
sqlx migrate run
```

**Revert Last Migration:**
```bash
sqlx migrate revert
```

## Project Structure

```
secure-sandbox-server/
├── src/
│   ├── main.rs                 # Entry point
│   ├── lib.rs                  # Library root
│   ├── config.rs               # Configuration management
│   ├── error.rs                # Error types
│   │
│   ├── server/                 # Web server layer
│   │   ├── mod.rs
│   │   ├── api.rs              # REST API routes
│   │   ├── websocket.rs        # WebSocket handlers
│   │   ├── auth.rs             # Authentication middleware
│   │   └── session.rs          # Session management
│   │
│   ├── sandbox/                # Sandbox engine
│   │   ├── mod.rs
│   │   ├── namespace.rs        # Namespace creation
│   │   ├── landlock.rs         # Filesystem policies
│   │   ├── cgroups.rs          # Resource limits
│   │   ├── seccomp.rs          # Syscall filtering
│   │   ├── mount.rs            # Mount orchestration
│   │   └── process.rs          # Process management
│   │
│   ├── video/                  # Video capture & encoding
│   │   ├── mod.rs
│   │   ├── capture.rs          # X11 screen capture
│   │   ├── encoder.rs          # H.264 encoding
│   │   └── pipeline.rs         # FFmpeg pipeline
│   │
│   ├── webrtc/                 # WebRTC module
│   │   ├── mod.rs
│   │   ├── peer.rs             # Peer connection
│   │   ├── signaling.rs        # SDP exchange
│   │   ├── media.rs            # Media tracks
│   │   └── ice.rs              # ICE handling
│   │
│   ├── input/                  # Input forwarding
│   │   ├── mod.rs
│   │   ├── events.rs           # Event types
│   │   ├── validator.rs        # Input validation
│   │   └── injector.rs         # X11 injection
│   │
│   ├── storage/                # File access control
│   │   ├── mod.rs
│   │   ├── permissions.rs      # RBAC logic
│   │   ├── encryption.rs       # File encryption
│   │   └── audit.rs            # Audit logging
│   │
│   └── db/                     # Database layer
│       ├── mod.rs
│       ├── models.rs           # Database models
│       ├── users.rs            # User CRUD
│       ├── permissions.rs      # Permission CRUD
│       └── sessions.rs         # Session CRUD
│
├── migrations/                 # SQL migrations
│   ├── 001_create_users.sql
│   ├── 002_create_permissions.sql
│   └── 003_create_sessions.sql
│
├── tests/                      # Integration tests
│   ├── api_tests.rs
│   ├── sandbox_tests.rs
│   └── webrtc_tests.rs
│
├── benches/                    # Benchmarks
│   └── sandbox_bench.rs
│
├── docs/                       # Documentation
│   ├── ARCHITECTURE.md
│   ├── SECURITY.md
│   ├── DEVELOPMENT.md
│   ├── API.md
│   └── DEPLOYMENT.md
│
├── client/                     # Web client (HTML/JS)
│   ├── index.html
│   ├── app.js
│   └── style.css
│
├── Cargo.toml                  # Rust dependencies
├── Cargo.lock                  # Dependency lock file
├── .env.example                # Environment template
├── .gitignore
├── README.md
└── LICENSE
```

## Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run
```

**Module-Specific Logging:**
```bash
RUST_LOG=sandbox_server::sandbox=trace,sandbox_server::webrtc=debug cargo run
```

### Debug Namespace Issues

**List Namespaces:**
```bash
sudo lsns
```

**Inspect Process Namespaces:**
```bash
sudo ls -la /proc/<pid>/ns/
```

**Enter Namespace (for debugging):**
```bash
sudo nsenter --target <pid> --all /bin/bash
```

### Debug Landlock Policies

**Check Kernel Support:**
```bash
cat /sys/kernel/security/lsm
# Should include "landlock"
```

**Trace Landlock Denials:**
```bash
sudo dmesg | grep landlock
```

### Debug cgroups

**List cgroups:**
```bash
ls /sys/fs/cgroup/
```

**Check Limits:**
```bash
cat /sys/fs/cgroup/sandbox_user_123/memory.max
cat /sys/fs/cgroup/sandbox_user_123/cpu.max
```

**Monitor Resource Usage:**
```bash
cat /sys/fs/cgroup/sandbox_user_123/memory.current
cat /sys/fs/cgroup/sandbox_user_123/cpu.stat
```

### Debug FFmpeg

**Test X11 Capture:**
```bash
Xvfb :99 -screen 0 1920x1080x24 &
DISPLAY=:99 openbox &
DISPLAY=:99 xeyes &

ffmpeg -f x11grab -video_size 1920x1080 -i :99 -t 5 test.mp4
```

**Test WebRTC Pipeline:**
```bash
ffmpeg -f x11grab -video_size 1920x1080 -i :99 \
    -c:v libx264 -preset ultrafast -tune zerolatency \
    -f rtp rtp://127.0.0.1:5004
```

### Debug WebRTC

**Enable WebRTC Logging:**
```bash
RUST_LOG=webrtc=trace cargo run
```

**Browser Console:**
```javascript
// In browser console
pc.getStats().then(stats => console.log(stats));
```

**Check ICE Candidates:**
```javascript
pc.onicecandidate = (event) => {
    if (event.candidate) {
        console.log('ICE candidate:', event.candidate);
    }
};
```

## Performance Profiling

### CPU Profiling

```bash
# Install perf
sudo apt install linux-tools-generic

# Run with profiling
cargo build --release
sudo perf record --call-graph dwarf ./target/release/sandbox-server
sudo perf report
```

### Memory Profiling

```bash
# Install valgrind
sudo apt install valgrind

# Run with memory check
cargo build
valgrind --leak-check=full ./target/debug/sandbox-server
```

### Benchmarking

```bash
# Run benchmarks
cargo bench

# Specific benchmark
cargo bench --bench sandbox_bench
```

## Common Issues

### "Operation not permitted" when creating namespace

**Cause:** User namespaces disabled

**Solution:**
```bash
sudo sysctl -w kernel.unprivileged_userns_clone=1
```

### "Landlock not supported"

**Cause:** Kernel too old or LSM not enabled

**Solution:**
- Upgrade to kernel 5.13+
- Check: `cat /sys/kernel/security/lsm` (should include "landlock")
- Enable in kernel config: `CONFIG_SECURITY_LANDLOCK=y`

### FFmpeg not found

**Cause:** FFmpeg not installed or not in PATH

**Solution:**
```bash
sudo apt install ffmpeg
which ffmpeg  # Verify installation
```

### WebRTC connection fails

**Cause:** Firewall blocking UDP, STUN server unreachable

**Solution:**
- Check firewall: `sudo ufw status`
- Allow UDP: `sudo ufw allow 3478/udp`
- Test STUN: `stunclient stun.l.google.com 19302`

### Database connection refused

**Cause:** PostgreSQL not running or wrong credentials

**Solution:**
```bash
sudo systemctl status postgresql
sudo systemctl start postgresql

# Test connection
psql -h localhost -U sandbox_user -d sandbox_server
```

## Contributing

### Code Style

- Follow Rust style guide (enforced by `cargo fmt`)
- Write documentation comments for public APIs
- Add tests for new features
- Update relevant documentation

### Pull Request Process

1. Create feature branch: `git checkout -b feature/my-feature`
2. Make changes and commit: `git commit -am "Add feature"`
3. Run tests: `cargo test`
4. Run clippy: `cargo clippy`
5. Format code: `cargo fmt`
6. Push: `git push origin feature/my-feature`
7. Create pull request on GitHub

### Commit Messages

Follow conventional commits:
```
feat: Add Landlock filesystem policies
fix: Resolve namespace cleanup race condition
docs: Update security documentation
test: Add integration tests for WebRTC
chore: Update dependencies
```

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Async Rust](https://rust-lang.github.io/async-book/)
- [Linux Namespaces](https://man7.org/linux/man-pages/man7/namespaces.7.html)
- [Landlock Documentation](https://docs.kernel.org/userspace-api/landlock.html)
- [cgroups v2](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html)
- [WebRTC Specification](https://www.w3.org/TR/webrtc/)
- [Axum Documentation](https://docs.rs/axum/latest/axum/)
