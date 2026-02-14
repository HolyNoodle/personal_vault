# User Personas & Authorization Model

## Overview

The Secure Sandbox Server supports three distinct user personas with different capabilities and workflows. This document defines each persona, their permissions, and interaction patterns.

---

## Persona Definitions

### 1. Super Admin

**Role:** System administrator with root-level access.

**Purpose:** System management, configuration, monitoring, and emergency intervention.

**Capabilities:**
- ✅ **Full system access** - All operations permitted
- ✅ **User management** - Create, modify, delete any user account
- ✅ **Access override** - Grant/revoke any permission
- ✅ **Session control** - View, terminate any session
- ✅ **Configuration** - Modify system settings, security policies
- ✅ **Audit access** - View all audit logs, export for compliance
- ✅ **Resource management** - Adjust cgroup limits, quotas
- ✅ **Emergency actions** - Force shutdown, disable features

**Security Constraints:**
⚠️ **Critical:** Super Admin accounts are HIGH RISK.

**MANDATORY CONTROLS:**
- Maximum 2-3 super admin accounts per deployment
- **WebAuthn hardware security key REQUIRED** (no magic links allowed)
- Minimum 2 registered FIDO2 devices per admin (backup key mandatory)
- All actions logged with enhanced detail
- Separate from regular operations (break-glass access)
- Regular access reviews (quarterly minimum)
- Cannot be created via API (manual database operation only)
- Activity alerts sent to security team in real-time
- Attestation verification enabled (cryptographic device validation)

**Use Cases:**
- Initial system setup
- Creating first users
- Investigating security incidents
- System maintenance and upgrades
- Compliance audits
- Emergency lockdowns

**Workflow Example:**
```
1. Super Admin logs in with MFA
2. Views audit logs for security investigation
3. Identifies compromised user account
4. Terminates all sessions for that user
5. Locks account
6. All actions logged and alerted
```

---

### 2. User (Data Owner)

**Role:** Person who owns data and controls access to it.

**Purpose:** Manage files and share them securely with authorized clients.

**Capabilities:**
- ✅ **Full file management** - Upload, organize, delete files via browser-based File Explorer
- ✅ **Browser mode applications** - Run apps directly in browser with full rights
- ✅ **Download capability** - Download files to local device
- ✅ **Copy/paste** - Full clipboard access
- ✅ **Share management** - Create shares with expiration, access controls
- ✅ **Access control** - Approve/deny client user connection attempts
- ✅ **Permission configuration** - Define what clients can access
- ✅ **Session monitoring** - View active sandboxed sessions accessing their data
- ✅ **Audit logs** - View who accessed their data and when
- ✅ **Revocation** - Revoke client access at any time
- ✅ **Settings** - Configure share settings (watermarking, time limits, read-only)

**Execution Mode:** **Browser Mode**
- Applications (File Explorer, etc.) run directly in owner's browser
- Direct API access to backend for file operations
- File System Access API for local downloads
- Full manipulation capabilities

**Restrictions:**
- ❌ Cannot access other users' data
- ❌ Cannot modify system configuration
- ❌ Cannot view other users' sessions
- ❌ Cannot grant super admin privileges

**Use Cases:**
- Healthcare: Doctor organizes patient records, shares with specialists
- Legal: Lawyer manages case files, shares with clients
- Finance: Accountant organizes tax documents, shares with customers
- Enterprise: Employee manages confidential reports, shares with partners

**Workflow Example:**
```
1. Owner logs in, launches File Explorer (browser mode)
2. Owner uploads new documents via drag & drop
3. Owner organizes files into folders
4. Owner creates a "share" for specific files/folders
5. Owner configures share: time limit (2 hours), watermarking (enabled)
6. Owner generates invitation link for client
7. Client User requests access
8. Owner reviews request (IP, location, purpose) and approves
9. Client User accesses files in sandboxed mode (video stream)
10. Owner monitors active session in real-time
11. Owner revokes access when no longer needed
12. Owner downloads backup of shared files
```

**Domain Model:**
```rust
pub struct User {
    pub id: UserId,
    pub role: UserRole::Owner,
    pub email: EmailAddress,
    pub webauthn_credentials: Vec<WebAuthnCredential>,  // Registered passkeys
    pub magic_link_enabled: bool,                       // Allow email auth?
    pub storage_quota: StorageQuota,
    pub files: Vec<FileReference>,                      // Files they own
    pub shares: Vec<ShareId>,                           // Shares they created
    pub audit_preferences: AuditConfig,                 // Notification settings
    pub browser_sessions: Vec<BrowserSession>,          // Active browser sessions
}

pub struct BrowserSession {
    pub session_id: SessionId,
    pub app_id: AppId,
    pub jwt_token: String,
    pub api_scopes: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}
```

---

### 3. Client User (Data Consumer)

**Role:** Person who requests and consumes access to another user's data.

**Purpose:** View sensitive documents securely without ability to download or exfiltrate data.

**Capabilities:**
- ✅ **Access request** - Request access to a user's shared data
- ✅ **Sandboxed session** - View files via WebRTC video stream (File Explorer runs server-side)
- ✅ **Input interaction** - Mouse/keyboard to navigate File Explorer
- ✅ **File preview** - View PDFs, images, videos inline (no download)
- ✅ **Navigation** - Browse directory structure
- ✅ **Limited metadata** - View file names, sizes (if permitted)
- ✅ **Own audit log** - View their own access history

**Execution Mode:** **Sandboxed Mode**
- Applications (File Explorer, etc.) run server-side in isolated sandbox
- WebRTC video stream to client browser
- Input events forwarded from browser to sandbox
- Zero data exfiltration - client sees only video pixels

**Restrictions:**
- ❌ **No downloads** - Cannot download files to local device (sees only video)
- ❌ **No copy/paste** - Clipboard disabled
- ❌ **No screenshots** - Watermarking enabled (optional per owner)
- ❌ **No local file access** - Files stay server-side
- ❌ **No network access** - Sandbox has no internet connectivity
- ❌ **No data ownership** - Cannot upload files
- ❌ **No sharing** - Cannot create shares or grant access
- ❌ **No persistence** - Session data deleted on termination
- ❌ **Time-limited** - Access expires per owner's configuration
- ❌ **Cannot view other users' data** (unless separately granted)

**Use Cases:**
- Specialist views patient records shared by doctor (cannot download HIPAA data)
- Client reviews contract shared by lawyer (cannot copy confidential clauses)
- Partner accesses financial data shared by accountant (cannot export sensitive reports)
- Contractor views project files shared by employee (cannot save proprietary documents)

**Workflow Example:**
```
1. Client User receives invitation link from Owner
2. Client User clicks link, sees login/registration page
3. Client User registers or logs in
4. System shows access request form
5. Client User provides purpose, identity verification
6. Client User submits request
7. Client User waits for Owner approval
8. Upon approval, system creates isolated sandbox server-side
9. File Explorer app launches in sandbox (Xvfb virtual display)
10. FFmpeg captures display, encodes to H.264/VP8 video
11. Client User establishes WebRTC connection
12. Client User sees File Explorer in video stream
13. Client User interacts via mouse/keyboard (input forwarding)
14. Client User navigates files, previews PDFs/images/videos
15. All interactions logged to audit trail
16. Session automatically terminates after time limit
17. Sandbox destroyed, no trace of session remains
```

**Domain Model:**
```rust
pub struct ClientUser {
    pub id: UserId,
    pub role: UserRole::Client,
    pub email: EmailAddress,
    pub verified: bool,              // Identity verification status
    pub active_sessions: Vec<SandboxSession>,
}

pub struct SandboxSession {
    pub session_id: SessionId,
    pub app_id: AppId,
    pub sandbox_id: String,
    pub webrtc_connection: WebRTCPeerConnection,
    pub video_stream: VideoStream,
    pub allowed_paths: Vec<String>,  // Landlock-restricted paths
    pub constraints: SandboxConstraints,
    pub started_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
    pub access_requests: Vec<AccessRequestId>,
    pub granted_permissions: Vec<PermissionId>,
    pub session_history: Vec<SessionId>,
}
```

---

## Role Hierarchy

```
┌─────────────────────────────────────┐
│           Super Admin               │
│         (System Root)               │
│  - Full system access               │
│  - User management                  │
│  - Override any permission          │
└─────────────────────────────────────┘
                 │
        ┌────────┴────────┐
        │                 │
┌───────▼──────┐   ┌──────▼────────┐
│     User     │   │  Client User  │
│ (Data Owner) │   │ (Data Consumer)│
│              │   │                │
│ - Owns data  │   │ - Requests     │
│ - Creates    │   │   access       │
│   shares     │   │ - Views data   │
│ - Approves   │   │   via stream   │
│   access     │   │ - Time-limited │
└──────────────┘   └────────────────┘
```

**Authorization Rules:**
- Super Admin > User > Client User (privilege hierarchy)
- Users cannot affect other Users (peer isolation)
- Client Users can only access data explicitly granted by Users

---

## Domain Model Updates

### UserRole Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    /// System administrator with full access
    SuperAdmin,
    
    /// Data owner who shares files
    Owner,
    
    /// Data consumer who requests access
    Client,
}

impl UserRole {
    pub fn can_administrate_system(&self) -> bool {
        matches!(self, UserRole::SuperAdmin)
    }
    
    pub fn can_own_data(&self) -> bool {
        matches!(self, UserRole::Owner | UserRole::SuperAdmin)
    }
    
    pub fn can_create_shares(&self) -> bool {
        matches!(self, UserRole::Owner | UserRole::SuperAdmin)
    }
    
    pub fn can_approve_access_requests(&self) -> bool {
        matches!(self, UserRole::Owner | UserRole::SuperAdmin)
    }
}
```

### Share Aggregate (NEW)

```rust
/// Represents a shareable collection of files with access controls
pub struct Share {
    id: ShareId,
    owner_id: UserId,               // User who created the share
    title: ShareTitle,
    description: Option<String>,
    resources: Vec<ResourcePath>,   // Files/folders included
    access_policy: AccessPolicy,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    revoked: bool,
}

pub struct AccessPolicy {
    pub max_session_duration: Duration,        // e.g., 2 hours
    pub require_approval: bool,                // Manual vs auto-approve
    pub allowed_client_emails: Option<Vec<EmailAddress>>,  // Whitelist
    pub watermark_enabled: bool,
    pub allow_metadata_view: bool,             // Can see file names?
    pub max_concurrent_sessions: u32,          // e.g., only 1 at a time
}

impl Share {
    /// Invariant: Only owner can modify share
    pub fn update_access_policy(
        &mut self,
        requesting_user_id: &UserId,
        new_policy: AccessPolicy,
    ) -> Result<(), DomainError> {
        if requesting_user_id != &self.owner_id {
            return Err(DomainError::Unauthorized);
        }
        
        self.access_policy = new_policy;
        Ok(())
    }
    
    /// Invariant: Cannot unrevoke
    pub fn revoke(&mut self) -> Result<(), DomainError> {
        if self.revoked {
            return Err(DomainError::AlreadyRevoked);
        }
        
        self.revoked = true;
        Ok(())
    }
    
    /// Check if share is currently valid
    pub fn is_valid(&self) -> bool {
        !self.revoked
            && self.expires_at.map_or(true, |exp| Utc::now() < exp)
    }
}
```

### AccessRequest Aggregate (NEW)

```rust
/// Represents a client user's request to access a share
pub struct AccessRequest {
    id: AccessRequestId,
    share_id: ShareId,
    requesting_user_id: UserId,     // Client User
    requested_at: DateTime<Utc>,
    status: AccessRequestStatus,
    purpose: String,                 // Why requesting access
    reviewed_at: Option<DateTime<Utc>>,
    reviewed_by: Option<UserId>,     // User who approved/denied
    denial_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessRequestStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

impl AccessRequest {
    /// Invariant: Only share owner can approve
    pub fn approve(
        &mut self,
        reviewer_id: &UserId,
        share_owner_id: &UserId,
    ) -> Result<Permission, DomainError> {
        if reviewer_id != share_owner_id {
            return Err(DomainError::Unauthorized);
        }
        
        if self.status != AccessRequestStatus::Pending {
            return Err(DomainError::InvalidState(
                "Can only approve pending requests".to_string()
            ));
        }
        
        self.status = AccessRequestStatus::Approved;
        self.reviewed_at = Some(Utc::now());
        self.reviewed_by = Some(*reviewer_id);
        
        // Create permission for client user
        Ok(Permission {
            id: PermissionId::generate(),
            user_id: self.requesting_user_id.clone(),
            share_id: self.share_id.clone(),
            granted_at: Utc::now(),
            expires_at: None, // Set based on access policy
            revoked: false,
        })
    }
    
    /// Invariant: Only share owner can deny
    pub fn deny(
        &mut self,
        reviewer_id: &UserId,
        share_owner_id: &UserId,
        reason: String,
    ) -> Result<(), DomainError> {
        if reviewer_id != share_owner_id {
            return Err(DomainError::Unauthorized);
        }
        
        if self.status != AccessRequestStatus::Pending {
            return Err(DomainError::InvalidState(
                "Can only deny pending requests".to_string()
            ));
        }
        
        self.status = AccessRequestStatus::Denied;
        self.reviewed_at = Some(Utc::now());
        self.reviewed_by = Some(*reviewer_id);
        self.denial_reason = Some(reason);
        
        Ok(())
    }
}
```

### Updated Permission Aggregate

```rust
/// Links a client user to a share they can access
pub struct Permission {
    pub id: PermissionId,
    pub user_id: UserId,           // Client User who has access
    pub share_id: ShareId,         // Share they can access
    pub granted_at: DateTime<Utc>,
    pub granted_by: UserId,        // User (owner) who granted access
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked: bool,
}

impl Permission {
    /// Check if permission is currently valid
    pub fn is_valid(&self) -> bool {
        !self.revoked
            && self.expires_at.map_or(true, |exp| Utc::now() < exp)
    }
    
    /// Invariant: Only share owner or super admin can revoke
    pub fn revoke(
        &mut self,
        revoking_user_id: &UserId,
        revoking_user_role: UserRole,
        share_owner_id: &UserId,
    ) -> Result<(), DomainError> {
        if revoking_user_role != UserRole::SuperAdmin
            && revoking_user_id != share_owner_id
        {
            return Err(DomainError::Unauthorized);
        }
        
        self.revoked = true;
        Ok(())
    }
}
```

---

## Authorization Workflows

### 1. User Shares Data with Client User

```
┌─────────┐                                    ┌─────────────┐
│  User   │                                    │ Client User │
│ (Owner) │                                    │  (Consumer) │
└────┬────┘                                    └──────┬──────┘
     │                                                 │
     │ 1. Upload files                                │
     ├──────────────────────────────────────────┐     │
     │                                          │     │
     │ 2. Create Share                          │     │
     │    - Select files                        │     │
     │    - Set access policy                   │     │
     │    - Generate invitation link            │     │
     ├──────────────────────────────────────────┤     │
     │                                          │     │
     │ 3. Send invitation ─────────────────────────────►
     │                                          │     │
     │                                          │     │ 4. Click link
     │                                          │     ├────────────┐
     │                                          │     │ 5. Register│
     │                                          │     │  - Email   │
     │                                          │     │  - WebAuthn│
     │                                          │     │    passkey │
     │                                          │     ├────────────┘
     │                                          │     │
     │                                          │     │ 6. Submit AccessRequest
     │                                          │     │    - Purpose
     │ 7. Receive notification ◄─────────────────────┤    - Identity
     ├────────────────┐                        │     │
     │ 8. Review:     │                        │     │
     │    - Who?      │                        │     │
     │    - Why?      │                        │     │
     │    - IP/Loc    │                        │     │
     ├────────────────┘                        │     │
     │                                          │     │
     │ 9. Approve request                       │     │
     │    → Permission created ──────────────────────►
     │                                          │     │
     │                                          │     │ 10. Start Session
     │                                          │     ├────────────────┐
     │                                          │     │ 11. View files │
     │ 12. Monitor active session               │     │     via video  │
     │     (User can see client is connected)   │     ├────────────────┘
     │                                          │     │
     │ 13. Revoke access (optional)             │     │
     │     → Session terminated ─────────────────────► X
     │                                          │     │
```

### 2. Super Admin Emergency Intervention

```
┌──────────────┐                    ┌──────────┐
│  Super Admin │                    │  System  │
└──────┬───────┘                    └────┬─────┘
       │                                  │
       │ 1. MFA Login                     │
       ├──────────────────────────────────►
       │                                  │
       │ 2. View audit logs               │
       │    → Suspicious activity detected│
       ├──────────────────────────────────►
       │                                  │
       │ 3. List active sessions          │
       │    → Compromised user found      │
       ├──────────────────────────────────►
       │                                  │
       │ 4. Terminate all user sessions   │
       ├──────────────────────────────────►
       │                                  │
       │ 5. Lock user account              │
       ├──────────────────────────────────►
       │                                  │
       │ 6. Export forensic logs          │
       ├──────────────────────────────────►
       │                                  │
       │    All actions logged + alerted  │
       │                                  │
```

---

## New Commands

### CreateShareCommand

```rust
pub struct CreateShareCommand {
    pub owner_id: UserId,              // Must have Owner or SuperAdmin role
    pub title: ShareTitle,
    pub description: Option<String>,
    pub resources: Vec<ResourcePath>,  // Files to share
    pub access_policy: AccessPolicy,
    pub expires_at: Option<DateTime<Utc>>,
}
```

### RequestAccessCommand

```rust
pub struct RequestAccessCommand {
    pub share_id: ShareId,
    pub requesting_user_id: UserId,    // Must have Client role
    pub purpose: String,               // Why requesting access
}
```

### ApproveAccessRequestCommand

```rust
pub struct ApproveAccessRequestCommand {
    pub request_id: AccessRequestId,
    pub reviewer_id: UserId,           // Must be share owner or SuperAdmin
}
```

### DenyAccessRequestCommand

```rust
pub struct DenyAccessRequestCommand {
    pub request_id: AccessRequestId,
    pub reviewer_id: UserId,           // Must be share owner or SuperAdmin
    pub reason: String,
}
```

### RevokeShareCommand

```rust
pub struct RevokeShareCommand {
    pub share_id: ShareId,
    pub revoking_user_id: UserId,      // Must be owner or SuperAdmin
}
```

---

## New Queries

### ListPendingAccessRequestsQuery

```rust
pub struct ListPendingAccessRequestsQuery {
    pub owner_id: UserId,              // User who owns the shares
    pub pagination: Pagination,
}
```

### GetShareDetailsQuery

```rust
pub struct GetShareDetailsQuery {
    pub share_id: ShareId,
    pub requesting_user_id: UserId,    // Must be owner or have permission
}
```

### ListMySharesQuery

```rust
pub struct ListMySharesQuery {
    pub owner_id: UserId,
    pub include_revoked: bool,
    pub pagination: Pagination,
}
```

### ListMyPermissionsQuery

```rust
pub struct ListMyPermissionsQuery {
    pub client_user_id: UserId,        // Client viewing their access
    pub pagination: Pagination,
}
```

---

## Security Considerations

### Role-Based Access Control (RBAC)

```rust
pub struct AuthorizationService {
    user_repository: Arc<dyn UserRepository>,
}

impl AuthorizationService {
    /// Check if user can perform operation
    pub async fn authorize(
        &self,
        user_id: &UserId,
        operation: Operation,
        resource: Option<ResourceId>,
    ) -> Result<bool, DomainError> {
        let user = self.user_repository.find_by_id(user_id).await?;
        
        match operation {
            Operation::CreateShare => {
                // Only Owners and SuperAdmins can create shares
                Ok(matches!(user.role, UserRole::Owner | UserRole::SuperAdmin))
            }
            
            Operation::ApproveAccessRequest => {
                // Must be share owner or SuperAdmin
                if user.role == UserRole::SuperAdmin {
                    return Ok(true);
                }
                
                if let Some(ResourceId::Share(share_id)) = resource {
                    let share = self.share_repository.find_by_id(&share_id).await?;
                    Ok(share.owner_id == *user_id)
                } else {
                    Ok(false)
                }
            }
            
            Operation::RequestAccess => {
                // Any authenticated user (typically Client role)
                Ok(true)
            }
            
            Operation::TerminateAnySession => {
                // Only SuperAdmin
                Ok(user.role == UserRole::SuperAdmin)
            }
            
            Operation::ViewAuditLogs => {
                // SuperAdmin: all logs
                // Owner: only their logs
                // Client: only their own access logs
                Ok(true)  // Filtering done in query
            }
            
            // ... other operations
        }
    }
}
```

### Privilege Escalation Prevention

**CRITICAL RULES:**

1. ❌ **Cannot self-promote** - Users cannot change their own role
2. ❌ **Cannot create SuperAdmin via API** - Manual database operation only
3. ❌ **Owner cannot access other owner's data** - Peer isolation enforced
4. ❌ **Client cannot grant permissions** - Only receive, not give
5. ✅ **All role changes logged** - Audit trail with reviewer

---

## Database Schema

### users table

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    role VARCHAR(20) NOT NULL CHECK (role IN ('super_admin', 'owner', 'client')),
    magic_link_enabled BOOLEAN NOT NULL DEFAULT TRUE,  -- Allow email auth?
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    locked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    
    -- Owner-specific
    storage_quota_bytes BIGINT,
    storage_used_bytes BIGINT DEFAULT 0,
    
    -- Security
    failed_auth_attempts INT DEFAULT 0,
    locked_until TIMESTAMPTZ
);

CREATE TABLE webauthn_credentials (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id BYTEA UNIQUE NOT NULL,          -- FIDO2 credential ID
    public_key BYTEA NOT NULL,                    -- COSE public key
    counter BIGINT NOT NULL DEFAULT 0,            -- Signature counter (replay protection)
    aaguid BYTEA,                                 -- Authenticator AAGUID
    transports TEXT[],                            -- usb, nfc, ble, internal
    backup_eligible BOOLEAN NOT NULL DEFAULT FALSE,
    backup_state BOOLEAN NOT NULL DEFAULT FALSE,
    attestation_format VARCHAR(50),               -- packed, fido-u2f, etc.
    device_name VARCHAR(255),                     -- User-friendly name
    created_at TIMESTAMPTZ NOT NULL,
    last_used_at TIMESTAMPTZ
);

CREATE TABLE magic_link_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT UNIQUE NOT NULL,              -- SHA-256 of token
    expires_at TIMESTAMPTZ NOT NULL,              -- 15 minutes from creation
    used BOOLEAN NOT NULL DEFAULT FALSE,
    used_at TIMESTAMPTZ,
    ip_address INET,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_role ON users(role);
```

### shares table

```sql
CREATE TABLE shares (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL REFERENCES users(id),
    title VARCHAR(255) NOT NULL,
    description TEXT,
    access_policy JSONB NOT NULL,  -- Serialized AccessPolicy
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ,
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    revoked_at TIMESTAMPTZ
);

CREATE INDEX idx_shares_owner_id ON shares(owner_id);
CREATE INDEX idx_shares_revoked ON shares(revoked) WHERE revoked = FALSE;
```

### access_requests table

```sql
CREATE TABLE access_requests (
    id UUID PRIMARY KEY,
    share_id UUID NOT NULL REFERENCES shares(id),
    requesting_user_id UUID NOT NULL REFERENCES users(id),
    purpose TEXT NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('pending', 'approved', 'denied', 'expired')),
    requested_at TIMESTAMPTZ NOT NULL,
    reviewed_at TIMESTAMPTZ,
    reviewed_by UUID REFERENCES users(id),
    denial_reason TEXT
);

CREATE INDEX idx_access_requests_share_id ON access_requests(share_id);
CREATE INDEX idx_access_requests_requesting_user ON access_requests(requesting_user_id);
CREATE INDEX idx_access_requests_status ON access_requests(status);
```

### permissions table

```sql
CREATE TABLE permissions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    share_id UUID NOT NULL REFERENCES shares(id),
    granted_at TIMESTAMPTZ NOT NULL,
    granted_by UUID NOT NULL REFERENCES users(id),
    expires_at TIMESTAMPTZ,
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    revoked_at TIMESTAMPTZ,
    
    UNIQUE(user_id, share_id, revoked)  -- One active permission per user/share
);

CREATE INDEX idx_permissions_user_id ON permissions(user_id);
CREATE INDEX idx_permissions_share_id ON permissions(share_id);
CREATE INDEX idx_permissions_valid ON permissions(revoked, expires_at) WHERE revoked = FALSE;
```

---

## API Endpoints

### User (Owner) Endpoints

```
POST   /api/shares                    # Create share
GET    /api/shares                    # List my shares
GET    /api/shares/:id                # Get share details
PATCH  /api/shares/:id                # Update access policy
DELETE /api/shares/:id                # Revoke share

GET    /api/access-requests/pending   # List pending requests for my shares
POST   /api/access-requests/:id/approve  # Approve request
POST   /api/access-requests/:id/deny     # Deny request

GET    /api/sessions                  # List sessions accessing my data
DELETE /api/sessions/:id              # Terminate specific session
```

### Client User Endpoints

```
POST   /api/access-requests           # Request access to share
GET    /api/access-requests           # List my access requests
GET    /api/permissions                # List shares I can access
GET    /api/sessions                  # List my active sessions
```

### Authentication Endpoints (All Users)

```
# WebAuthn Registration
POST   /api/auth/webauthn/register/begin    # Start passkey registration
POST   /api/auth/webauthn/register/finish  # Complete passkey registration

# WebAuthn Authentication
POST   /api/auth/webauthn/login/begin       # Start passkey login
POST   /api/auth/webauthn/login/finish     # Complete passkey login

# Magic Link (if enabled for user)
POST   /api/auth/magic-link/request         # Request magic link email
GET    /api/auth/magic-link/verify/:token  # Verify and login

# Session Management
POST   /api/auth/refresh                    # Refresh access token
POST   /api/auth/logout                     # Invalidate session
GET    /api/auth/devices                    # List registered devices
DELETE /api/auth/devices/:id                # Revoke device
```

### Super Admin Endpoints

```
GET    /api/admin/users               # List all users
PATCH  /api/admin/users/:id/lock      # Lock user account
DELETE /api/admin/sessions/:id        # Terminate any session
GET    /api/admin/audit-logs          # View all audit logs
POST   /api/admin/alerts              # Configure security alerts
```

---

## Compliance & Audit

### Logged Events (Per Persona)

**Super Admin:**
- All actions logged with `actor_role: super_admin`
- Enhanced metadata (IP, user agent, MFA status)
- Real-time alerts to security team
- Quarterly access reviews mandatory

**User (Owner):**
- Share creation, modification, revocation
- Access request approvals/denials
- File uploads and deletions
- Session monitoring
- Permission grants/revocations

**Client User:**
- Access requests submitted
- Session starts/ends
- File views (which files accessed)
- Input actions (if enabled in audit policy)

---

## Migration Path

For existing deployments without personas:

1. Add `role` column to `users` table (default: `owner`)
2. Prompt for SuperAdmin creation on first startup
3. Create backward-compatible API (sessions without shares)
4. Deprecate old flow over 6 months

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related Documents:** [DOMAIN_OBJECTS.md](DOMAIN_OBJECTS.md), [COMMANDS.md](COMMANDS.md), [QUERIES.md](QUERIES.md), [SECURITY.md](SECURITY.md)
