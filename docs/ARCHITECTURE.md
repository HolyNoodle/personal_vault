# Architecture

## Overview

> **📖 See [APPLICATION_PLATFORM.md](APPLICATION_PLATFORM.md) for complete dual-execution model documentation with detailed flows and examples.**

The Secure Sandbox Platform is a Rust-based application hosting system with two execution modes:

1. **Sandboxed Mode (Client Users)**: Applications run server-side in extreme isolation, streamed via WebRTC video feed. Users interact with the video stream (input forwarding), ensuring zero data exfiltration.
2. **Browser Mode (Owner Users)**: Applications execute directly in browser with full file system access and download capabilities.

**Key Insight**: Video streaming IS the sandboxing mechanism - clients see only video pixels, preventing data exfiltration while allowing rich interactions.

**First Application**: File Explorer with PDF, image, and video preview capabilities. Client users view files in read-only sandboxed environment; owner users manipulate files with full permissions.

## ⚠️ Security-First Design

This architecture follows the **SECURITY-FIRST DIRECTIVE** (see [REQUIREMENTS.md](REQUIREMENTS.md)). Every architectural decision prioritizes security over convenience, performance, or ease of implementation.

## Architectural Style

The system follows **Hexagonal Architecture** (Ports and Adapters) with **Domain-Driven Design (DDD)** principles and **CQRS** (Command Query Responsibility Segregation) pattern.

### Why Hexagonal Architecture?

1. **Security Isolation**: Clear boundaries between trusted core domain and untrusted external adapters
2. **Testability**: Domain logic tested independently of infrastructure
3. **Flexibility**: Swap implementations (e.g., storage backends) without affecting core
4. **Separation of Concerns**: Business logic isolated from technical concerns
5. **Explicit Dependencies**: All external dependencies flow through ports

## High-Level Hexagonal View

```
┌────────────────────────────────────────────────────────────────────┐
│                        EXTERNAL WORLD                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐            │
│  │   Browser    │  │  PostgreSQL  │  │  GStreamer   │            │
│  │   (WebRTC)   │  │  (Database)  │  │  (Video)     │            │
│  └───────┬──────┘  └──────┬───────┘  └──────┬───────┘            │
└──────────┼─────────────────┼──────────────────┼────────────────────┘
           │                 │                  │
           │ Primary Ports   │ Secondary Ports  │
           │ (Driving)       │ (Driven)         │
           │                 │                  │
┌──────────▼─────────────────▼──────────────────▼────────────────────┐
│                       ADAPTERS LAYER                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐            │
│  │ HTTP/WS      │  │ PostgreSQL   │  │ GStreamer    │            │
│  │ Adapter      │  │ Adapter      │  │ Adapter      │            │
│  └───────┬──────┘  └──────┬───────┘  └──────┬───────┘            │
└──────────┼─────────────────┼──────────────────┼────────────────────┘
           │                 │                  │
           │ Port Interfaces │ Port Interfaces  │
           │                 │                  │
┌──────────▼─────────────────▼──────────────────▼────────────────────┐
│                   APPLICATION LAYER (CQRS)                         │
│  ┌────────────────────────────────────────────────────────┐       │
│  │  Command Handlers          Query Handlers              │       │
│  │  ────────────────           ─────────────              │       │
│  │  • CreateSessionCommand    • GetSessionQuery           │       │
│  │  • AuthenticateCommand     • ListFilesQuery            │       │
│  │  • ForwardInputCommand     • GetAuditLogsQuery         │       │
│  │  • TerminateSessionCommand                             │       │
│  └─────────────────────┬──────────────────────────────────┘       │
└────────────────────────┼───────────────────────────────────────────┘
                         │
┌────────────────────────▼───────────────────────────────────────────┐
│                      DOMAIN LAYER (CORE)                           │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────┐     │
│  │  Aggregates                                              │     │
│  │  ──────────                                              │     │
│  │  • Session (root)         • SandboxEnvironment           │     │
│  │  • User (root)            • VideoStream                  │     │
│  │  • Permission (root)      • InputEvent                   │     │
│  └──────────────────────────────────────────────────────────┘     │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────┐     │
│  │  Domain Services                                         │     │
│  │  ───────────────                                         │     │
│  │  • SandboxIsolationService  • AuthenticationService      │     │
│  │  • EncryptionService        • AuthorizationService       │     │
│  │  • VideoEncodingService     • AuditService               │     │
│  └──────────────────────────────────────────────────────────┘     │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────┐     │
│  │  Value Objects                                           │     │
│  │  ─────────────                                           │     │
│  │  • UserId          • SessionId       • FilePermission    │     │
│  │  • ResourceLimits  • VideoConfig     • IpAddress         │     │
│  │  • EncryptionKey   • JwtToken        • AuditEvent        │     │
│  └──────────────────────────────────────────────────────────┘     │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────┐     │
│  │  Repository Interfaces (Ports)                           │     │
│  │  ─────────────────────────                               │     │
│  │  • UserRepository       • SessionRepository              │     │
│  │  • PermissionRepository • AuditLogRepository             │     │
│  └──────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
```

## High-Level Architecture (Traditional View)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         CLIENT USER BROWSER (Sandboxed Mode)               │
│  ┌──────────────────┐  ┌──────────────┐  ┌──────────────┐                │
│  │ File Explorer UI │  │  WebRTC      │  │  WebSocket   │                │
│  │ (React/Vue)      │  │  Video Stream│  │  Signaling   │                │
│  │ Read-Only View   │  │  (Sandboxed) │  │              │                │
│  └──────────────────┘  └──────────────┘  └──────────────┘                │
│  ❌ No Downloads  ❌ No Copy  ❌ No Local File Access                     │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         OWNER USER BROWSER (Browser Mode)                  │
│  ┌──────────────────┐  ┌──────────────┐  ┌──────────────┐                │
│  │ File Explorer    │  │  FileSystem  │  │  Download    │                │
│  │ (WASM/JS)        │  │  Access API  │  │  Manager     │                │
│  │ Full Control     │  │  (Direct)    │  │              │                │
│  └──────────────────┘  └──────────────┘  └──────────────┘                │
│  ✅ Downloads  ✅ Copy/Paste  ✅ Full File Manipulation                   │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Rust Backend (Axum) - Application Server                │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐                 │
│  │ Auth API     │  │ Application  │  │ Session         │                 │
│  │ (WebAuthn)   │  │ Manager      │  │ Manager         │                 │
│  └──────────────┘  └──────────────┘  └─────────────────┘                 │
│                                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐                 │
│  │ WebRTC       │  │ Sandbox      │  │ File System     │                 │
│  │ Signaling    │  │ Controller   │  │ Abstraction     │                 │
│  └──────────────┘  └──────────────┘  └─────────────────┘                 │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                   Sandbox Isolation Layer (Landlock LSM)                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                    │
│  │ Network      │  │ Landlock     │  │ cgroups v2   │                    │
│  │ Isolation    │  │ File Policies│  │ Limits       │                    │
│  │ (No Internet)│  │ (Read-Only)  │  │              │                    │
│  └──────────────┘  └──────────────┘  └──────────────┘                    │
│                                                                              │
│  ┌──────────────┐  ┌──────────────┐                                        │
│  │ seccomp      │  │ Namespace    │                                        │
│  │ Syscall Block│  │ Isolation    │                                        │
│  └──────────────┘  └──────────────┘                                        │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│              Sandboxed File Explorer Instances (Client Users)              │
│                                                                              │
│  ┌────────────────────────────────────────────────┐                        │
│  │  Client Session 1                               │                        │
│  │  ┌────────────┐  ┌─────────────────────────┐  │                        │
│  │  │ Xvfb :100  │  │ File Explorer (Rust)    │  │                        │
│  │  │ + GStreamer │  │ - Read-only files       │  │                        │
│  │  │ Streaming  │  │ - PDF viewer            │  │                        │
│  │  │            │  │ - Image/Video preview   │  │                        │
│  │  └────────────┘  └─────────────────────────┘  │                        │
│  │  Files: /mnt/user_files (RO, Landlock-restricted)                       │
│  │  Network: DISABLED  Syscalls: FILTERED                                  │
│  └────────────────────────────────────────────────┘                        │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      Data Storage Layer                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                    │
│  │ PostgreSQL   │  │ Encrypted    │  │ Audit Logs   │                    │
│  │ (Users/Perms)│  │ File Storage │  │ (All Access) │                    │
│  │ (App State)  │  │ (Per-User)   │  │ (Immutable)  │                    │
│  └──────────────┘  └──────────────┘  └──────────────┘                    │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Hexagonal Architecture Layers

### Domain Layer (Core / Business Logic)

**Location:** `src/domain/`

The innermost layer containing pure business logic with zero dependencies on infrastructure. All external dependencies are abstracted through port interfaces.

**Components:**

#### Aggregates
Clusters of domain objects treated as a single unit for data changes.

- **Session Aggregate** - Manages user session lifecycle, sandbox state, video streaming
- **User Aggregate** - User identity, authentication credentials, roles
- **Permission Aggregate** - File/resource access permissions, authorization policies

See [DOMAIN_OBJECTS.md](DOMAIN_OBJECTS.md) for detailed documentation.

#### Domain Services
Business logic that doesn't belong to a single aggregate.

- **SandboxIsolationService** - Creates isolated namespaces, applies Landlock policies
- **AuthenticationService** - Validates credentials, issues JWT tokens
- **AuthorizationService** - Enforces RBAC policies
- **EncryptionService** - Encrypts/decrypts files and sensitive data
- **VideoEncodingService** - Manages GStreamer pipeline configuration
- **AuditService** - Records security events

#### Value Objects
Immutable objects defined by their attributes, not identity.

- **UserId, SessionId, FilePermission** - Identifiers
- **ResourceLimits** - CPU/memory/PID limits
- **VideoConfig** - Framerate, bitrate, codec settings
- **EncryptionKey** - Cryptographic keys
- **JwtToken** - Authentication tokens
- **AuditEvent** - Security event records

#### Repository Interfaces (Ports)
Abstractions for data persistence (implemented by adapters).

```rust
// Example port interface
trait UserRepository {
    async fn find_by_id(&self, id: &UserId) -> Result<User>;
    async fn find_by_username(&self, username: &str) -> Result<User>;
    async fn save(&self, user: &User) -> Result<()>;
}
```

### Application Layer (Use Cases)

**Location:** `src/application/`

Orchestrates domain objects to fulfill use cases. Implements CQRS pattern with commands (write operations) and queries (read operations).

**Components:**

#### Command Handlers (Write Operations)
See [COMMANDS.md](COMMANDS.md) for detailed documentation.

- **CreateSessionCommand** - Initialize new sandbox session
- **AuthenticateUserCommand** - Validate credentials and issue tokens
- **ForwardInputCommand** - Send mouse/keyboard input to sandbox
- **TerminateSessionCommand** - Stop and cleanup sandbox
- **GrantPermissionCommand** - Give user access to file
- **RevokePermissionCommand** - Remove user access

#### Query Handlers (Read Operations)
See [QUERIES.md](QUERIES.md) for detailed documentation.

- **GetSessionQuery** - Retrieve session details
- **ListSessionsQuery** - Get all sessions for user
- **ListFilesQuery** - Get accessible files
- **GetAuditLogsQuery** - Retrieve audit trail
- **GetUserQuery** - Fetch user details

#### Application Services
Coordinate multiple commands/queries for complex workflows.

- **SessionOrchestrationService** - Manages session lifecycle
- **AuthenticationFlowService** - Handles login/logout/refresh flows

### Adapters Layer (Infrastructure)

**Location:** `src/adapters/`

Implements port interfaces, connecting domain to external systems.

#### Primary Adapters (Driving / Inbound)
External systems that drive the application.

**HTTP/WebSocket Adapter** (`src/adapters/http/`)
- REST API endpoints
- WebSocket signaling server
- Maps HTTP requests to commands/queries
- Serializes responses

**CLI Adapter** (`src/adapters/cli/`)
- Command-line interface for admin tasks
- User creation, system management

#### Secondary Adapters (Driven / Outbound)
External systems driven by the application.

**PostgreSQL Adapter** (`src/adapters/persistence/postgres/`)
- Implements repository interfaces
- SQL queries and migrations
- Connection pool management

**GStreamer Adapter** (`backend/src/infrastructure/driven/sandbox/`)
- Video capture from Xvfb via ximagesrc
- VP8 encoding via vp8enc
- Pipeline lifecycle management

**Sandbox Adapter** (`src/adapters/sandbox/linux/`)
- Linux namespace creation
- Landlock policy application
- cgroups resource limits
- seccomp filter setup

**WebRTC Adapter** (`src/adapters/webrtc/`)
- Peer connection management
- SDP offer/answer exchange
- ICE candidate handling
- Media track streaming

**Encryption Adapter** (`src/adapters/encryption/`)
- File encryption/decryption
- Key derivation and management

**Audit Adapter** (`src/adapters/audit/`)
- Append-only log storage
- Structured event logging

### Ports (Interfaces)

**Location:** `src/ports/`

Defines contracts between layers. Domain layer defines port interfaces; adapters implement them.

**Primary Ports (Inbound):**
```rust
// Commands
trait CommandHandler<C> {
    async fn handle(&self, command: C) -> Result<CommandResult>;
}

// Queries  
trait QueryHandler<Q, R> {
    async fn handle(&self, query: Q) -> Result<R>;
}
```

**Secondary Ports (Outbound):**
```rust
// Repository pattern
trait UserRepository { ... }
trait SessionRepository { ... }
trait PermissionRepository { ... }
trait AuditLogRepository { ... }

// External services
trait VideoEncodingPort { ... }
trait SandboxIsolationPort { ... }
trait EncryptionPort { ... }
```

## Core Components (Detailed)

### 1. HTTP/WebSocket Adapter (Primary)

**Responsibilities:**
- HTTP API endpoints for authentication and session management
- WebSocket signaling for WebRTC peer connection negotiation
- Static file serving for client application
- JWT token validation and session lifecycle
- Maps REST requests to commands/queries

**Key Modules:**
- `adapters::http::api` - REST API handlers
- `adapters::http::websocket` - WebSocket connection manager
- `adapters::http::auth` - Authentication middleware
- `adapters::http::middleware` - Rate limiting, CORS, security headers

### 2. Sandbox Adapter (Secondary)

**Responsibilities:**
- Implements `SandboxIsolationPort` interface
- Create isolated Linux namespaces (user, mount, PID, network, IPC)
- Apply Landlock filesystem access policies
- Configure cgroups v2 resource limits
- Inject seccomp-bpf syscall filters
- Mount custom filesystem views

**Key Modules:**
- `adapters::sandbox::linux::namespace` - Namespace creation
- `adapters::sandbox::linux::landlock` - Filesystem access control
- `adapters::sandbox::linux::cgroups` - Resource limit enforcement
- `adapters::sandbox::linux::seccomp` - Syscall filtering
- `adapters::sandbox::linux::mount` - Filesystem mount orchestration

**Isolation Primitives:**

```
┌─────────────────────────────────────┐
│   Application (user-facing API)     │
├─────────────────────────────────────┤
│ Landlock: File access policies      │ ← Fine-grained FS control
├─────────────────────────────────────┤
│ seccomp-bpf: Syscall filtering      │ ← Attack surface reduction
├─────────────────────────────────────┤
│ Namespaces: Isolation primitives    │ ← Core isolation
│  - User (rootless operation)        │
│  - Mount (custom FS view)           │
│  - PID, Network, IPC, UTS           │
├─────────────────────────────────────┤
│ cgroups v2: Resource limits         │ ← CPU/memory/I/O quotas
└─────────────────────────────────────┘
```

### 3. Video Encoding Adapter (Secondary)

**Responsibilities:**
- Implements `VideoEncodingPort` interface
- Capture X11 display from isolated Xvfb instance via ximagesrc
- Encode video stream to VP8 with low-latency settings
- Pipe encoded frames to WebRTC media track via appsink
- Handle pipeline lifecycle (start/stop per session)

**Technology:**
- GStreamer via the `gstreamer` crate
- ximagesrc for X11 display capture
- VP8 codec via vp8enc

**Pipeline:**
```
ximagesrc display=:N → videoconvert → capsfilter (I420) → vp8enc → appsink → WebRTC
```

**Key Modules:**
- `backend/src/infrastructure/driven/sandbox/gstreamer.rs` - GStreamer pipeline management
- `backend/src/infrastructure/driven/sandbox/xvfb.rs` - Xvfb + XTEST input injection

### 4. WebRTC Adapter (Secondary)

**Responsibilities:**
- Implements WebRTC streaming port
- Establish peer connections with browser clients
- Send video via RTP media tracks
- Receive input via Data Channels
- Handle ICE candidate exchange and STUN/TURN

**Key Components:**
- `adapters::webrtc::peer` - Peer connection management
- `adapters::webrtc::media` - Media track handling
- `adapters::webrtc::signaling` - SDP offer/answer exchange
- `adapters::webrtc::ice` - ICE candidate gathering

**Library:** `webrtc-rs` crate

### 5. Input Forwarding (Domain Service + Adapter)

**Domain Service:** `domain::services::InputValidationService`
- Validates input events against security policies
- Rate limiting logic
- Event sanitization

**Adapter:** `adapters::input::x11::X11InputAdapter`
- Implements `InputInjectionPort`
- Injects events into sandboxed X11 session via X11 XTEST extension (`xtest_fake_input` from x11rb)
- Maps abstract input events to X11 protocol

### 6. Permission Domain (Aggregate + Repository)

**Domain Aggregate:** `domain::aggregates::Permission`
- Encapsulates file/resource access rules
- Validates permission grants/revocations
- Enforces read/write/execute constraints

**Repository Adapter:** `adapters::persistence::postgres::PermissionRepositoryImpl`
- Stores permissions in PostgreSQL
- Queries permissions by user/file

**Encryption Adapter:** `adapters::encryption::FileEncryptionAdapter`
- Implements `EncryptionPort`
- Encrypts files at rest using AES-256-GCM
- Key derivation and management

## Data Flow

### Session Initialization

```
1. User → HTTPS POST /api/auth/login
   ↓
2. Server validates credentials (PostgreSQL + argon2)
   ↓
3. Server generates JWT token
   ↓
4. User → WebSocket connect with JWT
   ↓
5. Server creates sandbox environment:
   - Spawn namespace with user/mount/PID isolation
   - Apply Landlock policy for file access
   - Set cgroups limits (CPU/memory)
   - Apply seccomp filter
   - Mount user files and X11 libraries
   ↓
6. Server launches Xvfb :100 in sandbox
   ↓
7. Server starts GStreamer pipeline: ximagesrc → vp8enc → appsink
   ↓
8. Server initiates WebRTC peer connection
   ↓
9. Browser receives SDP offer via WebSocket
   ↓
10. WebRTC connection established
   ↓
11. Video stream flows: Xvfb → GStreamer → WebRTC → Browser
```

### Input Handling

```
1. User clicks/types in browser
   ↓
2. JavaScript captures DOM events
   ↓
3. Events sent via WebRTC Data Channel
   ↓
4. Server validates event format
   ↓
5. Server rate-limits (prevent abuse)
   ↓
6. Server injects via X11 XTEST (x11rb) into Xvfb :100
   ↓
7. Application in sandbox receives input
```

### Session Teardown

```
1. User disconnects or timeout expires
   ↓
2. Server closes WebRTC connection
   ↓
3. Server stops GStreamer pipeline
   ↓
4. Server kills processes in PID namespace
   ↓
5. Server unmounts sandboxed filesystem
   ↓
6. Server removes cgroup
   ↓
7. Server logs session metadata
```

## Security Model

### Defense in Depth

1. **Network Layer**
   - TLS 1.3 for all HTTPS/WSS connections
   - DTLS-SRTP for WebRTC media encryption
   - No direct client access to file storage

2. **Authentication Layer**
   - argon2id password hashing
   - JWT tokens with short expiry (15 min access, 7 day refresh)
   - Session invalidation on logout

3. **Authorization Layer**
   - Role-based access control (RBAC)
   - Per-file read/write/execute permissions
   - Server-side enforcement before mounting

4. **Isolation Layer**
   - User namespaces (rootless containers)
   - Mount namespaces (custom FS views)
   - PID namespaces (process isolation)
   - Network namespaces (optional network isolation)

5. **Filesystem Layer**
   - Landlock access policies (kernel-enforced)
   - Read-only mounts for system libraries
   - Write access only to user-specific paths
   - No access to host filesystem outside allowed paths

6. **Syscall Layer**
   - seccomp-bpf filters
   - Deny dangerous syscalls: `ptrace`, `kexec`, `module_load`
   - Allow only required syscalls for X11 apps

7. **Resource Layer**
   - cgroups v2 CPU limits (prevent DoS)
   - Memory limits (OOM isolation)
   - I/O bandwidth limits
   - PID limits (fork bomb protection)

### Threat Model

**Protected Against:**
- Data exfiltration via browser (no downloads, copy disabled)
- Privilege escalation (user namespaces + seccomp)
- Container escape (namespace isolation)
- Resource exhaustion (cgroups limits)
- File access outside permissions (Landlock)

**Not Protected Against:**
- Screen recording by external devices (physical security)
- Compromised client browser (video stream is visible)
- Side-channel attacks (timing, cache)
- Kernel vulnerabilities (requires host hardening)

## Performance Characteristics

### Latency Budget

| Component | Latency | Notes |
|-----------|---------|-------|
| Namespace creation | <5ms | User namespace setup |
| Xvfb startup | ~50ms (polls for X11 socket) | Virtual display initialization |
| GStreamer encoding | 16-33ms | At 30-60 FPS |
| WebRTC transmission | 20-100ms | Network dependent |
| Input injection | <1ms | X11 XTEST overhead |
| **Total (local)** | **~100ms** | Glass-to-glass latency |

### Resource Usage (per session)

| Resource | Usage | Limit |
|----------|-------|-------|
| Memory | 100-500MB | Configurable via cgroups |
| CPU | 10-50% | Configurable via cgroups |
| Disk I/O | Varies | Limited via cgroups |
| Network | 2-10 Mbps | Video bitrate dependent |
| Startup overhead | <1MB | Namespace/cgroup metadata |

### Scalability

**Single Server:**
- Expected: 20-50 concurrent sessions
- Bottleneck: CPU for video encoding
- Mitigation: Hardware encoding (NVENC/VAAPI)

**Multi-Node:**
- Session affinity via load balancer
- Shared PostgreSQL + file storage (NFS/S3)
- Stateless server design enables horizontal scaling

## Traceability & Logging Architecture

**CRITICAL REQUIREMENT: TOTAL TRACEABILITY**

Everything in the system MUST be logged for security, compliance, and forensics. See [TRACEABILITY.md](TRACEABILITY.md) for complete analysis.

### Hybrid Logging Strategy

The system uses a **multi-layer logging approach** for comprehensive traceability with minimal performance impact (~5-10% overhead):

```
┌─────────────────────────────────────────────────────┐
│              HYBRID LOGGING LAYERS                  │
├─────────────────────────────────────────────────────┤
│  1. Command/Query Interceptor (Automatic)          │
│     → ALL commands & queries logged                │
│     → Zero boilerplate, correlation IDs            │
│                                                     │
│  2. Domain Events (Selective)                      │
│     → Business-significant events only             │
│     → SessionCreated, PermissionGranted, etc.      │
│                                                     │
│  3. Security Event Logger (Direct)                 │
│     → Authentication, authorization                │
│     → Synchronous, high-priority                   │
│                                                     │
│  4. Adapter Logging (Infrastructure)               │
│     → Database, GStreamer, WebRTC, Sandbox          │
│                                                     │
│  5. Structured Tracing (Development)               │
│     → Performance metrics, debugging               │
└─────────────────────────────────────────────────────┘
```

**Why Not Domain Events Only?**

While domain events provide clean separation, using them for EVERYTHING would:
- Add 5-15% overhead (event creation, dispatch, serialization)
- Require events for even trivial operations (query execution)
- Create async complexity for synchronous requirements (security logs)
- Result in "event explosion" with hundreds of event types

The hybrid approach uses the **right tool for each type of event**:
- Interceptors: Automatic logging of all operations (zero boilerplate)
- Domain Events: Business-significant state changes (rich context)
- Direct Logging: Security-critical events (synchronous, immediate)
- Adapter Logging: Infrastructure operations (performance metrics)

See [TRACEABILITY.md](TRACEABILITY.md) for detailed tradeoff analysis and implementation patterns.

## Technology Stack Summary

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Language | Rust | Memory safety, performance |
| Web Framework | Axum | Async, ergonomic, WebSocket support |
| WebRTC | webrtc-rs | Pure Rust WebRTC implementation |
| Database | PostgreSQL | ACID compliance, mature |
| DB Client | sqlx | Async, compile-time query checking |
| Video Encoding | GStreamer (VP8) | Flexible pipeline, ximagesrc capture |
| Sandboxing | Namespaces | Native Linux, zero overhead |
| Filesystem Control | Landlock | Kernel-enforced, unprivileged |
| Syscall Filter | seccomp-bpf | Attack surface reduction |
| Resource Limits | cgroups v2 | Unified hierarchy, comprehensive |
| Auth | JWT + argon2 | Stateless + secure hashing |
| Logging | tracing + custom | Structured, comprehensive |
| Audit Trail | PostgreSQL | Append-only, encrypted

## Future Enhancements

1. **Wayland Support** - Better per-app isolation than X11
2. **GPU Passthrough** - For CAD/3D applications
3. **Collaborative Sessions** - Multiple users in same sandbox
4. **Recording/Playback** - Audit trail and compliance
5. **Mobile Clients** - iOS/Android WebRTC apps
6. **File Upload** - Secure file ingestion to sandboxes
7. **Clipboard Sync** - Controlled copy/paste between client/sandbox
8. **Kubernetes Operator** - Cloud-native deployment
