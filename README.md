# Secure Sandbox Monorepo

A secure WebRTC-based file sandbox server with kernel-level access control using Landlock LSM.

---

## Quick Start

### First Time Setup

```bash
# 1. Install dependencies
npm install

# 2. Copy environment template
cp .env.default .env.development

# 3. (Optional) Edit .env.development with your settings

# 4. Start development environment
npm run dev
```

**Services will be available at:**
- Frontend: http://localhost:5173
- Backend API: http://localhost:8080
- Mailhog (email testing): http://localhost:8025
- PostgreSQL: localhost:5432
- Redis: localhost:6379

---

## NPM Commands (Single Entry Point)

### Development

```bash
npm run dev              # Start all services (PostgreSQL, Redis, Backend, Frontend)
npm run dev:build        # Rebuild containers and start
npm run dev:detach       # Start in background
npm run dev:stop         # Stop all services
npm run dev:clean        # Stop and remove volumes (⚠️ deletes database)
npm run dev:logs         # View all logs
npm run dev:logs:backend # View backend logs only
npm run dev:logs:frontend # View frontend logs only
```

### Production

```bash
# First time setup
npm run secrets:generate  # Generate secure passwords
cp .env.default .env.production
# Edit .env.production with your domain and secrets

# Deploy
npm run prod              # Start production stack
npm run prod:build        # Rebuild and start
npm run prod:stop         # Stop production
npm run prod:logs         # View logs
```

### Database

```bash
npm run db:migrate        # Run database migrations (dev)
npm run db:migrate:prod   # Run migrations (production)
npm run db:reset          # Reset database (⚠️ deletes all data)
```

### Build & Test

```bash
npm run build            # Build backend + frontend
npm run test             # Test backend + frontend
npm run lint             # Lint both projects
npm run format           # Format Rust code

# Individual commands
npm run backend:build
npm run backend:test
npm run frontend:build
npm run frontend:test
```

### Utilities

```bash
npm run health           # Check backend health
npm run clean            # Remove temporary files
npm run setup            # Install all dependencies
```

---

## Project Structure

```
/
├── backend/                  # Rust backend (API + static file serving)
│   ├── src/
│   ├── Cargo.toml
│   └── Dockerfile
├── frontend/web/             # React/Vue/Svelte frontend
│   ├── src/
│   └── package.json
├── scripts/                  # Node.js utility scripts
│   ├── setup-env.js
│   ├── generate-secrets.js
│   └── cleanup.js
├── haproxy/                  # Reverse proxy config
├── docker-compose.dev.yml    # Development stack
├── docker-compose.prod.yml   # Production stack
├── .env.default              # Template (committed)
├── .env.development          # Local dev (ignored)
├── .env.production           # Production (ignored)
└── package.json              # NPM workspace (main entry point)
```

---

## Development Workflow

1. **Make changes** to backend (`backend/src/`) or frontend (`frontend/web/src/`)
2. **Changes auto-reload** (cargo-watch for Rust, Vite for frontend)
3. **View logs**: `npm run dev:logs`
4. **Test**: `npm run test`
5. **Lint**: `npm run lint`

---

## Production Deployment

1. **Generate secrets**: `npm run secrets:generate`
2. **Configure**: Edit `.env.production`
3. **Add TLS certs**: Place in `haproxy/certs/sandbox.pem`
4. **Deploy**: `npm run prod`
5. **Monitor**: `npm run prod:logs`

Access:
- Application: https://your-domain.com
- HAProxy Stats: http://localhost:8404/stats

---

## Technical Stack

- **Backend**: Rust (Axum) - Serves API + static frontend files
- **Frontend**: TypeScript (React/Vue/Svelte) - Built and bundled into backend
- **Database**: PostgreSQL 16
- **Cache**: Redis 7
- **Proxy**: HAProxy (TLS termination, load balancing)
- **Security**: Landlock LSM (kernel-level file access control)
- **Authentication**: WebAuthn/FIDO2 (passwordless)
- **Video Streaming**: WebRTC
- **Containerization**: Docker + Docker Compose

---

## Environment Variables

See `.env.default` for all available options.

Key variables:
- `DATABASE_URL` - PostgreSQL connection
- `REDIS_URL` - Redis connection
- `WEBAUTHN_RP_ID` - Your domain
- `WEBAUTHN_ORIGIN` - Frontend URL
- `STORAGE_ROOT` - File storage path
- `SMTP_*` - Email configuration

---

## Troubleshooting

**Services won't start:**
```bash
npm run dev:clean  # Remove all volumes
npm run dev:build  # Rebuild containers
```

**Database connection error:**
```bash
# Check PostgreSQL is running
docker ps | grep postgres

# View logs
npm run dev:logs:backend
```

**Port conflicts:**
Edit `docker-compose.dev.yml` to change port mappings.

---

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Testing Strategy](docs/TESTING.md)
- [Deployment Guide](DEPLOYMENT_GUIDE.md)
- [Environment Setup](ENV_SETUP.md)
- [Security Requirements](docs/REQUIREMENTS.md)
- [Application Behavior](docs/APPLICATION_BEHAVIOR.md)
- [Personas & Roles](docs/PERSONAS.md)

---

**Tech Stack**: Rust, PostgreSQL, Redis, HAProxy, WebRTC, Landlock LSM, WebAuthn  
**License**: MIT

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for detailed workflow.

## 📊 Performance

| Metric | Value | Notes |
|--------|-------|-------|
| Sandbox startup | <10ms | Namespace creation |
| Session latency | ~100ms | Glass-to-glass (local network) |
| Memory per session | 100-500MB | Configurable via cgroups |
| CPU per session | 10-50% | Video encoding load |
| Concurrent sessions | 20-50 | Single server, hardware encoding recommended |

## 🔧 Configuration

Key settings in `.env`:

```bash
# Sandbox resource limits
SANDBOX_MEMORY_LIMIT_MB=512       # Max RAM per session
SANDBOX_CPU_PERCENT=50            # Max CPU % per session
SANDBOX_SESSION_TIMEOUT_SECS=1800 # 30 min inactivity timeout

# Video encoding
VIDEO_FRAMERATE=30                # FPS
VIDEO_BITRATE_KBPS=2000           # Network bandwidth
VIDEO_CODEC=libx264               # Or h264_nvenc for GPU
VIDEO_PRESET=ultrafast            # Encoding speed
```

## 📋 Roadmap

- [x] Core sandbox engine (namespaces, Landlock, cgroups)
- [x] WebRTC video streaming
- [x] Input forwarding (mouse/keyboard)
- [x] Authentication (JWT + argon2)
- [x] PostgreSQL user/permissions
- [ ] Web UI client
- [ ] File upload to sandbox
- [ ] Multi-user collaborative sessions
- [ ] Recording/playback for audit
- [ ] GPU passthrough for 3D apps
- [ ] Wayland compositor support
- [ ] Kubernetes deployment
- [ ] Clipboard sync (controlled)
- [ ] Mobile clients (iOS/Android)

## 🤝 Contributing

Contributions welcome! Please:

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make changes and add tests
4. Run `cargo test && cargo clippy && cargo fmt`
5. Submit a pull request

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

[MIT License](LICENSE) - See LICENSE file for details

## 🙏 Acknowledgments

Built with:
- [webrtc-rs](https://github.com/webrtc-rs/webrtc) - Pure Rust WebRTC
- [Axum](https://github.com/tokio-rs/axum) - Ergonomic web framework
- [nix](https://github.com/nix-rust/nix) - Rust-friendly Unix API
- [landlock](https://github.com/landlock-lsm/rust-landlock) - Kernel LSM bindings
- [FFmpeg](https://ffmpeg.org/) - Video encoding

Inspired by:
- [Kasm Workspaces](https://www.kasmweb.com/) - Browser-based containerized apps
- [Neko](https://github.com/m1k1o/neko) - Self-hosted virtual browser
- [Apache Guacamole](https://guacamole.apache.org/) - Clientless remote desktop

## 📞 Contact

- **Issues**: GitHub Issues
- **Security**: security@example.com
- **Discussions**: GitHub Discussions

## ⚠️ Disclaimer

This software is provided as-is. While designed with security in mind, no system is 100% secure. Always:
- Keep kernel and dependencies updated
- Review audit logs regularly
- Test in non-production environment first
- Follow deployment best practices (see [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md))
---

**Status**: Active Development | **Stability**: Alpha | **Production Ready**: No (pending security audit)
