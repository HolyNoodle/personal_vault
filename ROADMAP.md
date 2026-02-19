# Roadmap: Secure Sandbox Platform — End-to-End Security Implementation

## How to Read This Document

- `[x]` = implemented and working
- `[ ]` = not yet implemented
- Each phase builds on the previous. Phases are ordered by dependency.
- This is a living document — check boxes as work completes.


**Database Layer Note:**

- The backend uses the **Diesel query builder** (not the ORM features) for all database queries, targeting a **SQLite** database.
- Database migrations use Diesel's **up/down migration system**: each migration is a folder containing `up.sql` and `down.sql` scripts, located in `backend/migrations/`.
- All schema changes and new tables must be implemented as Diesel migrations, following this folder structure.
## Role Model

Roles are a **set on each user**, not a single enum value. A user can hold multiple roles simultaneously:

```
user.roles: HashSet<UserRole>   // e.g. {SuperAdmin, Owner}
```

| Role | Capabilities |
|------|-------------|
| `Client` | Use apps within paths an Owner has granted them |
| `Owner` | All Client capabilities + own a storage root, upload files, invite clients, manage permissions |
| `SuperAdmin` | All Owner capabilities + manage other users, create Owner accounts |

When the first SuperAdmin registers during initial setup, they receive **both** `SuperAdmin` and `Owner` roles and get a storage directory. A later SuperAdmin created by an existing SuperAdmin also gets both roles.

**Role checks must test set membership, not equality:**
```rust
user.has_role(UserRole::Owner)        // true for [Owner] and [SuperAdmin, Owner]
user.has_role(UserRole::SuperAdmin)   // true only for [SuperAdmin, Owner]
```

**What this changes from the current code:**
- `User.role: UserRole` → `User.roles: Vec<UserRole>` (domain entity)
- DB column `role user_role NOT NULL` → `roles user_role[] NOT NULL`
- All route guards use `has_role()` not `== role`
- `complete_webauthn_registration` assigns `[SuperAdmin, Owner]` not just `[SuperAdmin]`
- JWT claims carry `roles: Vec<String>` not `role: String`

**DB reset strategy:** The current schema already supports multi-role as a JSON array in a TEXT column (`roles TEXT NOT NULL DEFAULT '[]'`). No database reset or migration rewrite is required for Phase 1. You can proceed with further development without resetting the database.

---

## Current State (Baseline)

**What exists and works:**
- [x] Domain: `User` entity with `role: UserRole` (single — to be replaced)
- [x] Domain: `UserRole` enum (SuperAdmin / Owner / Client), `UserStatus`
- [x] Domain: `Credential` entity (WebAuthn passkey)
- [x] DB: `users` + `webauthn_credentials` tables (PoC migrations — **will be wiped and rewritten**)
- [x] Application: SuperAdmin WebAuthn registration flow (initiate + complete)
- [x] Application: WebAuthn login flow — challenge/response works, **credential verification disabled**
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
- [x] Route: `GET /api/setup/status` — checks `count_super_admins() == 0`

**What is absent:**
- Multi-role support in domain + DB
- Per-user storage directories
- Owner/Client creation flows
- Invitation / sharing model (DB tables, domain, API)
- Session table and session-role context
- File management API
- Sandbox security enforcement (Landlock, namespaces, seccomp, cgroups)
- Real-time permission push (session restart on revoke)
- Audit logging
- WebRTC / transport hardening

---

## Phase 0 — Initial Application Setup (First Run)

On first boot with an empty database, the application is in **uninitialized** state. The frontend detects this and shows a setup screen. The setup flow creates the first SuperAdmin, who is also an Owner.

### 0.1 Setup status endpoint (already exists — verify correct)
**File:** `backend/src/infrastructure/driving/http/check_setup_status.rs`

- [x] `GET /api/setup/status` returns `{ initialized: bool }` based on `count_super_admins() > 0`
- [x] Verify: while `initialized = false`, ALL routes except `/api/setup/*` and `/health` return `503 Service Unavailable` (app is not usable until a SuperAdmin exists)

### 0.2 First SuperAdmin registration
**File:** `backend/src/application/super_admin/commands/complete_webauthn_registration.rs`

- [x] `POST /api/setup/initiate-registration` — generates WebAuthn challenge
- [x] `POST /api/setup/complete-registration` — saves user + passkey
- [x] **Fix**: assign `roles = [SuperAdmin, Owner]` instead of just `[SuperAdmin]`
- [x] **Fix**: after saving user, create owner storage directory `$STORAGE_PATH/{user_id}/` (Phase 2)
- [x] Guard: return `409 Conflict` if a SuperAdmin already exists (setup can only run once)

### 0.3 Lock setup endpoints after initialization
**File:** `backend/src/infrastructure/driving/http/auth.rs` or middleware

- [x] If `count_super_admins() > 0`, `/api/setup/initiate-registration` and `/api/setup/complete-registration` return `403 Forbidden`

---

## Phase 1 — Authentication Completion & Role Model Fix

Fix existing auth plumbing and migrate the single-role model to multi-role.

### 1.1 Multi-role domain + DB reset
**File:** `backend/src/domain/entities/user.rs`
**File:** `backend/src/domain/value_objects/user_role.rs`
**Action:**

- [x] The DB schema already uses a `roles` field as a JSON array in a TEXT column; no DB reset or migration rewrite is needed.
- [x] `User.role: UserRole` has been changed to `User.roles: Vec<UserRole>`
- [x] `User::has_role(&self, role: UserRole) -> bool` exists
- [x] `UserRepository` reads/writes the `roles` array as JSON
- [x] `complete_webauthn_registration` writes `roles = [SuperAdmin, Owner]`
- [x] JWT claims use `roles: Vec<String>`
- [x] `AuthenticatedUser` extension struct uses `roles: Vec<UserRole>`

### 1.2 Login credential verification
**File:** `backend/src/application/super_admin/commands/complete_webauthn_login.rs`

- [x] Re-enable credential lookup by user id (`credential_repo.find_by_user_id`)
- [x] Call `webauthn.finish_passkey_authentication()` with the stored passkey
- [x] Update `sign_count` on the stored credential after successful auth
- [x] Return `403` on failed verification (not a silent pass)
- [x] JWT now carries `roles: Vec<String>`

### 1.3 JWT middleware
**New file:** `backend/src/infrastructure/driving/http/middleware/auth.rs`

- [x] Extract `Authorization: Bearer <token>` from requests
- [x] Validate JWT signature (`JWT_SECRET` env var)
- [x] Reject expired tokens
- [x] Inject `AuthenticatedUser { id, email, roles: Vec<UserRole> }` as Axum extension
- [x] Apply to all routes except `/api/setup/*`, `/api/auth/*`, `/health` (enforced per-handler via `AuthenticatedUser` extractor)

### 1.4 SuperAdmin: invite a new Owner (or another SuperAdmin)
**New:** `backend/src/application/super_admin/commands/invite_user.rs`
**Route:** `POST /api/admin/invite` (SuperAdmin role required)

- [ ] Validate caller `has_role(SuperAdmin)`
- [ ] Accept `{ email, display_name, roles: [Owner] | [SuperAdmin, Owner] }`
- [ ] Create `User` with the given roles + `UserStatus::Active`
- [ ] If roles includes `Owner`: create storage directory `$STORAGE_PATH/{user_id}/`
- [ ] Generate signed invite token; return invite link
- [ ] Invited user follows Owner registration flow (1.5)

### 1.5 Owner / SuperAdmin WebAuthn registration via invite link
**New routes:** `POST /api/invite/{token}/initiate`, `POST /api/invite/{token}/complete`

- [ ] Validate token (signed, not expired, not already used)
- [ ] On complete: save passkey for the pre-created user
- [ ] Mark user as fully registered; return JWT

### 1.6 Client user creation (deferred to Phase 3)
Clients are created implicitly when they accept an invitation from an Owner.

---

## Phase 2 — Per-User Storage

Every user with the `Owner` role (including SuperAdmins) gets an isolated directory. Clients never get their own storage root.

### 2.1 Storage root creation on user creation
**Triggered from:** Phase 0.2 (first SuperAdmin) and Phase 1.4 (invite with Owner role)
**Env var:** `STORAGE_PATH` (e.g. `/data/storage`)

- [x] Helper: `create_owner_storage(user_id: &UserId) -> Result<PathBuf>`
  - `mkdir -p $STORAGE_PATH/{user_id}/`
  - Storage root is always derived as `$STORAGE_PATH/{user_id}` — no DB column needed
- [x] Called in every code path that creates a user with `Owner` role

### 2.2 File Explorer app: scoped root
**File:** `apps/file-explorer/src/app.rs`
**New env var:** `ROOT_PATH`

- [x] Read `ROOT_PATH` from environment; default `/` only if unset
- [x] Clamp all navigation to within `ROOT_PATH` (reject `..` above root)
- [x] Display relative path in UI breadcrumb (strip `ROOT_PATH` prefix)

### 2.3 Backend passes `ROOT_PATH` to app
**File:** `backend/src/infrastructure/driven/sandbox/xvfb.rs` → `launch_app()`

- [x] Add `root_path: &str` parameter to `launch_app`
- [x] Set `ROOT_PATH={root_path}` in app env vars
- [x] Owner/SuperAdmin session: `root_path = $STORAGE_PATH/{user_id}`
- [x] Client session: `root_path = $STORAGE_PATH/{owner_id}` (Landlock restricts further — Phase 5)

---

## Phase 3 — Invitation & Permission Model

An Owner (or SuperAdmin acting as Owner) invites a Client and specifies which paths and access levels the client gets.

### 3.1 Database migrations
**New migration:** `create_invitations_and_permissions`

- [x] `invitations` table:
  ```sql
  id UUID PRIMARY KEY,
  owner_id UUID REFERENCES users(id),
  invitee_email VARCHAR(255),
  token VARCHAR(64) UNIQUE NOT NULL,
  granted_paths JSONB NOT NULL,   -- [{path, access: ["read","write","delete"]}]
  status VARCHAR(20) NOT NULL,    -- pending | accepted | revoked | expired
  expires_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
  ```
- [x] `file_permissions` table:
  ```sql
  id UUID PRIMARY KEY,
  owner_id UUID REFERENCES users(id),
  client_id UUID REFERENCES users(id),
  path TEXT NOT NULL,             -- relative to owner's storage root
  access TEXT[] NOT NULL,        -- {read, write, delete}
  granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ,
  revoked_at TIMESTAMPTZ
  ```
- [x] Index: `(client_id, revoked_at)` — fast active-permission lookups per client
- [x] Index: `(owner_id, client_id)` — per-pair queries

> ⚠️ **NOTE**: Migration uses SQLite-compatible types (`TEXT` for UUIDs/timestamps, `JSON` for JSONB) — correct for this backend, but differs from the PostgreSQL notation in the roadmap spec.

### 3.2 Domain entities
- [x] `Invitation`: `backend/src/domain/entities/invitation.rs`
  — fields: id, owner_id, invitee_email, token, granted_paths, status, expires_at
  — methods: `is_valid()`, `mark_accepted()`, `revoke()`
- [x] `FilePermission`: `backend/src/domain/entities/file_permission.rs`
  — fields: id, owner_id, client_id, path, access, granted_at, expires_at, revoked_at
  — methods: `is_active()`, `allows(AccessLevel)`, `revoke()`
- [x] `GrantedPath` value object: `{ path: String, access: Vec<AccessLevel> }`
- [x] `AccessLevel` enum: `Read`, `Write`, `Delete`

### 3.3 Repository ports + implementations
- [x] `InvitationRepository`: `save`, `find_by_token`, `find_by_owner`, `update_status` — exported from `ports/mod.rs`, real SQL in `SqliteInvitationRepository`
- [x] `FilePermissionRepository`: `save`, `find_active_for_client`, `find_active_by_owner`, `find_by_owner_client`, `revoke` — exported, real SQL in `SqliteFilePermissionRepository`
- [x] Both repositories added to `AppState` in `infrastructure/mod.rs` and instantiated in `main.rs`

### 3.4 Owner: create invitation
**New:** `backend/src/application/owner/commands/create_invitation.rs`
**Route:** `POST /api/invitations` — requires `has_role(Owner)`

- [x] Accept `{ invitee_email, granted_paths: [{path, access}], expires_in_hours }`
- [x] Validate paths are relative and reject `../` and absolute paths starting with `/`
- [x] Generate 32-byte cryptographically random token
- [x] Persist invitation; return `{ invitation_id, token, invite_url }` — real SQL, route registered in `main.rs`
- [ ] Optional: send email via SMTP (`SMTP_*` env vars already configured)

### 3.5 Client: view & accept invitation
**Route:** `GET  /api/invitations/{token}` — public (no auth needed)
**Route:** `POST /api/invitations/{token}/accept`

- [x] `GET /api/invitations/{token}`: return `{ owner_id, granted_paths, expires_at }` — public, no sensitive data
- [x] `POST /api/invitations/{token}/accept/initiate`: generate WebAuthn challenge, store in Redis
- [x] `POST /api/invitations/{token}/accept/complete`:
  - [x] Load invitation; verify `is_valid()`
  - [x] If invitee email matches an existing user: link permissions to that user
  - [x] Else: create new `User` with `roles = [Client]`
  - [x] Finish WebAuthn passkey registration, save credential
  - [x] Create `FilePermission` rows for each `granted_path`
  - [x] Mark invitation `status = Accepted`
  - [x] Return JWT for the client

### 3.6 Owner: list/revoke permissions
**Route:** `GET    /api/permissions?client_id=<id>` — requires `has_role(Owner)`
**Route:** `DELETE /api/permissions/{id}` — requires `has_role(Owner)`

- [x] List: all active permissions by owner (optionally filtered by client_id) — real SQL, route registered
- [x] Revoke: sets `revoked_at = NOW()` via real SQL — route registered in `main.rs`
- [ ] Revoke: trigger session restart if client has an active Xvfb session (Phase 6)

### 3.7 Client: list accessible paths
**Route:** `GET /api/my-permissions` — requires `has_role(Client)`

- [x] Return all non-revoked, non-expired `FilePermission` rows for the calling client — real SQL, route registered in `main.rs`

---

## Phase 4 — Session Model with Role & Permission Context

The app launch currently knows nothing about who is launching or what they're allowed. This phase binds identity and permissions to each session.

### 4.1 Sessions database table
**New migration:** `create_sessions_table`

- [x] `sessions` table:
  ```sql
  id UUID PRIMARY KEY,
  user_id UUID REFERENCES users(id),
  acting_as_owner_id UUID REFERENCES users(id), -- for Client sessions: whose storage root
  active_role TEXT NOT NULL,  -- "owner" or "client" (which role is being exercised this session)
  app_id TEXT NOT NULL,
  display_number SMALLINT,
  state TEXT NOT NULL,         -- initializing | ready | active | terminated
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ NOT NULL,
  terminated_at TIMESTAMPTZ
  ```
  Note: `active_role` distinguishes which role is being exercised. A SuperAdmin always uses `owner` for app sessions.

### 4.2 Session repository
- [x] `SessionRepository` trait: `save`, `find_by_id`, `find_active_by_user`, `update_state`, `terminate`
- [x] SQLite implementation (`SqliteSessionRepository`) — added to `AppState` and `main.rs`

### 4.3 Launch application — auth-aware
**File:** `backend/src/application/client/commands/launch_application.rs`
**Route:** `POST /api/applications/launch` — requires JWT (any role)

- [x] Extract `AuthenticatedUser` from JWT middleware
- [x] If `has_role(Owner)` (includes SuperAdmin):
  - `root_path = $STORAGE_PATH/{user_id}`, full read/write/delete on root
  - `active_role = "owner"`, `acting_as_owner_id = user_id`
- [x] If `has_role(Client)` only:
  - Load active `FilePermission` rows for this client
  - Resolve paths against the owner's storage root
  - `active_role = "client"`, `acting_as_owner_id = owner_id`
- [x] Create `Session` record in DB
- [x] Call `start_xvfb` → `launch_app(root_path, allowed_paths)` → WebRTC calls `start_capture`
- [x] Return `{ session_id, websocket_url }`

### 4.4 Session expiry enforcement
- [x] `expires_at = now + SESSION_TIMEOUT` (env var, default 3600s)
- [x] Background task every 60s: call `cleanup_session` on expired sessions
- [x] WebSocket disconnect sets session `state = terminated`

---

## Phase 5 — Sandbox Security Enforcement

Kernel-level isolation. Each item ships independently and incrementally.

### 5.1 App filesystem scoping (UX layer — defence-in-depth)
- [x] Pass `ALLOWED_PATHS` (colon-separated) env var to app process for Client sessions
- [x] File Explorer hides/disables navigation outside `ALLOWED_PATHS`
- [x] This is UX-only — kernel enforcement is below

### 5.2 Landlock LSM filesystem enforcement
**New file:** `backend/src/infrastructure/driven/sandbox/landlock.rs`
**Dependency:** `landlock` crate

- [x] Add `landlock` crate to `backend/Cargo.toml`
- [x] `apply_landlock(root_path, allowed_paths)` — grants access to data paths + system dirs, calls `restrict_self()`
- [x] In `launch_app` `pre_exec` closure:
  - Owner: full access to `root_path`
  - Client: one rule per `allowed_paths` entry
- [x] System paths (usr, lib, tmp/.X11-unix) added with read-only access

### 5.3 Network namespace (no internet for app)
- [x] In `pre_exec`: `libc::unshare(CLONE_NEWNET)` — new network namespace, no interfaces
- [x] Xvfb stays in host network namespace (needs loopback)

### 5.4 Mount namespace (basic isolation)
- [x] In `pre_exec`: `unshare(CLONE_NEWNS)` — new mount namespace
- [ ] Full bind-mount setup (storage root, X11 socket, system libs) — deferred

### 5.5 seccomp syscall denylist
**New file:** `backend/src/infrastructure/driven/sandbox/seccomp.rs`
**Dependency:** `seccompiler` crate

- [x] Implemented using raw BPF via `libc` (no external crate needed)
- [x] Denylist (returns EPERM): `ptrace`, `kexec_load`, `init_module`, `finit_module`, `delete_module`, `setuid/gid`, `mount`, `umount2`, `pivot_root`, `chroot`, `process_vm_readv/writev`, `perf_event_open`
- [x] Default action: allow all other syscalls
- [x] Apply in `pre_exec` after Landlock

### 5.6 cgroups v2 resource limits
**New file:** `backend/src/infrastructure/driven/sandbox/cgroups.rs`

- [x] Create `/sys/fs/cgroup/sandbox/{session_id}/` on session start
- [x] Write app PID to `cgroup.procs`
- [x] `cpu.max = 500000 1000000` (50%), `memory.max = 512MB`, `pids.max = 100`
- [x] Delete cgroup on session cleanup

---

## Phase 6 — Real-Time Permission Enforcement

### 6.1 Active session lookup by user
**File:** `backend/src/infrastructure/driven/sandbox/xvfb.rs`

- [ ] `find_session_by_user_id(user_id) -> Option<&str>` (session_id)

### 6.2 Permission revocation triggers session restart
**File:** `backend/src/application/owner/commands/revoke_permission.rs`

- [ ] After `revoked_at = NOW()`: check for active session
- [ ] If active: `cleanup_session` → re-launch with narrowed permission set
- [ ] Client reconnects (~2s); new process has new Landlock ruleset

### 6.3 Owner WebSocket channel (live dashboard)
**Route:** `GET /ws/owner` — JWT required before upgrade, `has_role(Owner)`

- [ ] Events: `session_started`, `session_ended`, `file_opened`, `permission_revoked`
- [ ] Source: IPC `AppMessage::State` events from running app + session lifecycle

### 6.4 Client session WebSocket notifications
**File:** existing WS handler

- [ ] Push `{ event: "permission_revoked", path }` on revocation
- [ ] Push `{ event: "session_expiring", seconds_remaining: 300 }` at 5-min warning
- [ ] Push `{ event: "session_terminated" }` on owner kill or expiry

### 6.5 Audit log
**New migration:** `create_audit_log_table`

- [ ] `audit_events`: `id`, `session_id`, `user_id`, `owner_id`, `event_type`, `payload JSONB`, `created_at`
- [ ] Write on: session start/end, file open (IPC), permission grant/revoke, invitation created/accepted
- [ ] `GET /api/audit?client_id=` — requires `has_role(Owner)`

---

## Phase 7 — Transport & WebRTC Hardening

Independent — can run in parallel with any other phase.

### 7.1 WebSocket authentication
- [ ] Extract JWT from `?token=` query param before WS upgrade
- [ ] `401` if missing/invalid; bind WS to `session_id` from claims

### 7.2 TLS / WSS
- [ ] HAProxy TLS termination (config already in `haproxy/`)
- [ ] WebSocket over `wss://`; set `WEBAUTHN_ORIGIN=https://` in production

### 7.3 TURN server hardening
- [ ] No hardcoded defaults for `TURN_USERNAME` / `TURN_CREDENTIAL`
- [ ] `turns://` in production; consider per-session HMAC-SHA1 credentials

### 7.4 Input rate limiting
- [ ] Cap at ~120 events/s per session; drop excess (don't queue)
- [ ] Log excessive rate as audit event

---

## Phase 8 — Management API Surface

Endpoints the frontend needs, built incrementally as each phase lands.

### 8.1 SuperAdmin APIs (requires `has_role(SuperAdmin)`)
- [ ] `GET  /api/admin/users` — list all users with their roles
- [ ] `POST /api/admin/invite` — invite new Owner or SuperAdmin (Phase 1.4)
- [ ] `PUT  /api/admin/users/{id}/status` — suspend / reactivate
- [ ] `DELETE /api/admin/users/{id}` — soft-delete
- [ ] `PUT  /api/admin/users/{id}/roles` — add or remove roles from a user

### 8.2 Owner APIs (requires `has_role(Owner)`)
- [ ] `GET  /api/storage/usage` — bytes used in caller's storage root
- [ ] `GET  /api/clients` — Client users with active permissions from this owner
- [ ] `GET  /api/clients/{client_id}/activity` — audit trail for a specific client
- [ ] `DELETE /api/sessions/{session_id}` — terminate a client's active session
- [ ] All invitation + permission endpoints (Phase 3)

### 8.3 Client APIs (requires `has_role(Client)`)
- [ ] `GET  /api/my-permissions` — active file permissions granted to caller
- [ ] `GET  /api/my-session` — current session info + time remaining
- [ ] `POST /api/applications/launch` — start sandboxed session

---

## Known Implementation Gaps (Phases 1–3)

These must be resolved before any Phase 3 feature is functional end-to-end:

1. **JWT middleware not applied** — `jwt_auth` must be layered onto the router in `main.rs`.
2. **AppState missing repositories** — `invitation_repo` and `file_permission_repo` must be added to `AppState` and instantiated in `main.rs`.
3. **Repository implementations are no-op stubs** — All Diesel queries for `SqliteInvitationRepository` and `SqliteFilePermissionRepository` need to be written.
4. **Module tree incomplete** — `application/mod.rs` missing `pub mod invite;`. `application/ports/mod.rs` missing declarations and re-exports for the two new repository traits.
5. **No Phase 3 routes registered** — None of the 8 new routes are mounted in `main.rs`.
6. **Invite WebAuthn flow is a stub** — Challenge generation, credential saving, permission creation, invitation acceptance, and JWT return all need implementing.
7. **sign_count never updated on login** — `complete_webauthn_login.rs` re-saves credential unchanged.

---

## Phase 9 — Frontend Views

React app located at `frontend/web/src/`. Uses React Router v6, MUI v7, Zustand, Formik/Yup, @simplewebauthn/browser.

### 9.0 Auth store + role-aware routing

**File:** `frontend/web/src/store/authStore.ts`
**File:** `frontend/web/src/App.tsx`

- [ ] Change `role: 'super_admin' | 'owner' | 'client'` → `roles: string[]` in `User` type and auth store
- [ ] Add `hasRole(role: string): boolean` helper (checks array membership)
- [ ] Role-aware `<ProtectedRoute>`: accept a `requiredRole` prop and redirect with 403 UI if not satisfied
- [ ] Update `Layout.tsx` navigation: show/hide nav items based on roles (SuperAdmin section, Owner section, Client section)

### 9.1 SuperAdmin views (requires SuperAdmin role)

**New page:** `frontend/web/src/pages/admin/UsersPage.tsx`
**Route:** `/admin/users`

- [ ] Table of all users: email, display name, roles, status (active / suspended)
- [ ] "Invite Owner / SuperAdmin" button → opens `InviteUserDialog`
- [ ] Per-row actions: suspend/reactivate, edit roles, soft-delete
- [ ] API calls: `GET /api/admin/users`, `POST /api/admin/invite`, `PUT /api/admin/users/{id}/status`, `PUT /api/admin/users/{id}/roles`, `DELETE /api/admin/users/{id}`

**New component:** `frontend/web/src/components/admin/InviteUserDialog.tsx`

- [ ] Form: email, display name, role selection (Owner / SuperAdmin+Owner)
- [ ] On submit: `POST /api/admin/invite` → display generated invite link with copy button

### 9.2 Owner views (requires Owner role)

**New page:** `frontend/web/src/pages/owner/InvitationsPage.tsx`
**Route:** `/owner/invitations`

- [ ] List existing invitations (pending / accepted / revoked) with expiry, granted paths, invitee email
- [ ] "New Invitation" button → opens `CreateInvitationDialog`
- [ ] Revoke button per invitation
- [ ] API calls: `GET /api/invitations` (owner's), `POST /api/invitations`, `DELETE /api/invitations/{id}`

**New component:** `frontend/web/src/components/owner/CreateInvitationDialog.tsx`

- [ ] Form: invitee email, path list (add/remove rows), access level per path (Read / Write / Delete checkboxes), expiry in hours
- [ ] Path inputs validated: no `..`, relative paths only
- [ ] On success: show invite link/token with copy button

**New page:** `frontend/web/src/pages/owner/PermissionsPage.tsx`
**Route:** `/owner/permissions`

- [ ] Filter by client (dropdown of clients with active permissions)
- [ ] Table: path, access levels, granted date, expiry, revoke button
- [ ] API calls: `GET /api/permissions?client_id=`, `DELETE /api/permissions/{id}`

**New page:** `frontend/web/src/pages/owner/ClientsPage.tsx`
**Route:** `/owner/clients`

- [ ] List of client users who have active permissions granted by this owner
- [ ] Click client → show their active permissions and recent activity
- [ ] Terminate active session button
- [ ] API calls: `GET /api/clients`, `GET /api/clients/{id}/activity`, `DELETE /api/sessions/{sessionId}`

### 9.3 Client views (requires Client role)

**New page:** `frontend/web/src/pages/client/MyPermissionsPage.tsx`
**Route:** `/my-permissions`

- [ ] List all active file permissions: path, access levels, owner name, expiry
- [ ] Visual indicator for expiring-soon permissions
- [ ] API call: `GET /api/my-permissions`

**Update:** `frontend/web/src/pages/LaunchApplicationPage.tsx`

- [ ] Replace hardcoded role/path options with data from `GET /api/my-permissions`
- [ ] Owner sessions auto-populate with owner's storage root (no selection needed)
- [ ] Client sessions show only their granted paths as selectable scope

### 9.4 Invitation acceptance flow (public — no auth required)

**New page:** `frontend/web/src/pages/InvitePage.tsx`
**Route:** `/invite/:token`

- [ ] On load: `GET /api/invitations/{token}` → show owner display name, list of granted paths + access levels, expiry date
- [ ] Show 404/expired UI if invitation is invalid
- [ ] "Accept Invitation" button → triggers WebAuthn registration flow (same pattern as SetupPage)
  - `POST /api/invite/{token}/initiate` → get WebAuthn challenge
  - Collect credential via `@simplewebauthn/browser`
  - `POST /api/invite/{token}/complete` → receive JWT + user info
- [ ] On success: store auth token → redirect to `/my-permissions`

### 9.5 Owner/SuperAdmin session monitoring

**Update:** `frontend/web/src/pages/SessionsPage.tsx` (currently placeholder)

- [ ] List active sessions: user email, role, app, start time, expiry countdown
- [ ] Terminate session button (Owner: only their clients' sessions; SuperAdmin: any)
- [ ] API calls: `GET /api/sessions` (scoped by role), `DELETE /api/sessions/{sessionId}`

### 9.6 File management

**Wire existing page:** `frontend/web/src/pages/FilesPage.tsx`
**Route:** `/files`

- [ ] Add `/files` route to `App.tsx` (page is implemented but not routed)
- [ ] Scope displayed root path to the authenticated user's storage root (Owner) or granted paths (Client)
- [ ] Wire download button to actual file download endpoint
- [ ] Wire create folder button
- [ ] API calls: `GET /api/files?path=`, `POST /api/files/upload`, `POST /api/files/mkdir`

### 9.7 Navigation updates

**File:** `frontend/web/src/components/Layout.tsx`

- [ ] SuperAdmin nav section: "Users" (`/admin/users`)
- [ ] Owner nav section: "Invitations" (`/owner/invitations`), "Permissions" (`/owner/permissions`), "Clients" (`/owner/clients`)
- [ ] Client nav section: "My Access" (`/my-permissions`)
- [ ] Common: "Files" (`/files`), "Sessions" (`/sessions`)
- [ ] Sections hidden when user lacks the required role

---

## Dependency Order

```
Phase 0 (Initial setup — first SuperAdmin)
    → Phase 1 (Multi-role model + auth fix)
        → Phase 2 (Storage — needs Owner role to exist properly)
            → Phase 3 (Invitations — needs storage + Owner)
                → Phase 4 (Session context — needs permissions)
                    → Phase 5 (Kernel sandbox — needs session context)
                        → Phase 6 (Real-time enforcement — needs Landlock)
Phase 7 (Transport) — independent, any time
Phase 8 (APIs)      — incremental, follows each phase
```

---

## Critical Files

| File | Change |
|------|--------|
| `backend/migrations/` | **Delete all**; replace with single `001_initial_schema.sql` |
| `backend/src/domain/entities/user.rs` | `role: UserRole` → `roles: Vec<UserRole>`, add `has_role()` |
| `backend/src/application/super_admin/commands/complete_webauthn_registration.rs` | Assign `[SuperAdmin, Owner]`, create storage dir |
| `backend/src/application/super_admin/commands/complete_webauthn_login.rs` | Re-enable credential verification |
| `backend/src/infrastructure/driving/http/middleware/auth.rs` | **New** — JWT middleware, `has_role()` guards |
| `backend/src/application/super_admin/commands/invite_user.rs` | **New** — create user with given roles |
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
| **Diesel Query Builder** | All DB queries use Diesel's query builder (not ORM) with SQLite |
| **Migrations** | Diesel up/down migration folders in `backend/migrations/` |
| `backend/src/infrastructure/driving/http/middleware/auth.rs` | **New** — JWT middleware, `has_role()` guards |
