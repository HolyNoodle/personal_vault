# DeleteUserCommand

**Purpose:** Super Admin permanently deletes a user account and all associated data.

**Persona:** Super Admin

**Module:** `application::super_admin::commands::delete_user`

---

## Command Structure

```rust
pub struct DeleteUserCommand {
    pub user_id: UserId,
    pub deleted_by: UserId,      // Super Admin
    pub permanent: bool,          // If false, soft delete (30-day retention)
}
```

---

## Validations

- ✅ user_id exists
- ✅ deleted_by has SuperAdmin role
- ✅ cannot delete own account
- ✅ user must be disabled first (safety check)

---

## Acceptance Criteria

### AC1: Happy Path - Soft Delete User
**GIVEN** a disabled user exists  
**WHEN** DeleteUserCommand is executed with permanent=false  
**THEN** User is marked as deleted (is_deleted=true, deleted_at=now)  
**AND** all active sessions terminated  
**AND** all files moved to `.trash/`  
**AND** all permissions revoked  
**AND** UserDeleted event emitted  
**AND** data retained for 30 days

### AC2: Permanent Delete - Data Removed
**GIVEN** a disabled user exists  
**WHEN** DeleteUserCommand is executed with permanent=true  
**THEN** User record marked as permanently deleted  
**AND** all files physically deleted from filesystem  
**AND** all database records for user deleted (cascade)  
**AND** UserPermanentlyDeleted event emitted

### AC3: Validation - Cannot Delete Active User
**GIVEN** a user that is NOT disabled  
**WHEN** DeleteUserCommand is executed  
**THEN** command fails with `DomainError::UserMustBeDisabledFirst`  
**AND** no deletion occurs

### AC4: Validation - Cannot Delete Self
**GIVEN** a Super Admin is authenticated  
**WHEN** DeleteUserCommand is executed with user_id = deleted_by  
**THEN** command fails with `DomainError::CannotDeleteSelf`

### AC5: GDPR Compliance - All Data Removed
**GIVEN** a user with files, sessions, permissions, audit logs  
**WHEN** DeleteUserCommand is executed with permanent=true  
**THEN** ALL user data is removed:
- User record
- Files (filesystem and metadata)
- Sessions
- Permissions
- Invitations
- Audit logs (anonymized, not deleted)
- WebAuthn credentials

---

## Handler Implementation

```rust
impl CommandHandler<DeleteUserCommand> for DeleteUserCommandHandler {
    async fn handle(&self, cmd: DeleteUserCommand) -> Result<(), DomainError> {
        // 1. Get user
        let mut user = self.user_repository
            .find_by_id(&cmd.user_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;
        
        // 2. Must be disabled first
        if !user.is_disabled {
            return Err(DomainError::UserMustBeDisabledFirst);
        }
        
        // 3. Cannot delete self
        if cmd.user_id == cmd.deleted_by {
            return Err(DomainError::CannotDeleteSelf);
        }
        
        // 4. Terminate all sessions
        let sessions = self.session_repository
            .find_active_by_user(&cmd.user_id)
            .await?;
        for session in sessions {
            self.command_bus.send(TerminateSessionCommand {
                session_id: session.id,
                terminated_by: cmd.deleted_by.clone(),
                reason: TerminationReason::UserDeleted,
            }).await?;
        }
        
        // 5. Delete or move files
        if cmd.permanent {
            // Permanent delete
            self.filesystem.delete_user_folder(&cmd.user_id).await?;
            self.file_repository.delete_all_for_user(&cmd.user_id).await?;
        } else {
            // Soft delete - move to trash
            let trash_path = format!("/data/trash/{}", cmd.user_id);
            self.filesystem.move_folder(
                &format!("/data/users/{}", cmd.user_id),
                &trash_path
            ).await?;
        }
        
        // 6. Revoke all permissions (as owner and as client)
        self.permission_repository.delete_all_for_user(&cmd.user_id).await?;
        
        // 7. Delete user
        user.delete(cmd.permanent)?;
        self.user_repository.save(&user).await?;
        
        // 8. Emit event
        let event = if cmd.permanent {
            DomainEvent::UserPermanentlyDeleted {
                user_id: cmd.user_id,
                deleted_by: cmd.deleted_by,
                timestamp: Utc::now(),
            }
        } else {
            DomainEvent::UserDeleted {
                user_id: cmd.user_id,
                deleted_by: cmd.deleted_by,
                timestamp: Utc::now(),
            }
        };
        self.event_publisher.publish(event).await?;
        
        Ok(())
    }
}
```

---

## API Endpoint

```http
DELETE /api/admin/users/{user_id}?permanent=false
Authorization: Bearer {super_admin_jwt_token}

Response 200 OK:
{
  "success": true,
  "user_id": "usr_123",
  "deleted_at": "2026-02-14T10:30:00Z",
  "permanent": false,
  "retention_days": 30
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [DisableUserCommand](disable_user.md)
