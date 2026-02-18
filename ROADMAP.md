# Roadmap: Secure Sandbox Platform — End-to-End Security Implementation

## How to Read This Document

- `[x]` = implemented and working
- `[ ]` = not yet implemented
- Each phase builds on the previous. Phases are ordered by dependency.
- This is a living document — check boxes as work completes.

---

## Current State (Baseline)

**What exists:**
- [x] Domain: `User` entity with `UserRole` (SuperAdmin / Owner / Client), `UserStatus`
- [x] Domain: `Credential` entity (WebAuthn passkey)
- [x] DB: `users` + `webauthn_credentials` tables (2 migrations)
- [x] Application: SuperAdmin WebAuthn registration flow (initiate + complete)
- [x] Application: WebAuthn login flow — challenge/response works, but **credential verification is disabled in code**
- [x] Infrastructure: PostgreSQL user + credential repositories
- [x] Infrastructure: Redis challenge repository (5 min TTL)
- [x] Infrastructure: Xvfb per-session lifecycle (`start_xvfb`, `launch_app`, `start_capture`, `cleanup_session`)
- [x] Infrastructure: GStreamer VP8 pipeline (`ximagesrc → vp8enc → appsink`)
- [x] Infrastructure: X11 XTEST input injection via x11rb
- [x] Infrastructure: Xvfb socket polling (10ms intervals, 5s timeout)
- [x] App: File Explorer native binary (eframe/egui, reads `DISPLAY`, browsable from `/`)
- [x] IPC: Unix socket protocol designed (`PlatformMessage` / `AppMessage`)
- [x] WebRTC: Signaling, peer connection, VP8 video track (DTLS-SRTP)
- [x] WebRTC: STUN/TURN configuration

**What is absent (nothing implemented yet):**
- Per-user storage directories
- Owner and Client user creation flows
- Invitation / sharing model (DB tables, domain, API)
- Session table and session-role context
- File management API
- Sandbox security enforcement (Landlock, namespaces, seccomp, cgroups)
- Real-time permission push (session restart on revoke)
- Audit logging
- WebRTC / transport hardening

---

## Phase 1 — Authentication Completion

Fix the existing auth plumbing before building on top of it.

### 1.1 Login credential verification
**File:** `backend/src/application/super_admin/commands/complete_webauthn_login.rs`

- [ ] Re-enable credential lookup by email (`credential_repo.find_by_user_id`)
- [ ] Call `webauthn.finish_passkey_authentication()` with the stored passkey
- [ ] Update `sign_count` on the stored credential after successful auth
- [ ] Return `403` on failed verification (not a silent pass)

### 1.2 JWT middleware
**New file:** `backend/src/infrastructure/driving/http/middleware/auth.rs`

- [ ] Extract `Authorization: Bearer <token>` from requests
- [ ] Validate JWT signature (HS256, `JWT_SECRET` env var — already read in main.rs)
- [ ] Reject expired tokens
- [ ] Inject `AuthenticatedUser { id, email, role }` as Axum extension
- [ ] Apply middleware to all routes except `/api/setup/*`, `/api/auth/*`, `/health`

### 1.3 SuperAdmin: create Owner user
**New:** `backend/src/application/super_admin/commands/create_owner.rs`
**New route:** `POST /api/admin/users` (SuperAdmin only)

- [ ] Validate caller has `UserRole::SuperAdmin`
- [ ] Accept `{ email, display_name }` body
- [ ] Create `User` with `UserRole::Owner`, `UserStatus::Active`
- [ ] Persist user; return user ID
- [ ] On creation: generate owner storage directory (see Phase 2)

### 1.4 Owner WebAuthn registration (invitation-based)
**New:** `backend/src/application/owner/commands/register_owner.rs`

- [ ] SuperAdmin sends invite link containing a signed token (`owner_invite:{user_id}`)
- [ ] Owner opens link, initiates WebAuthn registration (`POST /api/owner/register/initiate`)
- [ ] On completion (`POST /api/owner/register/complete`): verify token, save passkey, activate user

### 1.5 Client user creation (invitation-based, Phase 3 prerequisite)
Deferred — covered in Phase 3. Client users are created implicitly when they accept an invitation.

---

## Phase 2 — Per-User Storage

Every Owner gets an isolated directory. Client users never get their own storage root — they access a subset of an Owner's tree.

### 2.1 Storage root on user creation
**File:** `backend/src/application/super_admin/commands/create_owner.rs` (Phase 1.3)
**Env var:** `STORAGE_PATH` (already defined, e.g. `/data/storage`)

- [ ] On Owner user creation: `mkdir -p $STORAGE_PATH/{owner_id}/`
- [ ] Set directory ownership to backend process UID
- [ ] Storage root is derived at runtime as `$STORAGE_PATH/{owner_id}` — no DB column needed

### 2.2 File management API
**File:** `backend/src/infrastructure/driving/http/files.rs` (exists but empty)
**Auth:** Owner JWT required on all endpoints

- [ ] `GET  /api/files?path=<relative>` — list directory contents (name, size, modified, type)
- [ ] `POST /api/files/upload` — multipart upload; store under `storage_root/{path}`
- [ ] `GET  /api/files/download/{encoded_path}` — stream file to caller
- [ ] `DELETE /api/files/{encoded_path}` — delete file or empty directory
- [ ] `POST /api/files/mkdir` — create directory
- [ ] `PUT  /api/files/rename` — rename/move within storage root
- [ ] All paths are **relative to the owner's storage root** — backend prepends root and rejects `../` traversal

### 2.3 File Explorer app: scoped root
**File:** `apps/file-explorer/src/app.rs`
**New env var:** `ROOT_PATH` (set by backend when launching the app)

- [ ] Read `ROOT_PATH` from environment; default to `/` only if not set
- [ ] Clamp all navigation to within `ROOT_PATH` (disallow `..` traversal above root)
- [ ] Display relative path in UI breadcrumb (strip `ROOT_PATH` prefix)

### 2.4 Backend passes `ROOT_PATH` to app
**File:** `backend/src/infrastructure/driven/sandbox/xvfb.rs` → `launch_app()`

- [ ] Add `root_path: &str` parameter to `launch_app`
- [ ] Set `ROOT_PATH={root_path}` in app env vars
- [ ] For Owner session: `root_path = $STORAGE_PATH/{owner_id}`
- [ ] For Client session: `root_path = $STORAGE_PATH/{owner_id}` (Landlock restricts further — Phase 5)

---

## Phase 3 — Invitation & Permission Model

This is the core of the sharing system. An Owner invites a Client and specifies which paths (and what access level) the client gets.

### 3.1 Database migrations
**New migration:** `create_invitations_and_permissions`

- [ ] `invitations` table:
  ```sql
  id UUID PRIMARY KEY,
  owner_id UUID REFERENCES users(id),
  invitee_email VARCHAR(255),
  token VARCHAR(64) UNIQUE NOT NULL,   -- random secure token
  granted_paths JSONB NOT NULL,         -- [{path, access: [read,write,delete]}]
  status VARCHAR(20) NOT NULL,          -- pending, accepted, revoked, expired
  expires_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
  ```
- [ ] `file_permissions` table:
  ```sql
  id UUID PRIMARY KEY,
  owner_id UUID REFERENCES users(id),
  client_id UUID REFERENCES users(id),
  path TEXT NOT NULL,                   -- relative to owner's storage root
  access TEXT[] NOT NULL,              -- {read, write, delete}
  granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ,
  revoked_at TIMESTAMPTZ
  ```
- [ ] Index: `(client_id, revoked_at)` for fast active-permission lookups
- [ ] Index: `(owner_id, client_id)` for per-pair queries

### 3.2 Domain entities
- [ ] `Invitation` entity: `backend/src/domain/entities/invitation.rs`
  - Fields: id, owner_id, invitee_email, token, granted_paths, status, expires_at
  - Methods: `is_valid()`, `mark_accepted()`, `revoke()`
- [ ] `FilePermission` entity: `backend/src/domain/entities/file_permission.rs`
  - Fields: id, owner_id, client_id, path, access, granted_at, expires_at, revoked_at
  - Methods: `is_active()`, `allows(AccessLevel)`, `revoke()`
- [ ] `GrantedPath` value object: `{ path: String, access: Vec<AccessLevel> }`
- [ ] `AccessLevel` enum: `Read`, `Write`, `Delete`

### 3.3 Repository ports
- [ ] `InvitationRepository` trait: `save`, `find_by_token`, `find_by_owner`, `update_status`
- [ ] `FilePermissionRepository` trait: `save`, `find_active_for_client`, `find_by_owner_client`, `revoke`
- [ ] PostgreSQL implementations for both

### 3.4 Owner: create invitation
**New:** `backend/src/application/owner/commands/create_invitation.rs`
**Route:** `POST /api/invitations` (Owner JWT)

- [ ] Validate caller is Owner
- [ ] Accept `{ invitee_email, granted_paths: [{path, access}], expires_in_hours }`
- [ ] Validate all paths are within owner's storage root (no `../`)
- [ ] Generate cryptographically random token (32 bytes → hex)
- [ ] Persist invitation; return `{ invitation_id, token, invite_url }`
- [ ] (Optional) Send email via SMTP with invite link (env: `SMTP_*` already configured)

### 3.5 Client: view & accept invitation
**Route:** `GET  /api/invitations/{token}` — public, shows what's being shared
**Route:** `POST /api/invitations/{token}/accept` — accepts, creates Client user + permissions

- [ ] `GET`: return `{ owner_display_name, granted_paths, expires_at }` (no sensitive data)
- [ ] `POST` accept flow:
  - [ ] Load invitation; check `is_valid()` (not expired, not revoked, not already accepted)
  - [ ] If `invitee_email` is a known user: link to existing; else create new `User` with `UserRole::Client`
  - [ ] Optionally trigger WebAuthn registration for new client user
  - [ ] Create `FilePermission` rows for each `granted_path`
  - [ ] Mark invitation `status = accepted`
  - [ ] Return JWT for the client user

### 3.6 Owner: list/revoke permissions
**Route:** `GET    /api/permissions?client_id=<id>` (Owner JWT)
**Route:** `DELETE /api/permissions/{id}` (Owner JWT)

- [ ] List returns all active permissions the owner has granted
- [ ] Revoke sets `revoked_at = NOW()` on the permission row
- [ ] Revoke triggers session restart if client has an active session (Phase 6)

### 3.7 Client: list accessible paths
**Route:** `GET /api/my-permissions` (Client JWT)

- [ ] Return all non-revoked, non-expired FilePermissions for the calling client
- [ ] Used by the client's file explorer to know what they can navigate

---

## Phase 4 — Session Model with Role & Permission Context

Currently the app launches without knowing who the user is or what they're allowed. This phase ties identity + permissions into the launch flow.

### 4.1 Sessions database table
**New migration:** `create_sessions_table`

- [ ] `sessions` table:
  ```sql
  id UUID PRIMARY KEY,
  user_id UUID REFERENCES users(id),
  owner_id UUID REFERENCES users(id),  -- NULL for Owner sessions (owner_id = user_id)
  role user_role NOT NULL,
  app_id TEXT NOT NULL,
  display_number SMALLINT,
  state TEXT NOT NULL,                 -- initializing, ready, active, terminated
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ NOT NULL,
  terminated_at TIMESTAMPTZ
  ```

### 4.2 Session repository port + implementation
- [ ] `SessionRepository` trait: `save`, `find_by_id`, `find_active_by_user`, `update_state`, `terminate`
- [ ] PostgreSQL implementation

### 4.3 Launch application — auth-aware
**File:** `backend/src/application/client/commands/launch_application.rs`
**Route:** `POST /api/applications/launch` — now requires JWT

- [ ] Extract `AuthenticatedUser` from JWT middleware extension
- [ ] If `role == Owner`: `root_path = $STORAGE_PATH/{user_id}`, full read/write/delete on root
- [ ] If `role == Client`:
  - [ ] Load active `FilePermission` rows for this client
  - [ ] Resolve each path against owner's storage root
  - [ ] `allowed_paths` = resolved permission paths only
  - [ ] `root_path` = owner's storage root (Landlock restricts further)
- [ ] Create `Session` record in DB
- [ ] Call `xvfb_manager.start_xvfb`, `launch_app` (passing `root_path`, `allowed_paths`), `start_capture`
- [ ] Return `{ session_id, websocket_url }`

### 4.4 Session expiry enforcement
- [ ] Sessions have `expires_at` (default 1h via `SESSION_TIMEOUT` env var)
- [ ] Background task: scan for expired sessions every 60s; call `cleanup_session` on expired ones
- [ ] WebSocket disconnection sets session state to `terminated`

---

## Phase 5 — Sandbox Security Enforcement

The actual kernel-level isolation. Each item is independent and can be shipped incrementally.

### 5.1 App filesystem scoping (application-level, defence-in-depth)
**File:** `backend/src/infrastructure/driven/sandbox/xvfb.rs` → `launch_app()`
**File:** `apps/file-explorer/src/app.rs`

- [ ] Pass `ALLOWED_PATHS` env var (colon-separated list) for Client sessions
- [ ] File Explorer reads `ALLOWED_PATHS` and hides navigation outside the allowed set
- [ ] This is **UX enforcement only** — kernel-level enforcement is below

### 5.2 Landlock LSM filesystem enforcement
**New file:** `backend/src/infrastructure/driven/sandbox/landlock.rs`
**Dependency:** `landlock` crate

- [ ] Add `landlock` crate to `backend/Cargo.toml`
- [ ] Implement `apply_landlock(allowed_paths: &[(String, AccessRights)])` function
  - Uses `Ruleset::new()`, adds `PathBeneath` rules per path
  - Calls `restrict_self()` — applies to current process and all descendants
- [ ] In `launch_app()`: call `apply_landlock` inside `pre_exec` closure before exec
  - Owner: `[(root_path, ReadWrite | Remove)]`
  - Client: `[(path, ReadOnly)]` per granted permission
- [ ] Test: app process gets `EPERM` on paths outside the ruleset

### 5.3 Network isolation (network namespace)
**File:** `backend/src/infrastructure/driven/sandbox/xvfb.rs` → `launch_app()`

- [ ] In `pre_exec`: call `libc::unshare(CLONE_NEWNET)` — new network namespace with no interfaces
- [ ] App has no network access (cannot exfiltrate via HTTP, DNS, etc.)
- [ ] Xvfb continues running in the host network namespace

### 5.4 Mount namespace (filesystem view isolation)
**File:** `backend/src/infrastructure/driven/sandbox/xvfb.rs`

- [ ] In `pre_exec`: call `unshare(CLONE_NEWNS)` to create a new mount namespace
- [ ] Bind-mount only required directories:
  - Owner's storage root (or client's allowed paths)
  - `/tmp/.X11-unix/X{N}` — Xvfb socket
  - Minimal system libs required by the app binary
- [ ] Paths outside the mount namespace don't exist (`ENOENT`, not `EACCES`)

### 5.5 seccomp syscall filtering
**New file:** `backend/src/infrastructure/driven/sandbox/seccomp.rs`
**Dependency:** `seccompiler` crate

- [ ] Add `seccompiler` crate to `backend/Cargo.toml`
- [ ] Build syscall allowlist for X11 desktop apps (read, write, open, mmap, socket AF_UNIX only, futex, clock_gettime, etc.)
- [ ] Deny with SIGSYS: `mount`, `pivot_root`, `kexec_load`, `ptrace`, `process_vm_readv`, `setuid`, `setgid`
- [ ] Apply filter in `pre_exec` after Landlock

### 5.6 cgroups v2 resource limits
**New file:** `backend/src/infrastructure/driven/sandbox/cgroups.rs`

- [ ] Create cgroup at `/sys/fs/cgroup/sandbox/{session_id}/` on session start
- [ ] Write app PID into `cgroup.procs`
- [ ] Set limits: `cpu.max = 500000 1000000` (50%), `memory.max = 512MB`, `pids.max = 100`
- [ ] On session cleanup: remove cgroup directory

---

## Phase 6 — Real-Time Permission Enforcement

When an owner revokes access mid-session, enforcement must happen immediately.

### 6.1 Active session registry (client → session lookup)
**File:** `backend/src/infrastructure/driven/sandbox/xvfb.rs`

- [ ] Add lookup: `find_session_by_user_id(user_id) -> Option<session_id>`

### 6.2 Permission revocation triggers session restart
**File:** `backend/src/application/owner/commands/revoke_permission.rs`

- [ ] After setting `revoked_at` in DB: check if client has an active Xvfb session
- [ ] If yes: `cleanup_session` + re-launch with updated (narrowed) permissions
- [ ] Client experiences ~2s reconnect; Landlock enforces immediately with new process

### 6.3 Owner dashboard WebSocket channel
**Route:** `GET /ws/owner` (Owner JWT required before upgrade)

- [ ] Push events: `session_started`, `session_ended`, `file_opened`, `permission_revoked`
- [ ] Source: IPC `AppMessage::State` events from the running app

### 6.4 Client session WebSocket notifications
**File:** `backend/src/infrastructure/driving/webrtc.rs` (extend existing WS)

- [ ] Push `{ event: "permission_revoked", path }` when owner revokes
- [ ] Push `{ event: "session_expiring", seconds_remaining: 300 }` on 5-min warning
- [ ] Push `{ event: "session_terminated" }` when owner kills session

### 6.5 Audit log
**New migration:** `create_audit_log_table`

- [ ] `audit_events` table: `id`, `session_id`, `user_id`, `owner_id`, `event_type`, `payload JSONB`, `created_at`
- [ ] Write on: session start/end, file open (from IPC), permission grant/revoke, invitation created/accepted
- [ ] `GET /api/audit?client_id=` (Owner JWT) — read audit trail

---

## Phase 7 — Transport & WebRTC Hardening

Can be done in parallel with Phases 5–6.

### 7.1 WebSocket authentication
**File:** `backend/src/infrastructure/driving/webrtc.rs`

- [ ] Extract JWT from `?token=` query param before WS upgrade
- [ ] Reject with `401` if token missing/invalid/expired
- [ ] Bind WS session to `session_id` from JWT claims

### 7.2 TLS / WSS
**Infrastructure:** `haproxy/` (already present)

- [ ] Enable TLS termination at HAProxy
- [ ] WebSocket served over `wss://`
- [ ] Set `WEBAUTHN_ORIGIN` to `https://` in production env

### 7.3 TURN server hardening
**File:** `backend/src/infrastructure/driving/webrtc.rs`

- [ ] Ensure `TURN_USERNAME` / `TURN_CREDENTIAL` have no hardcoded defaults
- [ ] Use `turns://` (TURN over TLS) in production
- [ ] Consider per-session time-limited TURN credentials (HMAC-SHA1 scheme)

### 7.4 Input rate limiting
**File:** `backend/src/infrastructure/driving/webrtc.rs`

- [ ] Cap input events at ~120/s per session
- [ ] Drop (not queue) events exceeding the limit
- [ ] Log excessive rate as audit event

---

## Phase 8 — Management APIs

API surface for the frontend to drive all the above features.

### 8.1 SuperAdmin APIs
- [ ] `GET  /api/admin/users` — list all users
- [ ] `POST /api/admin/users` — create Owner user (Phase 1.3)
- [ ] `PUT  /api/admin/users/{id}/status` — suspend / reactivate
- [ ] `DELETE /api/admin/users/{id}` — soft-delete

### 8.2 Owner APIs
- [ ] `GET  /api/storage/usage` — bytes used in owner's storage root
- [ ] `GET  /api/clients` — list Client users linked to this owner
- [ ] `GET  /api/clients/{client_id}/activity` — audit events for a specific client
- [ ] `DELETE /api/sessions/{session_id}` — terminate an active client session
- [ ] All invitation + permission endpoints (Phase 3)
- [ ] All file management endpoints (Phase 2.2)

### 8.3 Client APIs
- [ ] `GET  /api/my-files` — list files the client can access
- [ ] `GET  /api/my-session` — current session info + expiry
- [ ] `POST /api/applications/launch` — launch sandboxed session (Phase 4.3)

---

## Dependency Order

```
Phase 1 (Auth fix)
    → Phase 2 (Storage)         needs: Owner user creation
        → Phase 3 (Invitations) needs: storage + Owner user
            → Phase 4 (Session) needs: permissions
                → Phase 5 (Sandbox security) needs: session context
                    → Phase 6 (Real-time) needs: Landlock from Phase 5
Phase 7 (Transport) — independent, any time
Phase 8 (APIs)      — builds on each phase as it completes
```

---

## Critical Files

| File | Change |
|------|--------|
| `backend/src/application/super_admin/commands/complete_webauthn_login.rs` | Re-enable credential verification |
| `backend/src/infrastructure/driving/http/middleware/auth.rs` | **New** — JWT middleware |
| `backend/src/application/super_admin/commands/create_owner.rs` | **New** — Owner creation + storage dir |
| `backend/migrations/…_create_invitations_and_permissions.sql` | **New** |
| `backend/migrations/…_create_sessions_table.sql` | **New** |
| `backend/migrations/…_create_audit_log.sql` | **New** |
| `backend/src/domain/entities/invitation.rs` | **New** |
| `backend/src/domain/entities/file_permission.rs` | **New** |
| `backend/src/application/owner/commands/create_invitation.rs` | **New** |
| `backend/src/application/owner/commands/revoke_permission.rs` | **New** |
| `backend/src/infrastructure/driven/sandbox/landlock.rs` | **New** |
| `backend/src/infrastructure/driven/sandbox/seccomp.rs` | **New** |
| `backend/src/infrastructure/driven/sandbox/cgroups.rs` | **New** |
| `backend/src/infrastructure/driven/sandbox/xvfb.rs` | Add `root_path`, `allowed_paths`, `pre_exec` isolation |
| `apps/file-explorer/src/app.rs` | Respect `ROOT_PATH` / `ALLOWED_PATHS` |
| `backend/src/infrastructure/driving/webrtc.rs` | JWT auth before WS upgrade; input rate limit |
