# Docker Compose Deployment Guide

## Overview

This guide covers deploying the Secure Sandbox Server using Docker Compose, the recommended production deployment method.

## ⚠️ Security Notice

**All configurations in this guide follow the SECURITY-FIRST directive.** Defaults are restrictive, ports are minimized, and all services run with least privilege. See [REQUIREMENTS.md](REQUIREMENTS.md) for the complete security directive.

## Architecture

```
┌─────────────────────────────────────────────────┐
│                Docker Host                      │
│                                                 │
│  ┌────────────────────────────────────────┐    │
│  │  HAProxy Reverse Proxy (TLS 1.3 only)  │    │
│  │  Port 443 (HTTPS only)                 │    │
│  │  GDPR Compliant                        │    │
│  └──────────────┬─────────────────────────┘    │
│                 │                               │
│  ┌──────────────▼─────────────────────────┐    │
│  │  Application Container                 │    │
│  │  - Rust Server                         │    │
│  │  - WebRTC Engine                       │    │
│  │  - Sandbox Manager                     │    │
│  │  - No exposed ports (internal only)    │    │
│  └──────────┬──────────────┬──────────────┘    │
│             │              │                    │
│  ┌──────────▼─────────┐ ┌─▼──────────────┐    │
│  │  PostgreSQL        │ │  User Storage   │    │
│  │  (Internal only)   │ │  (Encrypted)    │    │
│  └────────────────────┘ └─────────────────┘    │
│                                                 │
│  Docker Networks:                               │
│  - frontend (HAProxy ↔ App)                    │
│  - backend (App ↔ PostgreSQL)                  │
│  - isolated (No internet access)               │
└─────────────────────────────────────────────────┘
```

## Prerequisites

### System Requirements

**Host System:**
- Linux kernel 5.13+ (for Landlock LSM)
- Docker Engine 24.0+
- Docker Compose V2.20+
- 16GB RAM minimum (32GB recommended)
- 100GB SSD storage
- CPU with 4+ cores

**Verify Kernel:**
```bash
uname -r  # Must be >= 5.13

# Check Landlock support
cat /sys/kernel/security/lsm | grep landlock
```

### Install Docker

**Ubuntu/Debian:**
```bash
# Remove old versions
sudo apt remove docker docker-engine docker.io containerd runc

# Install Docker from official repository
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# Install Docker Compose V2
sudo apt update
sudo apt install docker-compose-plugin

# Add user to docker group (optional, logout required)
sudo usermod -aG docker $USER

# Verify installation
docker --version        # Should be >= 24.0
docker compose version  # Should be >= 2.20
```

**Enable Rootless Docker (Recommended):**
```bash
# Install rootless prerequisites
sudo apt install -y uidmap dbus-user-session

# Setup rootless Docker
dockerd-rootless-setuptool.sh install

# Configure environment
systemctl --user start docker
systemctl --user enable docker

# Test
docker run hello-world
```

## Project Structure

```
secure-sandbox-server/
├── docker-compose.yml          # Main orchestration file
├── docker-compose.prod.yml     # Production overrides
├── .env                        # Environment configuration (DO NOT COMMIT)
├── .env.example                # Template for .env
│
├── docker/                     # Docker configurations
│   ├── app/
│   │   ├── Dockerfile          # Application container
│   │   └── entrypoint.sh       # Startup script
│   ├── nginx/
│   │   ├── Dockerfile          # Nginx container
│   │   ├── nginx.conf          # Nginx configuration
│   │   └── ssl/                # TLS certificates
│   └── postgres/
│       └── init.sql            # Database initialization
│
├── volumes/                    # Docker volume mount points
│   ├── user_data/              # User files (encrypted)
│   ├── postgres_data/          # Database storage
│   ├── audit_logs/             # Immutable audit trail
│   └── ssl_certs/              # TLS certificates
│
└── scripts/
    ├── generate-secrets.sh     # Generate secure secrets
    ├── backup.sh               # Backup volumes
    └── restore.sh              # Restore from backup
```

## Configuration

### Generate Secrets

**CRITICAL: Never use default or weak secrets.**

```bash
# Run secret generation script
./scripts/generate-secrets.sh

# This creates:
# - JWT signing secret (256-bit)
# - PostgreSQL password (32 characters)
# - Encryption keys
# - Self-signed TLS certificate (if not using Let's Encrypt)
```

### Environment Configuration

**Create `.env` file:**
```bash
cp .env.example .env
chmod 600 .env  # Restrict permissions
```

**Edit `.env`:**
```bash
# CRITICAL: Never commit this file to version control

# == Application ==
APP_ENV=production
APP_HOST=0.0.0.0                    # Bind inside container only
APP_PORT=8080                       # Internal port, not exposed to host

# == Database ==
POSTGRES_USER=sandbox_user
POSTGRES_PASSWORD=<GENERATED_SECRET>  # From generate-secrets.sh
POSTGRES_DB=sandbox_server
POSTGRES_HOST=postgres               # Docker service name
POSTGRES_PORT=5432                   # Internal port

DATABASE_URL=postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@${POSTGRES_HOST}:${POSTGRES_PORT}/${POSTGRES_DB}

# == JWT ==
JWT_SECRET=<GENERATED_SECRET>        # From generate-secrets.sh (256-bit)
JWT_ACCESS_EXPIRY=900                # 15 minutes
JWT_REFRESH_EXPIRY=604800            # 7 days

# == Sandbox Limits ==
SANDBOX_MEMORY_LIMIT_MB=512
SANDBOX_CPU_PERCENT=50
SANDBOX_PID_LIMIT=100
SANDBOX_SESSION_TIMEOUT_SECS=1800    # 30 minutes

# == File Storage ==
FILE_STORAGE_PATH=/data/users        # Inside container
FILE_ENCRYPTION_KEY=<GENERATED_KEY>  # From generate-secrets.sh
FILE_ENCRYPTION_ENABLED=true         # MUST be true in production

# == Video Encoding ==
VIDEO_FRAMERATE=30
VIDEO_BITRATE_KBPS=2500
VIDEO_CODEC=libx264
VIDEO_PRESET=fast
VIDEO_TUNE=zerolatency

# == TLS ==
TLS_ENABLED=true                     # MUST be true in production
TLS_CERT_PATH=/etc/ssl/certs/server.crt
TLS_KEY_PATH=/etc/ssl/private/server.key

# == Domain ==
DOMAIN=sandbox.example.com           # Your domain

# == Logging ==
RUST_LOG=info,sandbox_server=debug
LOG_FORMAT=json                      # JSON for log aggregation

# == STUN Server ==
STUN_SERVER=stun:stun.l.google.com:19302

# == Security ==
RATE_LIMIT_LOGIN=5                   # Requests per 5 minutes
RATE_LIMIT_API=100                   # Requests per minute
SESSION_TIMEOUT_INACTIVE=1800        # 30 minutes
PASSWORD_MIN_LENGTH=16               # Minimum password length
```

## Docker Compose Files

### Main Configuration (`docker-compose.yml`)

```yaml
version: '3.8'

# ========================================
# SECURITY-FIRST CONFIGURATION
# All defaults are restrictive
# All ports are internal only unless required
# All containers run as non-root
# ========================================

services:
  # Application Server
  app:
    build:
      context: .
      dockerfile: docker/app/Dockerfile
      args:
        RUST_VERSION: "1.75"
    image: sandbox-server:latest
    container_name: sandbox-app
    
    # Security: Run as non-root user
    user: "1000:1000"
    
    # Security: Read-only root filesystem
    read_only: true
    
    # Security: Drop all capabilities, add back only required ones
    cap_drop:
      - ALL
    cap_add:
      - SYS_ADMIN      # Required for namespaces
      - SETUID         # Required for user namespace mapping
      - SETGID         # Required for user namespace mapping
      - DAC_OVERRIDE   # Required for file access in sandboxes
    
    # Security: Prevent privilege escalation
    security_opt:
      - no-new-privileges:true
      - apparmor=docker-default
    
    # Resource limits (prevent DoS)
    deploy:
      resources:
        limits:
          cpus: '4.0'
          memory: 8G
        reservations:
          cpus: '2.0'
          memory: 4G
    
    # Environment from .env file
    env_file:
      - .env
    
    # Volumes
    volumes:
      # User data storage (read-write, encrypted)
      - user_data:/data/users:rw
      
      # Audit logs (append-only)
      - audit_logs:/var/log/audit:rw
      
      # Temporary files (writable, tmpfs)
      - /tmp
      
      # Cgroups access (required for resource limits)
      - /sys/fs/cgroup:/sys/fs/cgroup:ro
    
    # Tmpfs mounts for writable paths (read-only root FS)
    tmpfs:
      - /tmp:mode=1777,size=1G
      - /var/run:mode=755,size=100M
    
    # Networks
    networks:
      - frontend  # HAProxy communication
      - backend   # PostgreSQL communication
    
    # Dependencies
    depends_on:
      postgres:
        condition: service_healthy
    
    # Health check
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    
    # Restart policy
    restart: unless-stopped
    
    # No exposed ports (accessed via Nginx only)
    # expose:
    #   - "8080"  # Internal only

  # PostgreSQL Database
  postgres:
    image: postgres:16-alpine
    container_name: sandbox-postgres
    
    # Security: Run as postgres user (UID 70)
    user: postgres
    
    # Security: Read-only root filesystem
    read_only: true
    
    # Security: Drop all capabilities
    cap_drop:
      - ALL
    
    # Security: Prevent privilege escalation
    security_opt:
      - no-new-privileges:true
    
    # Resource limits
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 4G
        reservations:
          cpus: '1.0'
          memory: 2G
    
    # Environment
    environment:
      POSTGRES_USER: ${POSTGRES_USER}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_DB: ${POSTGRES_DB}
      PGDATA: /var/lib/postgresql/data/pgdata
    
    # Volumes
    volumes:
      - postgres_data:/var/lib/postgresql/data:rw
      - ./docker/postgres/init.sql:/docker-entrypoint-initdb.d/init.sql:ro
    
    # Tmpfs for PostgreSQL temp files
    tmpfs:
      - /var/run/postgresql:mode=770,size=100M
      - /tmp:mode=1777,size=500M
    
    # Networks (backend only, no internet)
    networks:
      - backend
    
    # Health check
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${POSTGRES_USER}"]
      interval: 10s
      timeout: 5s
      retries: 5
    
    # Restart policy
    restart: unless-stopped
    
    # No exposed ports (internal only)

  # Nginx Reverse Proxy
  nginx:
    build:
      context: ./docker/nginx
      dockerfile: Dockerfile
    image: sandbox-nginx:latest
    container_name: sandbox-nginx
    
    # Security: Run as nginx user (UID 101)
    user: "101:101"
    
    # Security: Read-only root filesystem
    read_only: true
    
    # Security: Drop all capabilities
    cap_drop:
      - ALL
    cap_add:
      - NET_BIND_SERVICE  # Bind to port 443
    
    # Security: Prevent privilege escalation
    security_opt:
      - no-new-privileges:true
    
    # Resource limits
    deploy:
      resources:
        limits:
          cpus: '1.0'
          memory: 512M
    
    # Volumes
    volumes:
      - ./docker/nginx/nginx.conf:/etc/nginx/nginx.conf:ro
      - ssl_certs:/etc/ssl:ro
      - ./client:/usr/share/nginx/html:ro
    
    # Tmpfs for writable paths
    tmpfs:
      - /var/cache/nginx:mode=755,size=100M
      - /var/run:mode=755,size=10M
    
    # Ports (HTTPS only, no HTTP)
    ports:
      - "443:443"     # HTTPS
      # NO port 80 - TLS is mandatory
    
    # Networks
    networks:
      - frontend
    
    # Dependencies
    depends_on:
      - app
    
    # Health check
    healthcheck:
      test: ["CMD", "curl", "-fk", "https://localhost/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    
    # Restart policy
    restart: unless-stopped

# ========================================
# Networks
# ========================================
networks:
  # Frontend: Nginx <-> App
  frontend:
    driver: bridge
    internal: false  # Needs internet for STUN/TURN
    ipam:
      config:
        - subnet: 172.20.0.0/24
  
  # Backend: App <-> PostgreSQL
  backend:
    driver: bridge
    internal: true   # No internet access
    ipam:
      config:
        - subnet: 172.21.0.0/24

# ========================================
# Volumes
# ========================================
volumes:
  # User data (encrypted at rest)
  user_data:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: ./volumes/user_data
  
  # PostgreSQL data
  postgres_data:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: ./volumes/postgres_data
  
  # Audit logs (immutable)
  audit_logs:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: ./volumes/audit_logs
  
  # SSL certificates
  ssl_certs:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: ./volumes/ssl_certs
```

### Production Overrides (`docker-compose.prod.yml`)

```yaml
version: '3.8'

# Production-specific overrides
# Usage: docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d

services:
  app:
    # Production image from registry
    image: registry.example.com/sandbox-server:${VERSION:-latest}
    
    # Stricter resource limits
    deploy:
      resources:
        limits:
          cpus: '8.0'
          memory: 16G
    
    # Production logging
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
  
  postgres:
    # Production-grade PostgreSQL configuration
    command:
      - postgres
      - -c
      - max_connections=200
      - -c
      - shared_buffers=4GB
      - -c
      - effective_cache_size=12GB
      - -c
      - maintenance_work_mem=1GB
      - -c
      - checkpoint_completion_target=0.9
      - -c
      - wal_buffers=16MB
      - -c
      - default_statistics_target=100
      - -c
      - random_page_cost=1.1
      - -c
      - effective_io_concurrency=200
      - -c
      - work_mem=20MB
      - -c
      - min_wal_size=1GB
      - -c
      - max_wal_size=4GB
      - -c
      - ssl=on
      - -c
      - ssl_cert_file=/etc/ssl/certs/postgres.crt
      - -c
      - ssl_key_file=/etc/ssl/private/postgres.key
    
    # Production logging
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
  
  nginx:
    # Production logging
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

## Deployment

### Initial Setup

```bash
# 1. Clone repository
git clone https://github.com/yourorg/secure-sandbox-server.git
cd secure-sandbox-server

# 2. Create volume directories
mkdir -p volumes/{user_data,postgres_data,audit_logs,ssl_certs}
chmod 700 volumes/user_data
chmod 700 volumes/postgres_data
chmod 700 volumes/audit_logs
chmod 700 volumes/ssl_certs

# 3. Generate secrets
./scripts/generate-secrets.sh

# 4. Configure environment
cp .env.example .env
chmod 600 .env
vim .env  # Edit with your settings

# 5. Generate or obtain TLS certificates
# Option A: Self-signed (development)
./scripts/generate-tls-cert.sh

# Option B: Let's Encrypt (production)
# Place certificates in volumes/ssl_certs/

# 6. Build images
docker compose build

# 7. Start services
docker compose up -d

# 8. Check status
docker compose ps
docker compose logs -f

# 9. Create admin user
docker compose exec app ./create-user --username admin --role admin
```

### TLS Certificate Setup

**Self-Signed (Development/Testing):**
```bash
openssl req -x509 -nodes -days 365 -newkey rsa:4096 \
  -keyout volumes/ssl_certs/server.key \
  -out volumes/ssl_certs/server.crt \
  -subj "/CN=localhost"

chmod 600 volumes/ssl_certs/server.key
chmod 644 volumes/ssl_certs/server.crt
```

**Let's Encrypt (Production):**
```bash
# Install certbot
sudo apt install certbot

# Obtain certificate
sudo certbot certonly --standalone -d sandbox.example.com

# Copy to volume
sudo cp /etc/letsencrypt/live/sandbox.example.com/fullchain.pem volumes/ssl_certs/server.crt
sudo cp /etc/letsencrypt/live/sandbox.example.com/privkey.pem volumes/ssl_certs/server.key
sudo chown $USER:$USER volumes/ssl_certs/*

# Setup auto-renewal
sudo systemctl enable certbot.timer
```

## Operations

### Start Services

```bash
# Start all services
docker compose up -d

# Start specific service
docker compose up -d app

# Start with production overrides
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

### Stop Services

```bash
# Stop all services
docker compose stop

# Stop specific service
docker compose stop app

# Stop and remove containers
docker compose down
```

### View Logs

```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f app

# Last 100 lines
docker compose logs --tail=100 app
```

### Execute Commands

```bash
# Shell in app container
docker compose exec app /bin/sh

# Run database migrations
docker compose exec app sqlx migrate run

# Create user
docker compose exec app ./create-user --username testuser --role user

# PostgreSQL shell
docker compose exec postgres psql -U sandbox_user -d sandbox_server
```

### Restart Services

```bash
# Restart all
docker compose restart

# Restart specific service
docker compose restart app

# Rebuild and restart
docker compose up -d --build app
```

## Monitoring & Maintenance

### Health Checks

```bash
# Check service health
docker compose ps

# Individual health check
curl -k https://localhost/health
```

### Resource Usage

```bash
# Container stats
docker stats

# Disk usage
docker system df

# Volume usage
du -sh volumes/*
```

### Backups

**Automated Backup Script (`scripts/backup.sh`):**
```bash
#!/bin/bash
set -e

BACKUP_DIR="/backups/sandbox-server/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$BACKUP_DIR"

# Stop services (optional, for consistency)
# docker compose stop app

# Backup PostgreSQL
docker compose exec -T postgres pg_dump -U sandbox_user sandbox_server | \
  gzip > "$BACKUP_DIR/postgres.sql.gz"

# Backup user data
tar czf "$BACKUP_DIR/user_data.tar.gz" -C volumes user_data

# Backup audit logs
tar czf "$BACKUP_DIR/audit_logs.tar.gz" -C volumes audit_logs

# Backup configuration (exclude secrets)
tar czf "$BACKUP_DIR/config.tar.gz" \
  docker-compose.yml \
  docker-compose.prod.yml \
  docker/

# Encrypt backup
gpg --symmetric --cipher-algo AES256 "$BACKUP_DIR"/*.gz

# Remove unencrypted files
rm "$BACKUP_DIR"/*.gz

# Restart services
# docker compose start app

echo "Backup completed: $BACKUP_DIR"
```

### Updates

```bash
# Pull latest images
docker compose pull

# Rebuild custom images
docker compose build --no-cache

# Apply updates with zero downtime (if load balanced)
docker compose up -d --no-deps --build app

# Or full restart
docker compose down
docker compose up -d
```

## Security Hardening

### Docker Daemon Security

```bash
# Enable user namespace remapping
sudo vim /etc/docker/daemon.json
```

```json
{
  "userns-remap": "default",
  "live-restore": true,
  "userland-proxy": false,
  "no-new-privileges": true,
  "seccomp-profile": "/etc/docker/seccomp.json"
}
```

```bash
sudo systemctl restart docker
```

### Firewall Configuration

```bash
# Allow only HTTPS
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp   # SSH
sudo ufw allow 443/tcp  # HTTPS
sudo ufw enable
```

### Audit Logging

```bash
# Enable Docker audit logging
sudo auditctl -w /usr/bin/docker -k docker
sudo auditctl -w /var/lib/docker -k docker
sudo auditctl -w /etc/docker -k docker

# View audit logs
sudo ausearch -k docker
```

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker compose logs app

# Check permissions
ls -la volumes/

# Check configuration
docker compose config

# Validate environment
docker compose exec app env
```

### Database Connection Issues

```bash
# Check PostgreSQL is running
docker compose ps postgres

# Check network connectivity
docker compose exec app ping postgres

# Test database connection
docker compose exec postgres psql -U sandbox_user -d sandbox_server -c "SELECT 1;"
```

### Performance Issues

```bash
# Check resource usage
docker stats

# Check cgroup limits
docker compose exec app cat /sys/fs/cgroup/memory/memory.max
docker compose exec app cat /sys/fs/cgroup/cpu/cpu.max

# Adjust limits in docker-compose.yml
```

## Production Checklist

Before deploying to production:

- [ ] All secrets generated with cryptographically secure random
- [ ] `.env` file permissions set to 600
- [ ] TLS certificates from trusted CA (not self-signed)
- [ ] Database password is strong (32+ characters)
- [ ] JWT secret is 256-bit random
- [ ] File encryption enabled
- [ ] All volume directories have correct permissions
- [ ] Firewall rules configured (only 443 open)
- [ ] Docker daemon security hardened
- [ ] Backup script tested and scheduled
- [ ] Monitoring and alerting configured
- [ ] Audit logging enabled
- [ ] Security scan passed (docker scan)
- [ ] Load testing completed
- [ ] Disaster recovery plan documented

---

**Security Contact:** security@example.com  
**Documentation Version:** 1.0  
**Last Updated:** 2026-02-13
