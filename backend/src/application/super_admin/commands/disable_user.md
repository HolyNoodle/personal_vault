# DisableUserCommand

**Purpose:** Super Admin disables a user account (prevent login without deleting data).

**Persona:** Super Admin

**Module:** `application::super_admin::commands::disable_user`

---

## Command Structure

```rust
pub struct DisableUserCommand {
    pub user_id: UserId,
    pub disabled_by: UserId,        // Super Admin performing action
    pub reason: String,              // Reason for disabling
}
```

---

## Validations

- ✅ user_id exists
- ✅ disabled_by has SuperAdmin role
- ✅ user_id is not already disabled
- ✅ cannot disable own account
- ✅ reason is not empty (max 500 chars)

---

## Acceptance Criteria

### AC1: Happy Path - Disable User Successfully
**GIVEN** a Super Admin is authenticated  
**AND** an active Owner user exists (usr_123)  
**WHEN** DisableUserCommand is executed with:
- user_id: usr_123
- disabled_by: admin_id
- reason: "Suspicious activity detected"

**THEN** User account is marked as disabled (is_disabled=true, disabled_at=now)  
**AND** User is persisted to database  
**AND** all active sessions for this user are terminated  
**AND** all WebAuthn credentials are deactivated  
**AND** UserDisabled event is emitted  
**AND** audit log entry created  
**AND** HTTP response 200 OK

### AC2: Authorization - Only Super Admin Can Disable
**GIVEN** a regular Owner user is authenticated  
**WHEN** DisableUserCommand is executed by the Owner  
**THEN** command fails with `DomainError::Unauthorized`  
**AND** user remains enabled  
**AND** audit log entry "UnauthorizedUserDisable" created

### AC3: Validation - Cannot Disable Own Account
**GIVEN** a Super Admin is authenticated  
**WHEN** DisableUserCommand is executed with user_id = disabled_by  
**THEN** command fails with `DomainError::CannotDisableSelf`  
**AND** account remains enabled

### AC4: Validation - User Already Disabled
**GIVEN** a user that is already disabled  
**WHEN** DisableUserCommand is executed  
**THEN** command fails with `DomainError::UserAlreadyDisabled`  
**AND** HTTP response 409 Conflict

### AC5: Side Effect - All Sessions Terminated
**GIVEN** a user with 2 active sessions  
**WHEN** DisableUserCommand is executed  
**THEN** both sessions are terminated immediately  
**AND** sandboxes are destroyed  
**AND** SessionTerminated events emitted for each

---

## Handler Implementation

```rust
impl CommandHandler<DisableUserCommand> for DisableUserCommandHandler {
    async fn handle(&self, cmd: DisableUserCommand) -> Result<(), DomainError> {
        // 1. Verify Super Admin
        let admin = self.user_repository
            .find_by_id(&cmd.disabled_by)
            .await?
            .ok_or(DomainError::Unauthorized)?;
        
        if admin.role != UserRole::SuperAdmin {
            return Err(DomainError::Unauthorized);
        }
        
        // 2. Cannot disable self
        if cmd.user_id == cmd.disabled_by {
            return Err(DomainError::CannotDisableSelf);
        }
        
        // 3. Get user
        let mut user = self.user_repository
            .find_by_id(&cmd.user_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;
        
        // 4. Disable user (domain logic)
        user.disable(&cmd.reason)?;
        
        // 5. Persist
        self.user_repository.save(&user).await?;
        
        // 6. Terminate all active sessions
        let sessions = self.session_repository
            .find_active_by_user(&cmd.user_id)
            .await?;
        
        for session in sessions {
            self.command_bus.send(TerminateSessionCommand {
                session_id: session.id,
                terminated_by: cmd.disabled_by.clone(),
                reason: TerminationReason::UserDisabled,
            }).await?;
        }
        
        // 7. Emit event
        self.event_publisher.publish(DomainEvent::UserDisabled {
            user_id: cmd.user_id,
            disabled_by: cmd.disabled_by,
            reason: cmd.reason,
            timestamp: Utc::now(),
        }).await?;
        
        Ok(())
    }
}
```

---

## API Endpoint

```http
POST /api/admin/users/{user_id}/disable
Authorization: Bearer {super_admin_jwt_token}
Content-Type: application/json

Request Body:
{
  "reason": "Suspicious activity detected"
}

Response 200 OK:
{
  "success": true,
  "user_id": "usr_123",
  "disabled_at": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [EnableUserCommand](enable_user.md), [DeleteUserCommand](delete_user.md)
