# Project Structure

This project follows a **hexagonal architecture** (ports & adapters) organized as a **monorepo** with clear separation between backend and frontend.

```
sandbox/ (monorepo root)
├── Cargo.toml                 # Workspace definition
│
├── backend/                   # Rust backend (secure streaming server)
│   ├── Cargo.toml             # Backend crate dependencies
│   ├── src/
│   │   ├── domain/            # Business logic (pure Rust, no dependencies)
│   │   ├── application/       # Use cases (commands/queries by persona)
│   │   ├── infrastructure/    # Technical implementations
│   │   ├── adapters/          # External integrations
│   │   └── main.rs            # Application entry point
│   ├── docs/
│   │   └── adapters/          # Adapter-specific documentation
│   └── README.md
│
├── frontend/                   # Frontend applications
│   ├── web/                   # Web UI (React/Vue/Svelte)
│   │   ├── package.json
│   │   └── src/
│   └── README.md
│
├── docs/                       # Root-level architecture documentation
│   ├── ARCHITECTURE.md
│   ├── PERSONAS.md
│   ├── TESTING.md
│   └── ...
│
└── README.md                   # Project overview
```

---

## Backend Structure

The backend is organized by hexagonal architecture principles:

### Domain Layer (`backend/src/domain/`)
Pure business logic with zero external dependencies:
- **Aggregates**: User, File, Permission, Session, etc.
- **Value Objects**: Email, UserId, FileId, PermissionSet
- **Domain Events**: UserRegistered, FileUploaded, SessionStarted
- **Repository Interfaces**: Ports for persistence

### Application Layer (`backend/src/application/`)
Organized by **persona** (role-based access control):
- **super_admin/**: Commands/queries for SuperAdmin persona
- **owner/**: Commands/queries for Owner persona  
- **client/**: Commands/queries for Client persona
- **services/**: Application services (orchestration)
- **ports/**: Interfaces for infrastructure

### Infrastructure Layer (`backend/src/infrastructure/`)
Technical implementations:
- **persistence/**: PostgreSQL repositories
- **security/**: Landlock LSM integration
- **webrtc/**: WebRTC video streaming
- **email/**: Email service (SMTP)
- **filesystem/**: File storage management

### Adapters Layer (`backend/src/adapters/`)
External system integrations:
- **http/**: REST API (Actix-Web/Axum)
- **webauthn/**: WebAuthn/FIDO2 authentication
- **haproxy/**: Reverse proxy integration
- **monitoring/**: Prometheus metrics
- **audit/**: Hybrid logging (PostgreSQL + filesystem)

See [backend/docs/adapters/](backend/docs/adapters/) for detailed adapter documentation.

---

## Frontend Structure

### Web UI (`frontend/web/`)
Primary user interface:
- **Technology**: TBD (React/Vue/Svelte + TypeScript)
- **Features**:
  - Passwordless authentication (WebAuthn)
  - File management (upload, download, organize)
  - Permission management
  - Session monitoring

### Admin Dashboard (`frontend/admin/`)
SuperAdmin interface (optional):
- User management
- System monitoring
- Audit log viewer
- System configuration

---

## Documentation Structure

### Architecture Docs (`docs/`)
High-level design and decisions:
- `ARCHITECTURE.md` - System architecture overview
- `PERSONAS.md` - User roles and authorization model
- `PASSWORDLESS.md` - WebAuthn/FIDO2 authentication
- `TRACEABILITY.md` - Audit logging strategy
- `TESTING.md` - Testing strategy
- `REVERSE_PROXY_DECISION.md` - HAProxy selection rationale

### Adapter Docs (`backend/docs/adapters/`)
Technical integration specifications:
- Each adapter has detailed documentation
- Configuration requirements
- Dependencies and setup
- Integration points
- Testing approach

---

## Development Workflow

1. **Domain-First**: Start with domain models (aggregates, value objects)
2. **Application Layer**: Implement commands/queries with acceptance criteria
3. **Infrastructure**: Build technical implementations
4. **Adapters**: Integrate external systems
5. **Frontend**: Build UI consuming backend API

---

## Technology Stack

### Backend
- **Language**: Rust (stable)
- **Web Framework**: Actix-Web or Axum
- **Database**: PostgreSQL
- **Security**: Landlock LSM (Linux Security Module)
- **Streaming**: WebRTC
- **Authentication**: WebAuthn/FIDO2
- **Reverse Proxy**: HAProxy

### Frontend
- **TBD**: React/Vue/Svelte + TypeScript
- **Build Tool**: Vite
- **UI Components**: TBD
- **State Management**: TBD
- **WebRTC**: Native WebRTC API

---

**Last Updated**: 2026-02-14
