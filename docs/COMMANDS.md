# Application Commands

## Overview

Commands represent **write operations** (user intentions) that modify system state. They follow CQRS principles and are handled by command handlers in the Application Layer.

**Command Characteristics:**
- Represent user intention (imperative names: "Create", "Grant", "Revoke")
- Contain only data needed to perform action
- Validated before execution
- May emit domain events
- Return success/failure (not data)
- Logged for audit trail

**Architecture Flow:**
```
HTTP Request → Command DTO → Validate → Command Handler → Domain Aggregate → Domain Event → Event Handlers
                                              ↓
                                         Repository
                                              ↓
                                         Database
```

## ⚠️ Security-First Command Design

All commands MUST:
1. **Validate input** - Check all parameters before execution
2. **Authorize action** - Verify user has permission
3. **Emit events** - Domain events for audit trail
4. **Be idempotent** - Safe to retry (where possible)
5. **Transactional** - Rollback on failure

---

## Command Structure

```rust
// Command (data only, no logic)
pub struct SomeCommand {
    pub field1: ValueObject,
    pub field2: String,
    pub actor_id: UserId,  // Who is performing this action
}

// Handler (application logic)
#[async_trait]
pub trait CommandHandler<C>: Send + Sync {
    async fn handle(&self, command: C) -> Result<CommandResult, DomainError>;
}

// Result
pub enum CommandResult {
    Success,
    Created { id: String },
    Updated,
}

// Context (injected by infrastructure)
pub struct CommandContext {
    pub user_id: UserId,
    pub ip_address: IpAddr,
    pub timestamp: DateTime<Utc>,
    pub request_id: String,
}
```

---

## Authentication & User Management

### 1. RegisterUserCommand

**Purpose:** Super Admin creates a new user account (Owner or Client).

**Command:**
```rust
pub struct RegisterUserCommand {
    pub email: Email,                    // Value object
    pub role: UserRole,                  // Owner or Client (not SuperAdmin)
    pub storage_quota_bytes: u64,        // e.g., 10GB = 10_000_000_000
    pub local_root_folder: PathBuf,      // e.g., /data/users/{user_id}/
    pub created_by_admin_id: UserId,     // Super Admin performing action
}
```

**Validations:**
- ✅ Email is valid format
- ✅ Role is Owner or Client (cannot create SuperAdmin via this command)
- ✅ Storage quota > 0 and <= system max
- ✅ Local root folder path is within allowed directory
- ✅ created_by_admin_id exists and has SuperAdmin role
- ✅ Email not already registered

**Acceptance Criteria:**

#### AC1: Happy Path - Valid User Registration
**GIVEN** a Super Admin is authenticated  
**AND** the email "john@example.com" is not registered  
**WHEN** RegisterUserCommand is executed with valid data  
**THEN** a new User is created with Owner role  
**AND** User is persisted to database  
**AND** filesystem folder `/data/users/{user_id}/` is created  
**AND** UserRegistered event is emitted  
**AND** audit log entry "UserRegistered" is created

#### AC2: Validation - Duplicate Email Rejected
**GIVEN** a user with email "existing@example.com" already exists  
**WHEN** RegisterUserCommand is executed with email "existing@example.com"  
**THEN** command fails with `DomainError::EmailAlreadyExists`  
**AND** no new user is created  
**AND** no filesystem folder is created  
**AND** no events are emitted

#### AC3: Authorization - Only Super Admin Can Register
**GIVEN** a regular Owner user is authenticated  
**WHEN** RegisterUserCommand is executed by the Owner  
**THEN** command fails with `DomainError::Unauthorized`  
**AND** audit log entry "UnauthorizedUserRegistration" is created

#### AC4: Validation - Zero Quota Rejected
**GIVEN** a Super Admin is authenticated  
**WHEN** RegisterUserCommand is executed with storage_quota_bytes = 0  
**THEN** command fails with `DomainError::InvalidStorageQuota`  
**AND** no user is created

#### AC5: Validation - Invalid Email Format Rejected
**GIVEN** a Super Admin is authenticated  
**WHEN** RegisterUserCommand is executed with email "not-an-email"  
**THEN** command fails with `DomainError::InvalidEmail`  
**AND** no user is created

**Handler:**
```rust
pub struct TerminateSessionCommand {
    pub session_id: SessionId,
    pub reason: TerminationReason,
}

pub enum TerminationReason {
    UserRequested,
    Timeout,
    Error(String),
    AdminTermination,
}
```

**Preconditions:**
- Session MUST exist
- User MUST own session OR be admin
- Session MUST NOT already be in Terminated state

**Business Logic:**
1. Load Session aggregate from repository
2. Validate user authorization
3. Transition session to Terminating state
4. Stop WebRTC connection
5. Stop video encoding process
6. Kill sandbox processes
7. Unmount sandbox filesystem
8. Remove cgroup
9. Transition to Terminated state
10. Emit `SessionTerminated` domain event

**Postconditions:**
- Session in Terminated state
- All sandbox resources cleaned up
- WebRTC connection closed
- Domain event published
- Audit log entry created

**Output:**
```rust
pub struct TerminateSessionResult {
    pub session_id: SessionId,
    pub terminated_at: DateTime<Utc>,
}
```

**Errors:**
- `SessionNotFound` - Session doesn't exist
- `PermissionDenied` - User doesn't own session
- `InvalidStateTransition` - Already terminated

**Security Considerations:**
- MUST verify user owns session before termination
- MUST ensure complete resource cleanup
- MUST log termination with reason

---

### ForwardInputCommand

**Purpose:** Forward mouse/keyboard input from browser to sandboxed application.

**Input:**
```rust
pub struct ForwardInputCommand {
    pub session_id: SessionId,
    pub input_event: InputEvent,
}

pub enum InputEvent {
    Mouse {
        x: u16,
        y: u16,
        button: MouseButton,
        action: MouseAction,
    },
    Keyboard {
        key: KeyCode,
        action: KeyAction,
        modifiers: Vec<KeyModifier>,
    },
}
```

**Preconditions:**
- Session MUST exist and be in Active state
- User MUST own session
- Input event MUST pass validation (coordinates in range, allowed keys)
- Rate limit MUST NOT be exceeded (100 events/sec)

**Business Logic:**
1. Validate session exists and is active
2. Validate user owns session
3. Validate input event (sanitize coordinates, check allowed keys)
4. Check rate limit
5. Inject input into sandbox X11 session
6. Update session last_activity timestamp
7. Emit `SessionInputReceived` domain event

**Postconditions:**
- Input forwarded to sandbox
- Session activity timestamp updated
- Rate limiter incremented

**Output:**
```rust
pub struct ForwardInputResult {
    pub accepted: bool,
}
```

**Errors:**
- `SessionNotFound` - Session doesn't exist
- `SessionNotActive` - Session not in Active state
- `PermissionDenied` - User doesn't own session
- `RateLimitExceeded` - Too many input events
- `InvalidInput` - Malformed input event

**Security Considerations:**
- MUST sanitize all input (clamp coordinates, filter keys)
- MUST rate limit to prevent DoS
- MUST NOT allow function keys that trigger OS commands
- MUST log suspicious input patterns

---

## User Commands

### RegisterUserCommand

**Purpose:** Create a new user account.

**Input:**
```rust
pub struct RegisterUserCommand {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub role: Role,  // Only admin can set role
}
```

**Preconditions:**
- Username MUST be 3-32 alphanumeric characters
- Password MUST meet strength requirements (16+ chars)
- Username MUST be unique
- Email MUST be valid format (if provided)
- Only admins can assign Admin role

**Business Logic:**
1. Validate username format and uniqueness
2. Validate password strength
3. Validate email format (if provided)
4. Check requester authorization for role assignment
5. Create User aggregate (hashes password with argon2id)
6. Emit `UserRegistered` domain event
7. Save to repository

**Postconditions:**
- User created and persisted
- Password hashed with argon2id
- Domain event published
- Audit log entry created

**Output:**
```rust
pub struct RegisterUserResult {
    pub user_id: UserId,
    pub username: String,
    pub created_at: DateTime<Utc>,
}
```

**Errors:**
- `UsernameAlreadyExists` - Username taken
- `WeakPassword` - Password doesn't meet requirements
- `InvalidEmail` - Email format invalid
- `PermissionDenied` - Cannot assign Admin role

**Security Considerations:**
- MUST hash password with argon2id (never store plaintext)
- MUST validate password strength
- MUST prevent username enumeration (same error for existing user)
- MUST log registration attempts

---

### AuthenticateUserCommand

**Purpose:** Validate user credentials and issue JWT tokens.

**Input:**
```rust
pub struct AuthenticateUserCommand {
    pub username: String,
    pub password: String,
}
```

**Preconditions:**
- User MUST exist
- User MUST be enabled
- Account MUST NOT be locked (after failed attempts)

**Business Logic:**
1. Find user by username
2. Check account is enabled
3. Check account not locked
4. Validate password against hash
5. If valid:
   - Generate access token (15 min expiry)
   - Generate refresh token (7 day expiry)
   - Reset failed attempt counter
   - Emit `UserAuthenticated` event
6. If invalid:
   - Increment failed attempt counter
   - Lock account if threshold exceeded
   - Emit `UserAuthenticationFailed` event

**Postconditions:**
- JWT tokens issued (if successful)
- Failed attempt counter updated
- Domain event published
- Audit log entry created

**Output:**
```rust
pub struct AuthenticateUserResult {
    pub access_token: JwtToken,
    pub refresh_token: JwtToken,
    pub user: UserDto,
}
```

**Errors:**
- `InvalidCredentials` - Username/password incorrect
- `UserDisabled` - Account disabled
- `AccountLocked` - Too many failed attempts

**Security Considerations:**
- MUST use constant-time password comparison
- MUST rate limit authentication attempts
- MUST lock account after N failed attempts (default: 5)
- MUST log all authentication attempts (success and failure)
- MUST NOT reveal whether username or password was wrong

---

## Permission Commands

### GrantPermissionCommand

**Purpose:** Give a user access to a file or directory.

**Input:**
```rust
pub struct GrantPermissionCommand {
    pub target_user_id: UserId,
    pub resource_path: ResourcePath,
    pub access_levels: HashSet<AccessLevel>,
    pub granted_by: UserId,  // From context
}
```

**Preconditions:**
- Target user MUST exist
- Resource MUST exist within storage
- Granter MUST be admin OR owner of resource
- Access levels MUST be non-empty
- Resource path MUST be within allowed storage root

**Business Logic:**
1. Validate target user exists
2. Validate resource exists
3. Validate granter has authority
4. Create Permission aggregate
5. Emit `PermissionGranted` event
6. Save to repository

**Postconditions:**
- Permission created and persisted
- Domain event published
- Audit log entry created

**Output:**
```rust
pub struct GrantPermissionResult {
    pub permission_id: PermissionId,
}
```

**Errors:**
- `UserNotFound` - Target user doesn't exist
- `ResourceNotFound` - File/directory doesn't exist
- `PermissionDenied` - Granter lacks authority
- `InvalidResourcePath` - Path outside storage root

**Security Considerations:**
- MUST validate resource path doesn't escape storage
- MUST verify granter authorization
- MUST log permission grants

---

### RevokePermissionCommand

**Purpose:** Remove user access to a file or directory.

**Input:**
```rust
pub struct RevokePermissionCommand {
    pub permission_id: PermissionId,
    pub revoked_by: UserId,  // From context
}
```

**Preconditions:**
- Permission MUST exist
- Revoker MUST be admin OR original granter

**Business Logic:**
1. Load Permission aggregate
2. Validate revoker has authority
3. Delete permission
4. Emit `PermissionRevoked` event

**Postconditions:**
- Permission deleted
- Domain event published
- Audit log entry created

**Output:**
```rust
pub struct RevokePermissionResult {
    pub revoked_at: DateTime<Utc>,
}
```

**Errors:**
- `PermissionNotFound` - Permission doesn't exist
- `PermissionDenied` - Revoker lacks authority

**Security Considerations:**
- MUST verify revoker authorization
- MUST log permission revocations
- MUST terminate active sessions using revoked permissions

---

## Command Handler Implementation Pattern

```rust
pub struct CreateSessionCommandHandler {
    session_repository: Arc<dyn SessionRepository>,
    user_repository: Arc<dyn UserRepository>,
    permission_repository: Arc<dyn PermissionRepository>,
    sandbox_service: Arc<dyn SandboxIsolationService>,
    event_publisher: Arc<dyn EventPublisher>,
    audit_service: Arc<dyn AuditService>,
}

#[async_trait]
impl CommandHandler<CreateSessionCommand> for CreateSessionCommandHandler {
    async fn handle(
        &self,
        command: CreateSessionCommand,
        context: CommandContext,
    ) -> Result<CommandResult, CommandError> {
        // 1. Validate
        let user = self.user_repository
            .find_by_id(&command.user_id)
            .await?;
        
        if !user.enabled {
            return Err(CommandError::UserDisabled);
        }
        
        // 2. Check business rules
        let active_sessions = self.session_repository
            .find_active_by_user(&command.user_id)
            .await?;
        
        if active_sessions.len() >= MAX_CONCURRENT_SESSIONS {
            return Err(CommandError::SessionLimitExceeded);
        }
        
        // 3. Create domain object
        let session = Session::create(
            command.user_id,
            ResourceLimits::default(),
            SessionTimeout::default(),
        )?;
        
        // 4. Persist
        self.session_repository.save(&session).await?;
        
        // 5. Emit event
        self.event_publisher.publish(DomainEvent::SessionCreated {
            session_id: session.id().clone(),
            user_id: session.user_id().clone(),
        }).await?;
        
        // 6. Audit log
        self.audit_service.log_session_event(
            &session,
            "session_created",
            context.ip_address,
        ).await?;
        
        // 7. Return result
        Ok(CommandResult::Created {
            id: session.id().to_string(),
        })
    }
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-13  
**Related Documents:** [QUERIES.md](QUERIES.md), [DOMAIN_OBJECTS.md](DOMAIN_OBJECTS.md), [ARCHITECTURE.md](ARCHITECTURE.md)
