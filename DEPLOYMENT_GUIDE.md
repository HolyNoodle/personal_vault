# Monorepo Structure
This is a monorepo containing both backend (Rust) and frontend (TypeScript) applications.

## Quick Start

### Development
```bash
# Start all services (Postgres, Redis, Backend, Frontend, Mailhog)
chmod +x scripts/*.sh
./scripts/dev-start.sh

# Access services:
# - Frontend: http://localhost:5173
# - Backend API: http://localhost:8080
# - Mailhog: http://localhost:8025

# Stop
./scripts/dev-stop.sh
```

### Production
```bash
# Generate secrets
./scripts/generate-secrets.sh

# Configure .env.production with your domain and secrets

# Start production stack
./scripts/prod-start.sh

# Access:
# - Application: https://your-domain.com
# - HAProxy Stats: http://localhost:8404/stats
```

## Project Structure
```
/
├── backend/                  # Rust backend
│   ├── src/
│   ├── Cargo.toml
│   ├── Dockerfile            # Production build
│   └── Dockerfile.dev        # Dev with hot-reload
├── frontend/web/             # React/Vue/Svelte frontend
│   ├── src/
│   ├── package.json
│   ├── Dockerfile            # Production build
│   └── Dockerfile.dev        # Dev with hot-reload
├── haproxy/                  # Reverse proxy config
├── scripts/                  # Utility scripts
├── docker-compose.dev.yml    # Development stack
├── docker-compose.prod.yml   # Production stack
├── .env.development          # Dev environment vars
└── .env.production           # Prod environment vars
```

## Environment Variables

Copy and configure:
- Development: `.env.development` (already configured)
- Production: `.env.production` (update passwords, domain, SMTP)

## Database Migrations

```bash
# Development
docker exec -it sandbox-backend-dev cargo sqlx migrate run

# Production
docker exec -it sandbox-backend-prod /app/bin/sandbox-server migrate
```

## Monitoring

- **Logs**: `docker-compose logs -f <service>`
- **Health**: `curl http://localhost:8080/health`
- **HAProxy Stats**: http://localhost:8404/stats (production)

## Security Checklist for Production

- [ ] Change all passwords in `.env.production`
- [ ] Generate secrets: `./scripts/generate-secrets.sh`
- [ ] Update `WEBAUTHN_RP_ID` and `WEBAUTHN_ORIGIN` to your domain
- [ ] Configure SMTP credentials
- [ ] Add TLS certificates to `haproxy/certs/`
- [ ] Update `CORS_ALLOWED_ORIGINS`
- [ ] Set firewall rules (only 80, 443 exposed)
