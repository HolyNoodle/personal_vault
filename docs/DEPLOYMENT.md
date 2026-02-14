# Deployment Guide

## Overview

This guide covers deploying the Secure Sandbox Server in production environments, including single-server and multi-node configurations.

## Prerequisites

### System Requirements

**Hardware (minimum per server):**
- CPU: 4+ cores (8+ recommended for video encoding)
- RAM: 16GB (32GB+ recommended)
- Storage: 100GB+ SSD
- Network: 1Gbps+ (10Gbps for >50 concurrent users)
- GPU: Optional (NVIDIA for NVENC encoding acceleration)

**Operating System:**
- Linux kernel 5.13+ (required for Landlock)
- Ubuntu 22.04 LTS or Debian 12 (recommended)
- RHEL 9+ or Fedora 36+ (supported)

**Software:**
- PostgreSQL 14+
- FFmpeg 5.0+
- HAProxy 2.8+ (reverse proxy - GDPR compliant)
- systemd (process management)

### Security Hardening

**Kernel Configuration:**
```bash
# Enable security features
sudo sysctl -w kernel.unprivileged_userns_clone=1
sudo sysctl -w kernel.kptr_restrict=2
sudo sysctl -w kernel.dmesg_restrict=1
sudo sysctl -w net.ipv4.conf.all.rp_filter=1

# Persist settings
sudo tee -a /etc/sysctl.conf <<EOF
kernel.unprivileged_userns_clone=1
kernel.kptr_restrict=2
kernel.dmesg_restrict=1
net.ipv4.conf.all.rp_filter=1
EOF

sudo sysctl -p
```

**Firewall Configuration:**
```bash
# Ubuntu/Debian (ufw)
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp      # SSH
sudo ufw allow 443/tcp     # HTTPS
sudo ufw allow 3478/udp    # STUN
sudo ufw enable

# RHEL/Fedora (firewalld)
sudo firewall-cmd --permanent --add-service=ssh
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --permanent --add-port=3478/udp
sudo firewall-cmd --reload
```

**AppArmor Profile (optional additional security):**
```bash
sudo tee /etc/apparmor.d/sandbox-server <<'EOF'
#include <tunables/global>

/usr/local/bin/sandbox-server {
  #include <abstractions/base>
  #include <abstractions/nameservice>
  
  /usr/local/bin/sandbox-server mr,
  /data/users/** rw,
  /tmp/** rw,
  /sys/fs/cgroup/** rw,
  
  capability sys_admin,
  capability setuid,
  capability setgid,
  
  deny /proc/sys/kernel/** w,
  deny /boot/** r,
  deny /sys/kernel/debug/** r,
}
EOF

sudo apparmor_parser -r /etc/apparmor.d/sandbox-server
```

## Single-Server Deployment

### Installation

**1. Install Dependencies:**
```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install required packages
sudo apt install -y \
    postgresql \
    postgresql-contrib \
    ffmpeg \
    xvfb \
    x11-utils \
    xdotool \
    openbox \
    fonts-liberation \
    fonts-noto \
    haproxy \
    certbot \
    git \
    build-essential \
    pkg-config \
    libssl-dev
```

**2. Install Rust:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**3. Clone and Build:**
```bash
# Create application directory
sudo mkdir -p /opt/sandbox-server
sudo chown $USER:$USER /opt/sandbox-server
cd /opt/sandbox-server

# Clone repository
git clone https://github.com/yourorg/secure-sandbox-server.git .

# Build release binary
cargo build --release

# Install binary
sudo cp target/release/sandbox-server /usr/local/bin/
sudo chmod +x /usr/local/bin/sandbox-server
```

### Database Setup

**1. Configure PostgreSQL:**
```bash
# Edit PostgreSQL config
sudo vim /etc/postgresql/14/main/postgresql.conf
```

Add/modify:
```conf
listen_addresses = 'localhost'
max_connections = 200
shared_buffers = 4GB
effective_cache_size = 12GB
maintenance_work_mem = 1GB
checkpoint_completion_target = 0.9
wal_buffers = 16MB
default_statistics_target = 100
random_page_cost = 1.1
effective_io_concurrency = 200
work_mem = 20MB
min_wal_size = 1GB
max_wal_size = 4GB
```

**2. Create Database:**
```bash
sudo -u postgres psql <<EOF
CREATE DATABASE sandbox_server;
CREATE USER sandbox_user WITH ENCRYPTED PASSWORD 'CHANGE_ME_STRONG_PASSWORD';
GRANT ALL PRIVILEGES ON DATABASE sandbox_server TO sandbox_user;

-- Enable extensions
\c sandbox_server
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
EOF
```

**3. Run Migrations:**
```bash
cd /opt/sandbox-server
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run
```

### Application Configuration

**1. Create Environment File:**
```bash
sudo mkdir -p /etc/sandbox-server
sudo tee /etc/sandbox-server/config.env <<EOF
# Database
DATABASE_URL=postgresql://sandbox_user:CHANGE_ME_STRONG_PASSWORD@localhost/sandbox_server

# Server
HOST=127.0.0.1
PORT=8080

# JWT Secret (generate with: openssl rand -base64 32)
JWT_SECRET=$(openssl rand -base64 32)

# JWT Expiry
JWT_ACCESS_EXPIRY=900
JWT_REFRESH_EXPIRY=604800

# Sandbox Limits
SANDBOX_MEMORY_LIMIT_MB=512
SANDBOX_CPU_PERCENT=50
SANDBOX_PID_LIMIT=100
SANDBOX_SESSION_TIMEOUT_SECS=1800

# File Storage
FILE_STORAGE_PATH=/data/users
FILE_ENCRYPTION_ENABLED=true

# Video Encoding
VIDEO_FRAMERATE=30
VIDEO_BITRATE_KBPS=2500
VIDEO_CODEC=libx264
VIDEO_PRESET=fast
VIDEO_TUNE=zerolatency

# Logging
RUST_LOG=info,sandbox_server=debug
LOG_FORMAT=json

# STUN Server
STUN_SERVER=stun:stun.l.google.com:19302

# Production
RUST_BACKTRACE=1
EOF

sudo chmod 600 /etc/sandbox-server/config.env
```

**2. Create Storage Directories:**
```bash
sudo mkdir -p /data/users
sudo chown sandbox-server:sandbox-server /data/users
sudo chmod 700 /data/users
```

**3. Create Service User:**
```bash
sudo useradd -r -s /bin/false -d /opt/sandbox-server sandbox-server
sudo chown -R sandbox-server:sandbox-server /opt/sandbox-server
```

### systemd Service

**Create Service File:**
```bash
sudo tee /etc/systemd/system/sandbox-server.service <<'EOF'
[Unit]
Description=Secure Sandbox Server
After=network.target postgresql.service
Requires=postgresql.service

[Service]
Type=simple
User=sandbox-server
Group=sandbox-server
WorkingDirectory=/opt/sandbox-server
EnvironmentFile=/etc/sandbox-server/config.env
ExecStart=/usr/local/bin/sandbox-server
Restart=always
RestartSec=10

# Security hardening
NoNewPrivileges=false
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/data/users /sys/fs/cgroup /tmp
CapabilityBoundingSet=CAP_SYS_ADMIN CAP_SETUID CAP_SETGID CAP_DAC_OVERRIDE

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=sandbox-server

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable sandbox-server
sudo systemctl start sandbox-server
```

**Check Status:**
```bash
sudo systemctl status sandbox-server
sudo journalctl -u sandbox-server -f
```

### Reverse Proxy (HAProxy)

**1. Install SSL Certificate:**
```bash
# Using Let's Encrypt (HAProxy must be stopped temporarily)
sudo systemctl stop haproxy
sudo certbot certonly --standalone -d sandbox.example.com
sudo systemctl start haproxy
```

**2. Combine Certificate for HAProxy:**
```bash
# HAProxy requires cert+key in single PEM file
sudo mkdir -p /etc/haproxy/certs
sudo cat /etc/letsencrypt/live/sandbox.example.com/fullchain.pem \
        /etc/letsencrypt/live/sandbox.example.com/privkey.pem \
        | sudo tee /etc/haproxy/certs/sandbox.pem > /dev/null
sudo chmod 600 /etc/haproxy/certs/sandbox.pem
sudo chown haproxy:haproxy /etc/haproxy/certs/sandbox.pem
```

**3. Configure HAProxy:**
```bash
sudo tee /etc/haproxy/haproxy.cfg <<'EOF'
global
    log /dev/log local0
    log /dev/log local1 notice
    chroot /var/lib/haproxy
    stats socket /run/haproxy/admin.sock mode 660 level admin
    stats timeout 30s
    user haproxy
    group haproxy
    daemon

    # TLS 1.3 Only (GDPR Compliance)
    ssl-default-bind-ciphers ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384
    ssl-default-bind-ciphersuites TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256
    ssl-default-bind-options ssl-min-ver TLSv1.3 no-tls-tickets
    tune.ssl.default-dh-param 2048
    maxconn 10000

defaults
    log     global
    mode    http
    option  httplog
    option  dontlognull
    option  http-server-close
    option  forwardfor except 127.0.0.0/8
    timeout connect 5s
    timeout client  50s
    timeout server  50s
    timeout http-request 10s

# HTTP Frontend (Redirect to HTTPS)
frontend http_frontend
    bind *:80
    http-request redirect scheme https code 301

# HTTPS Frontend
frontend https_frontend
    bind *:443 ssl crt /etc/haproxy/certs/sandbox.pem alpn h2,http/1.1
    
    # Security Headers
    http-response set-header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
    http-response set-header X-Frame-Options "DENY"
    http-response set-header X-Content-Type-Options "nosniff"
    http-response set-header X-XSS-Protection "1; mode=block"
    http-response set-header Referrer-Policy "strict-origin-when-cross-origin"
    http-response set-header Content-Security-Policy "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' wss://sandbox.example.com;"
    http-response del-header Server
    http-response del-header X-Powered-By
    
    # Rate Limiting: Login endpoint (5 req/min per IP)
    acl is_login_path path_beg /api/auth/webauthn/login
    stick-table type ip size 100k expire 1m store http_req_rate(1m)
    http-request track-sc0 src if is_login_path
    http-request deny deny_status 429 if is_login_path { sc_http_req_rate(0) gt 5 }
    
    # Rate Limiting: API endpoints (100 req/min per IP)
    acl is_api_path path_beg /api/
    stick-table type ip size 100k expire 1m store http_req_rate(1m)
    http-request track-sc1 src if is_api_path
    http-request deny deny_status 429 if is_api_path { sc_http_req_rate(1) gt 100 }
    
    # DDoS Protection: Connection limits
    acl too_many_connections src_conn_cur gt 10
    http-request deny deny_status 429 if too_many_connections
    
    # WebSocket upgrade detection
    acl is_websocket hdr(Upgrade) -i WebSocket
    acl is_websocket_path path_beg /ws
    
    # Routing
    use_backend websocket_backend if is_websocket is_websocket_path
    default_backend http_backend

# HTTP/API Backend
backend http_backend
    mode http
    balance roundrobin
    
    # Health check
    option httpchk GET /health HTTP/1.1\r\nHost:\ localhost
    http-check expect status 200
    
    # Application server
    server app1 127.0.0.1:8080 check inter 5s rise 2 fall 3 maxconn 1000

# WebSocket Backend
backend websocket_backend
    mode http
    balance leastconn
    
    # WebSocket-specific settings
    option http-server-close
    timeout tunnel 86400s  # 24 hours for long WebRTC sessions
    timeout server 86400s
    
    # Application server
    server app1 127.0.0.1:8080 check inter 5s rise 2 fall 3 maxconn 500

# Stats Dashboard (localhost only)
listen stats
    bind 127.0.0.1:8404
    mode http
    stats enable
    stats uri /stats
    stats refresh 10s
    stats admin if TRUE
EOF

# Test and reload
sudo haproxy -c -f /etc/haproxy/haproxy.cfg
sudo systemctl enable haproxy
sudo systemctl restart haproxy
```

**4. Auto-renewal for Let's Encrypt:**
```bash
# Create renewal hook to update HAProxy certificate
sudo tee /etc/letsencrypt/renewal-hooks/deploy/haproxy.sh <<'EOF'
#!/bin/bash
cat /etc/letsencrypt/live/sandbox.example.com/fullchain.pem \
    /etc/letsencrypt/live/sandbox.example.com/privkey.pem \
    > /etc/haproxy/certs/sandbox.pem
chmod 600 /etc/haproxy/certs/sandbox.pem
chown haproxy:haproxy /etc/haproxy/certs/sandbox.pem
systemctl reload haproxy
EOF

sudo chmod +x /etc/letsencrypt/renewal-hooks/deploy/haproxy.sh

# Test renewal
sudo certbot renew --dry-run
```

### Create Initial Admin User

```bash
cd /opt/sandbox-server
cargo run --release --bin create-user -- \
    --username admin \
    --password "$(openssl rand -base64 32)" \
    --role admin
```

## Multi-Node Deployment

For high availability and scalability, deploy multiple application servers behind a load balancer.

### Architecture

```
                    ┌─────────────┐
                    │ Load Balancer│
                    │  (HAProxy)   │
                    └──────┬───────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
    ┌────▼────┐      ┌────▼────┐      ┌────▼────┐
    │ Server 1│      │ Server 2│      │ Server 3│
    └────┬────┘      └────┬────┘      └────┬────┘
         │                │                 │
         └────────────────┼─────────────────┘
                          │
         ┌────────────────┼─────────────────┐
         │                │                 │
    ┌────▼────┐     ┌────▼────┐      ┌────▼────┐
    │PostgreSQL│     │  NFS    │      │  Logs   │
    │(Primary) │     │ Storage │      │(Elastic)│
    └─────────┘     └─────────┘      └─────────┘
```

### Shared PostgreSQL

Use PostgreSQL with replication:

```bash
# Primary server configuration
# /etc/postgresql/14/main/postgresql.conf
wal_level = replica
max_wal_senders = 10
max_replication_slots = 10
```

### Shared File Storage

**Option 1: NFS**
```bash
# NFS server
sudo apt install nfs-kernel-server
sudo mkdir -p /export/users
sudo tee -a /etc/exports <<EOF
/export/users 192.168.1.0/24(rw,sync,no_root_squash,no_subtree_check)
EOF
sudo exportfs -a

# NFS clients (application servers)
sudo apt install nfs-common
sudo mount -t nfs nfs-server:/export/users /data/users
```

**Option 2: S3-Compatible Storage (MinIO)**
```bash
# Deploy MinIO for object storage
# Update application to use S3 API
```

### Load Balancer (HAProxy)

```bash
# Install HAProxy
sudo apt install haproxy

# Configure
sudo tee /etc/haproxy/haproxy.cfg <<'EOF'
global
    log /dev/log local0
    maxconn 4096
    user haproxy
    group haproxy
    daemon

defaults
    log global
    mode http
    option httplog
    option dontlognull
    timeout connect 5000
    timeout client 50000
    timeout server 50000

frontend https_front
    bind *:443 ssl crt /etc/ssl/certs/sandbox.pem
    mode http
    default_backend sandbox_servers
    
    # WebSocket detection
    acl is_websocket hdr(Upgrade) -i WebSocket
    use_backend websocket_backend if is_websocket

backend sandbox_servers
    balance leastconn
    option httpchk GET /health
    server server1 192.168.1.10:8080 check
    server server2 192.168.1.11:8080 check
    server server3 192.168.1.12:8080 check

backend websocket_backend
    balance source  # Sticky sessions for WebSocket
    option httpchk GET /health
    server server1 192.168.1.10:8080 check
    server server2 192.168.1.11:8080 check
    server server3 192.168.1.12:8080 check
EOF

sudo systemctl reload haproxy
```

## Monitoring

### Prometheus Metrics

```bash
# Install Prometheus Node Exporter
sudo apt install prometheus-node-exporter

# Configure scraping
sudo tee -a /etc/prometheus/prometheus.yml <<EOF
scrape_configs:
  - job_name: 'sandbox-server'
    static_configs:
      - targets: ['localhost:9090']
EOF
```

### Logging (ELK Stack)

```bash
# Install Filebeat
curl -L -O https://artifacts.elastic.co/downloads/beats/filebeat/filebeat-8.0.0-amd64.deb
sudo dpkg -i filebeat-8.0.0-amd64.deb

# Configure to ship logs to Elasticsearch
sudo vim /etc/filebeat/filebeat.yml
```

### Alerting

```bash
# Install Alertmanager
# Configure alerts for:
# - High CPU usage
# - Memory exhaustion
# - Failed authentication attempts
# - Sandbox creation failures
```

## Backup & Recovery

### Database Backups

```bash
# Automated daily backups
sudo tee /etc/cron.daily/backup-postgres <<'EOF'
#!/bin/bash
pg_dump -U sandbox_user sandbox_server | gzip > /backups/postgres-$(date +\%Y\%m\%d).sql.gz
find /backups -name "postgres-*.sql.gz" -mtime +30 -delete
EOF

sudo chmod +x /etc/cron.daily/backup-postgres
```

### File Storage Backups

```bash
# Snapshot-based backups (if using LVM)
sudo lvcreate -L 10G -s -n users_snapshot /dev/vg0/users

# Rsync to backup server
rsync -avz /data/users/ backup-server:/backups/users/
```

## Security Checklist

- [ ] TLS 1.3 certificates installed and auto-renewal configured
- [ ] Firewall rules configured (only 22, 443, 3478 open)
- [ ] SSH key-based authentication only (password auth disabled)
- [ ] Database uses strong password (32+ characters)
- [ ] JWT secret is cryptographically random (256 bits)
- [ ] AppArmor/SELinux profiles enabled
- [ ] Kernel security parameters configured
- [ ] Regular security updates enabled (`unattended-upgrades`)
- [ ] Audit logging to centralized system
- [ ] File storage encrypted at rest
- [ ] Backup encryption enabled
- [ ] Fail2ban configured for SSH/API
- [ ] Intrusion detection system (IDS) deployed
- [ ] Security scanning scheduled (weekly)

## Troubleshooting

### Service Won't Start

```bash
# Check logs
sudo journalctl -u sandbox-server -n 100

# Common issues:
# - Database connection failed: Check DATABASE_URL
# - Permission denied: Ensure user has CAP_SYS_ADMIN
# - Port in use: Check if another process is using 8080
```

### High Memory Usage

```bash
# Check cgroup limits
sudo systemctl status sandbox-server
cat /sys/fs/cgroup/system.slice/sandbox-server.service/memory.max

# Increase if needed
sudo systemctl edit sandbox-server
# Add: MemoryMax=8G
```

### WebRTC Connection Failures

```bash
# Check STUN server reachability
stunclient stun.l.google.com 19302

# Check UDP ports
sudo netstat -ulnp | grep 3478

# Check firewall
sudo ufw status
```

---

**Production Support:** support@example.com  
**Emergency Contact:** +1-555-0100 (24/7)
