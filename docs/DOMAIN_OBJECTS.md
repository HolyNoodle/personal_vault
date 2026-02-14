# Domain Objects Documentation

## Overview

This document describes all domain objects in the Secure Sandbox Server. Domain objects reside in the **Domain Layer** and contain pure business logic with zero infrastructure dependencies.

## ⚠️ Security-First Domain Design

All domain objects enforce security invariants at the business logic level. Invalid states are unrepresentable, and all state transitions are validated.

---

## Aggregates

Aggregates are clusters of domain objects that form a consistency boundary. Only the aggregate root can be referenced from outside.

### Session Aggregate

**Root Entity:** `Session`

**Description:** Represents a user's isolated sandbox session with video streaming and input handling.

**Aggregate Boundary:**
- Session (root)
- SandboxEnvironment (entity)
- VideoStream (entity)
- InputChannel (entity)

**Invariants:**
1. Session MUST have valid user reference
2. Session MUST have resource limits within allowed ranges
3. Session CANNOT exceed maximum lifetime (configured timeout)
4. Session state transitions MUST follow valid flow: `Initializing → Ready → Active → Terminating → Terminated`
5. Terminated sessions CANNOT be reactivated
6. Active sessions MUST have associated sandbox environment

**Value Objects:**
- `SessionId` - Unique session identifier (UUID v4)
- `ResourceLimits` - CPU/memory/PID limits
- `SessionState` - Enum: Initializing, Ready, Active, Terminating, Terminated
- `SessionTimeout` - Inactivity timeout duration

**Domain Events:**
- `SessionCreated` - Emitted when session initialized
- `SessionReady` - Sandbox created, ready for streaming
- `SessionActivated` - WebRTC connection established
- `SessionTerminated` - Session ended (timeout, user action, error)
- `SessionInputReceived` - Input event forwarded to sandbox

**Commands:**
- `CreateSession` - Initialize new session
- `ActivateSession` - Mark session as active
- `TerminateSession` - End session and cleanup
- `ForwardInput` - Send input to session

**Example:**
```rust
pub struct Session {
    id: SessionId,
    user_id: UserId,
    state: SessionState,
    sandbox: Option<SandboxEnvironment>,
    video_stream: Option<VideoStream>,
    created_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl Session {
    /// Create new session (enforces invariants)
    pub fn create(
        user_id: UserId,
        limits: ResourceLimits,
        timeout: SessionTimeout,
    ) -> Result<Self, DomainError> {
        // Validate limits
        limits.validate()?;
        
        Ok(Self {
            id: SessionId::generate(),
            user_id,
            state: SessionState::Initializing,
            sandbox: None,
            video_stream: None,
            created_at: Utc::now(),
            last_activity: Utc::now(),
            expires_at: Utc::now() + timeout.duration(),
        })
    }
    
    /// Transition to Ready state
    pub fn mark_ready(&mut self, sandbox: SandboxEnvironment) -> Result<(), DomainError> {
        match self.state {
            SessionState::Initializing => {
                self.state = SessionState::Ready;
                self.sandbox = Some(sandbox);
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: self.state.clone(),
                to: SessionState::Ready,
            }),
        }
    }
    
    /// Check if session has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}
```

---

### User Aggregate

**Root Entity:** `User`

**Description:** Represents a user account with authentication credentials and authorization roles.

**Aggregate Boundary:**
- User (root)
- Credential (value object)
- Role (value object)

**Invariants:**
1. Username MUST be unique
2. Password MUST be hashed with argon2id (never store plaintext)
3. Username MUST be 3-32 alphanumeric characters
4. Password MUST be minimum 16 characters (configured)
5. User CANNOT have duplicate roles
6. Active users MUST have at least one role

**Value Objects:**
- `UserId` - Unique user identifier (UUID v4)
- `Username` - Validated username string
- `PasswordHash` - argon2id hashed password
- `Role` - Enum: Admin, User, Viewer
- `Email` - Validated email address (optional)

**Domain Events:**
- `UserRegistered` - New user created
- `UserAuthenticated` - Successful login
- `UserAuthenticationFailed` - Failed login attempt
- `UserRoleGranted` - Role added to user
- `UserRoleRevoked` - Role removed from user
- `UserDisabled` - Account disabled
- `UserEnabled` - Account re-enabled

**Commands:**
- `RegisterUser` - Create new user
- `AuthenticateUser` - Validate credentials
- `ChangePassword` - Update password
- `GrantRole` - Add role to user
- `RevokeRole` - Remove role from user
- `DisableUser` - Deactivate account
- `EnableUser` - Reactivate account

**Example:**
```rust
pub struct User {
    id: UserId,
    username: Username,
    password_hash: PasswordHash,
    roles: HashSet<Role>,
    email: Option<Email>,
    enabled: bool,
    created_at: DateTime<Utc>,
}

impl User {
    /// Register new user with password
    pub fn register(
        username: Username,
        password: &str,
        role: Role,
    ) -> Result<Self, DomainError> {
        // Validate password strength
        Self::validate_password_strength(password)?;
        
        // Hash password with argon2id
        let password_hash = PasswordHash::from_plaintext(password)?;
        
        let mut roles = HashSet::new();
        roles.insert(role);
        
        Ok(Self {
            id: UserId::generate(),
            username,
            password_hash,
            roles,
            email: None,
            enabled: true,
            created_at: Utc::now(),
        })
    }
    
    /// Authenticate user with password
    pub fn authenticate(&self, password: &str) -> Result<(), DomainError> {
        if !self.enabled {
            return Err(DomainError::UserDisabled);
        }
        
        if !self.password_hash.verify(password)? {
            return Err(DomainError::InvalidCredentials);
        }
        
        Ok(())
    }
    
    /// Check if user has specific role
    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }
    
    /// Validate password meets security requirements
    fn validate_password_strength(password: &str) -> Result<(), DomainError> {
        if password.len() < 16 {
            return Err(DomainError::WeakPassword("Minimum 16 characters required"));
        }
        
        // Additional checks: uppercase, lowercase, digits, special chars
        // ...
        
        Ok(())
    }
}
```

---

### Permission Aggregate

**Root Entity:** `Permission`

**Description:** Represents access control rules for files and resources.

**Aggregate Boundary:**
- Permission (root)
- AccessLevel (value object)
- ResourcePath (value object)

**Invariants:**
1. Permission MUST reference valid user
2. Permission MUST reference valid resource path
3. Resource path MUST be within allowed storage boundaries
4. Access level MUST be one of: Read, Write, Execute
5. Permissions CANNOT grant access outside user's storage quota
6. Admin users bypass permission checks (enforced in authorization service)

**Value Objects:**
- `PermissionId` - Unique permission identifier
- `ResourcePath` - Validated file/directory path
- `AccessLevel` - Enum: Read, Write, Execute
- `PermissionScope` - File vs Directory permission

**Domain Events:**
- `PermissionGranted` - User given access to resource
- `PermissionRevoked` - User access removed
- `PermissionModified` - Access level changed

**Commands:**
- `GrantPermission` - Give user access to resource
- `RevokePermission` - Remove user access
- `ModifyPermission` - Change access level

**Example:**
```rust
pub struct Permission {
    id: PermissionId,
    user_id: UserId,
    resource_path: ResourcePath,
    access_levels: HashSet<AccessLevel>,
    scope: PermissionScope,
    granted_at: DateTime<Utc>,
    granted_by: UserId,
}

impl Permission {
    /// Grant new permission
    pub fn grant(
        user_id: UserId,
        resource_path: ResourcePath,
        access_levels: HashSet<AccessLevel>,
        granted_by: UserId,
    ) -> Result<Self, DomainError> {
        // Validate resource path is within allowed boundaries
        resource_path.validate_within_storage_root()?;
        
        // At least one access level required
        if access_levels.is_empty() {
            return Err(DomainError::InvalidPermission("No access levels specified"));
        }
        
        Ok(Self {
            id: PermissionId::generate(),
            user_id,
            resource_path,
            access_levels,
            scope: PermissionScope::File,
            granted_at: Utc::now(),
            granted_by,
        })
    }
    
    /// Check if permission allows specific access
    pub fn allows(&self, access: &AccessLevel) -> bool {
        self.access_levels.contains(access)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AccessLevel {
    Read,
    Write,
    Execute,
}
```

---

## Domain Services

Services that contain business logic not belonging to a single aggregate.

### AuthenticationService

**Responsibility:** Validate user credentials and issue JWT tokens.

**Operations:**
- `authenticate(username, password) -> Result<JwtToken>`
- `refresh_token(refresh_token) -> Result<JwtToken>`
- `revoke_token(token_id) -> Result<()>`

**Security Invariants:**
- Tokens MUST have expiry (15 min for access, 7 days for refresh)
- Tokens MUST be signed with HMAC-SHA256
- Failed authentication attempts MUST be rate-limited
- Account locked after N failed attempts

---

### AuthorizationService

**Responsibility:** Enforce role-based access control and permission checks.

**Operations:**
- `authorize_file_access(user, path, access_level) -> Result<()>`
- `authorize_session_access(user, session) -> Result<()>`
- `check_role(user, required_role) -> bool`

**Security Invariants:**
- Admin users bypass file permissions
- Users can only access own sessions (unless admin)
- Audit log MUST be written for all authorization failures

---

### SandboxIsolationService

**Responsibility:** Create and configure isolated sandbox environments.

**Operations:**
- `create_sandbox(session, permissions) -> Result<SandboxEnvironment>`
- `apply_resource_limits(sandbox, limits) -> Result<()>`
- `destroy_sandbox(sandbox) -> Result<()>`

**Security Invariants:**
- Sandbox MUST have user namespace (rootless)
- Landlock policies MUST be applied before execution
- seccomp filters MUST deny dangerous syscalls
- cgroups limits MUST be enforced

---

### EncryptionService

**Responsibility:** Encrypt and decrypt files and sensitive data.

**Operations:**
- `encrypt_file(path, key) -> Result<()>`
- `decrypt_file(path, key) -> Result<Vec<u8>>`
- `derive_key(user_id, salt) -> Result<EncryptionKey>`

**Security Invariants:**
- MUST use AES-256-GCM for file encryption
- Keys MUST be derived with argon2id or scrypt
- Keys NEVER stored in plaintext
- Encrypted files include authentication tag

---

### AuditService

**Responsibility:** Record security events to immutable audit log.

**Operations:**
- `log_authentication(user, success, ip) -> Result<()>`
- `log_authorization_failure(user, resource, reason) -> Result<()>`
- `log_file_access(user, path, action) -> Result<()>`
- `log_session_event(session, event) -> Result<()>`

**Security Invariants:**
- Audit logs MUST be append-only
- Logs MUST include timestamp, user, action, result
- Logs MUST NOT contain sensitive data (passwords, tokens)
- Logs MUST survive application crashes

---

## Value Objects

Immutable objects defined by their attributes.

### SessionId

```rust
pub struct SessionId(Uuid);

impl SessionId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
    
    pub fn from_string(s: &str) -> Result<Self, DomainError> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|_| DomainError::InvalidSessionId)
    }
}
```

### ResourceLimits

```rust
pub struct ResourceLimits {
    memory_mb: u32,
    cpu_percent: u8,
    pid_limit: u16,
}

impl ResourceLimits {
    pub fn new(memory_mb: u32, cpu_percent: u8, pid_limit: u16) -> Result<Self, DomainError> {
        if cpu_percent > 100 {
            return Err(DomainError::InvalidResourceLimit("CPU percent > 100"));
        }
        
        if memory_mb > 8192 {
            return Err(DomainError::InvalidResourceLimit("Memory > 8GB"));
        }
        
        Ok(Self { memory_mb, cpu_percent, pid_limit })
    }
}
```

### JwtToken

```rust
pub struct JwtToken {
    value: String,
    expires_at: DateTime<Utc>,
    token_type: TokenType,
}

pub enum TokenType {
    Access,
    Refresh,
}

impl JwtToken {
    /// Validate token hasn't expired
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }
}
```

### VideoConfig

```rust
pub struct VideoConfig {
    framerate: u8,
    bitrate_kbps: u16,
    codec: VideoCodec,
    preset: EncodingPreset,
}

pub enum VideoCodec {
    H264,
    VP8,
    VP9,
}

pub enum EncodingPreset {
    Ultrafast,
    Fast,
    Medium,
}
```

---

## Repository Interfaces (Ports)

Abstractions for data persistence.

### UserRepository

```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: &UserId) -> Result<User, RepositoryError>;
    async fn find_by_username(&self, username: &str) -> Result<User, RepositoryError>;
    async fn save(&self, user: &User) -> Result<(), RepositoryError>;
    async fn delete(&self, id: &UserId) -> Result<(), RepositoryError>;
}
```

### SessionRepository

```rust
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn find_by_id(&self, id: &SessionId) -> Result<Session, RepositoryError>;
    async fn find_active_by_user(&self, user_id: &UserId) -> Result<Vec<Session>, RepositoryError>;
    async fn save(&self, session: &Session) -> Result<(), RepositoryError>;
    async fn delete(&self, id: &SessionId) -> Result<(), RepositoryError>;
}
```

### PermissionRepository

```rust
#[async_trait]
pub trait PermissionRepository: Send + Sync {
    async fn find_by_user_and_path(
        &self,
        user_id: &UserId,
        path: &ResourcePath,
    ) -> Result<Vec<Permission>, RepositoryError>;
    
    async fn save(&self, permission: &Permission) -> Result<(), RepositoryError>;
    async fn delete(&self, id: &PermissionId) -> Result<(), RepositoryError>;
}
```

### AuditLogRepository

```rust
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    async fn append(&self, event: AuditEvent) -> Result<(), RepositoryError>;
    
    async fn find_by_user(
        &self,
        user_id: &UserId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<AuditEvent>, RepositoryError>;
}
```

---

## Domain Events

Events emitted when significant business events occur.

```rust
pub enum DomainEvent {
    SessionCreated { session_id: SessionId, user_id: UserId },
    SessionTerminated { session_id: SessionId, reason: TerminationReason },
    UserAuthenticated { user_id: UserId, ip_address: IpAddress },
    UserAuthenticationFailed { username: String, ip_address: IpAddress },
    PermissionGranted { permission_id: PermissionId, user_id: UserId, path: ResourcePath },
    PermissionRevoked { permission_id: PermissionId },
}
```

---

## Error Types

Domain-specific errors.

```rust
pub enum DomainError {
    // User errors
    UserNotFound,
    UserDisabled,
    InvalidCredentials,
    WeakPassword(&'static str),
    
    // Session errors
    SessionNotFound,
    SessionExpired,
    InvalidStateTransition { from: SessionState, to: SessionState },
    
    // Permission errors
    PermissionDenied,
    InvalidPermission(&'static str),
    
    // Resource errors
    InvalidResourceLimit(&'static str),
    ResourcePathOutOfBounds,
    
    // Validation errors
    InvalidSessionId,
    InvalidUserId,
    InvalidUsername,
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-13  
**Related Documents:** [COMMANDS.md](COMMANDS.md), [QUERIES.md](QUERIES.md), [ARCHITECTURE.md](ARCHITECTURE.md)
