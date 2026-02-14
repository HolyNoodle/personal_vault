# RegisterUserCommand

**Purpose:** Super Admin creates a new user account (Owner or Client).

**Persona:** Super Admin

**Module:** `application::super_admin::commands::register_user`

---

## Command Structure

```rust
pub struct RegisterUserCommand {
    pub email: Email,                    // Value object
    pub role: UserRole,                  // Owner or Client (not SuperAdmin)
    pub storage_quota_bytes: u64,        // e.g., 10GB = 10_000_000_000
    pub local_root_folder: PathBuf,      // e.g., /data/users/{user_id}/
    pub created_by_admin_id: UserId,     // Super Admin performing action
}
```

---

## Validations

- ✅ Email is valid format (RFC 5322)
- ✅ Role is Owner or Client (cannot create SuperAdmin via this command)
- ✅ Storage quota > 0 and <= system max (configurable, default 1TB)
- ✅ Local root folder path is within allowed directory (`/data/users/`)
- ✅ created_by_admin_id exists and has SuperAdmin role
- ✅ Email not already registered

---

## Acceptance Criteria

### AC1: Happy Path - Valid User Registration
**GIVEN** a Super Admin is authenticated  
**AND** the email "john@example.com" is not registered  
**WHEN** RegisterUserCommand is executed with:
- email: "john@example.com"
- role: Owner
- storage_quota_bytes: 10_000_000_000 (10GB)
- local_root_folder: "/data/users/{generated_user_id}/"

**THEN** a new User aggregate is created with:
- Unique UserId generated
- Email: "john@example.com"
- Role: Owner
- Storage quota: 10GB
- is_deleted: false
- created_at: current UTC timestamp

**AND** User is persisted to database table `users`  
**AND** filesystem folder `/data/users/{user_id}/` is created with permissions 700  
**AND** UserRegistered domain event is emitted  
**AND** audit log entry "UserRegistered" is created with:
- actor_id: {created_by_admin_id}
- target_user_id: {new_user_id}
- action: "UserRegistered"
- timestamp: current UTC

**AND** HTTP response 201 Created with:
```json
{
  "user_id": "usr_550e8400e29b41d4a716446655440000",
  "invitation_link": "https://sandbox.example.com/invite/tk_abc123def456"
}
```

### AC2: Validation - Duplicate Email Rejected
**GIVEN** a user with email "existing@example.com" already exists in database  
**WHEN** RegisterUserCommand is executed with email "existing@example.com"  
**THEN** command fails with `DomainError::EmailAlreadyExists`  
**AND** no new user is created in database  
**AND** no filesystem folder is created  
**AND** no UserRegistered events are emitted  
**AND** HTTP response 409 Conflict with:
```json
{
  "error": "EmailAlreadyExists",
  "message": "A user with this email already exists"
}
```

### AC3: Authorization - Only Super Admin Can Register
**GIVEN** a regular Owner user with role=Owner is authenticated  
**WHEN** RegisterUserCommand is executed by the Owner (created_by_admin_id = owner_user_id)  
**THEN** command fails with `DomainError::Unauthorized`  
**AND** no user is created  
**AND** audit log entry "UnauthorizedUserRegistration" is created with:
- actor_id: {owner_user_id}
- action: "UnauthorizedUserRegistration"
- result: "Denied"

**AND** HTTP response 403 Forbidden

### AC4: Validation - Zero Quota Rejected
**GIVEN** a Super Admin is authenticated  
**WHEN** RegisterUserCommand is executed with storage_quota_bytes = 0  
**THEN** command fails with `DomainError::InvalidStorageQuota`  
**AND** no user is created  
**AND** HTTP response 400 Bad Request

### AC5: Validation - Invalid Email Format Rejected
**GIVEN** a Super Admin is authenticated  
**WHEN** RegisterUserCommand is executed with email "not-an-email"  
**THEN** Email value object creation fails with `DomainError::InvalidEmail`  
**AND** command handler is never called (fails at DTO validation)  
**AND** no user is created  
**AND** HTTP response 400 Bad Request

### AC6: Validation - Path Traversal Rejected
**GIVEN** a Super Admin is authenticated  
**WHEN** RegisterUserCommand is executed with local_root_folder = "/data/users/../../etc/"  
**THEN** command fails with `DomainError::InvalidPath`  
**AND** no user is created  
**AND** no filesystem operations performed

### AC7: Validation - Excessive Quota Rejected
**GIVEN** a Super Admin is authenticated  
**AND** system maximum storage quota is 1TB (1_000_000_000_000 bytes)  
**WHEN** RegisterUserCommand is executed with storage_quota_bytes = 10_000_000_000_000 (10TB)  
**THEN** command fails with `DomainError::QuotaExceedsSystemLimit`  
**AND** no user is created

### AC8: Side Effect - Invitation Created
**GIVEN** a Super Admin is authenticated  
**WHEN** RegisterUserCommand successfully creates a user  
**THEN** an invitation token is generated  
**AND** invitation is saved to `invitations` table with:
- user_id: {new_user_id}
- token: {random_secure_token}
- expires_at: 7 days from now
- used: false

**AND** invitation link is returned in response

---

## Handler Implementation

```rust
pub struct RegisterUserCommandHandler {
    user_repository: Arc<dyn UserRepository>,
    filesystem: Arc<dyn FileSystem>,
    event_publisher: Arc<dyn EventPublisher>,
    invitation_repository: Arc<dyn InvitationRepository>,
}

#[async_trait]
impl CommandHandler<RegisterUserCommand> for RegisterUserCommandHandler {
    async fn handle(&self, cmd: RegisterUserCommand) -> Result<UserId, DomainError> {
        // 1. Validate Super Admin authorization
        let admin = self.user_repository
            .find_by_id(&cmd.created_by_admin_id)
            .await?
            .ok_or(DomainError::Unauthorized)?;
        
        if admin.role != UserRole::SuperAdmin {
            return Err(DomainError::Unauthorized);
        }
        
        // 2. Validate email not already used
        if self.user_repository.exists_by_email(&cmd.email).await? {
            return Err(DomainError::EmailAlreadyExists);
        }
        
        // 3. Validate path is within allowed directory
        if !cmd.local_root_folder.starts_with("/data/users/") {
            return Err(DomainError::InvalidPath);
        }
        
        // 4. Validate quota
        if cmd.storage_quota_bytes == 0 {
            return Err(DomainError::InvalidStorageQuota);
        }
        if cmd.storage_quota_bytes > MAX_STORAGE_QUOTA {
            return Err(DomainError::QuotaExceedsSystemLimit);
        }
        
        // 5. Create User aggregate
        let user = User::register(
            UserId::generate(),
            cmd.email,
            cmd.role,
            cmd.storage_quota_bytes,
        )?;
        
        // 6. Create filesystem folder
        self.filesystem.create_user_folder(&cmd.local_root_folder).await?;
        
        // 7. Persist user
        self.user_repository.save(&user).await?;
        
        // 8. Create invitation
        let invitation = Invitation::create(
            user.id.clone(),
            InvitationToken::generate(),
            Duration::days(7),
        )?;
        self.invitation_repository.save(&invitation).await?;
        
        // 9. Emit domain events
        self.event_publisher.publish_all(user.events()).await?;
        
        Ok(user.id)
    }
}
```

---

## Domain Events Emitted

### UserRegistered
```rust
pub struct UserRegistered {
    pub user_id: UserId,
    pub email: Email,
    pub role: UserRole,
    pub timestamp: DateTime<Utc>,
}
```

**Event Handlers:**
- `AuditEventSubscriber` - Creates audit log entry
- `EmailNotificationSubscriber` - Sends welcome email (future)

---

## API Endpoint

```http
POST /api/admin/users
Authorization: Bearer {super_admin_jwt_token}
Content-Type: application/json

Request Body:
{
  "email": "john@example.com",
  "role": "Owner",
  "storage_quota_gb": 100
}

Response 201 Created:
{
  "user_id": "usr_550e8400e29b41d4a716446655440000",
  "invitation_link": "https://sandbox.example.com/invite/tk_abc123def456",
  "created_at": "2026-02-14T10:30:00Z"
}

Response 409 Conflict (duplicate email):
{
  "error": "EmailAlreadyExists",
  "message": "A user with this email already exists"
}

Response 403 Forbidden (non-admin):
{
  "error": "Unauthorized",
  "message": "Only Super Admins can register users"
}
```

---

## Test Implementation

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn ac1_valid_user_registration() {
        // GIVEN
        let ctx = TestContext::new().await;
        let admin = ctx.create_super_admin("admin@example.com").await;
        
        // WHEN
        let command = RegisterUserCommand {
            email: Email::new("john@example.com").unwrap(),
            role: UserRole::Owner,
            storage_quota_bytes: 10_000_000_000,
            local_root_folder: ctx.temp_dir().join("john"),
            created_by_admin_id: admin.id,
        };
        let result = ctx.execute_command(command).await;
        
        // THEN
        assert!(result.is_ok());
        let user_id = result.unwrap();
        
        // User persisted
        let user = ctx.get_user(&user_id).await.unwrap();
        assert_eq!(user.email.as_str(), "john@example.com");
        assert_eq!(user.role, UserRole::Owner);
        assert_eq!(user.storage_quota_bytes, 10_000_000_000);
        assert!(!user.is_deleted);
        
        // Folder created
        assert!(ctx.temp_dir().join("john").exists());
        
        // Event emitted
        ctx.assert_event_published::<UserRegistered>();
        
        // Audit log
        ctx.assert_audit_log_exists("UserRegistered", admin.id);
    }
    
    #[tokio::test]
    async fn ac2_duplicate_email_rejected() {
        let ctx = TestContext::new().await;
        let admin = ctx.create_super_admin().await;
        ctx.create_owner("existing@example.com").await;
        
        let command = RegisterUserCommand {
            email: Email::new("existing@example.com").unwrap(),
            role: UserRole::Client,
            storage_quota_bytes: 1_000_000_000,
            local_root_folder: ctx.temp_dir().join("duplicate"),
            created_by_admin_id: admin.id,
        };
        let result = ctx.execute_command(command).await;
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DomainError::EmailAlreadyExists);
        assert!(!ctx.temp_dir().join("duplicate").exists());
    }
    
    // ... additional test cases for AC3-AC8
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Parent Document:** [../../COMMANDS.md](../../COMMANDS.md)  
**Related:** [TESTING.md](../../../docs/TESTING.md), [User Aggregate](../../domain/aggregates/user.md)
