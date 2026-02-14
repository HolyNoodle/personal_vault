# UpdateUserQuotaCommand

**Purpose:** Super Admin modifies a user's storage quota.

**Persona:** Super Admin

**Module:** `application::super_admin::commands::update_user_quota`

---

## Command Structure

```rust
pub struct UpdateUserQuotaCommand {
    pub user_id: UserId,
    pub new_quota_bytes: u64,
    pub updated_by: UserId,  // Super Admin
}
```

---

## Validations

- ✅ user_id exists
- ✅ updated_by has SuperAdmin role
- ✅ new_quota_bytes > 0
- ✅ new_quota_bytes <= system max (1TB)
- ✅ new_quota_bytes >= user's current storage usage

---

## Acceptance Criteria

### AC1: Happy Path - Increase Quota
**GIVEN** a user with 10GB quota and 5GB used  
**WHEN** UpdateUserQuotaCommand is executed with new_quota_bytes = 50GB  
**THEN** User's quota is updated to 50GB  
**AND** UserQuotaUpdated event is emitted  
**AND** audit log entry created

### AC2: Happy Path - Decrease Quota (Within Usage)
**GIVEN** a user with 100GB quota and 30GB used  
**WHEN** UpdateUserQuotaCommand is executed with new_quota_bytes = 40GB  
**THEN** User's quota is updated to 40GB

### AC3: Validation - Cannot Decrease Below Current Usage
**GIVEN** a user with 100GB quota and 60GB used  
**WHEN** UpdateUserQuotaCommand is executed with new_quota_bytes = 50GB  
**THEN** command fails with `DomainError::QuotaBelowCurrentUsage`  
**AND** quota remains unchanged

### AC4: Validation - Zero Quota Rejected
**GIVEN** a Super Admin is authenticated  
**WHEN** UpdateUserQuotaCommand is executed with new_quota_bytes = 0  
**THEN** command fails with `DomainError::InvalidStorageQuota`

---

## Handler Implementation

```rust
impl CommandHandler<UpdateUserQuotaCommand> for UpdateUserQuotaCommandHandler {
    async fn handle(&self, cmd: UpdateUserQuotaCommand) -> Result<(), DomainError> {
        // 1. Get user
        let mut user = self.user_repository
            .find_by_id(&cmd.user_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;
        
        // 2. Check current usage
        let current_usage = self.file_repository
            .calculate_storage_usage(&cmd.user_id)
            .await?;
        
        if cmd.new_quota_bytes < current_usage {
            return Err(DomainError::QuotaBelowCurrentUsage {
                requested: cmd.new_quota_bytes,
                current_usage,
            });
        }
        
        // 3. Update quota
        user.update_quota(cmd.new_quota_bytes)?;
        
        // 4. Persist
        self.user_repository.save(&user).await?;
        
        // 5. Emit event
        self.event_publisher.publish(DomainEvent::UserQuotaUpdated {
            user_id: cmd.user_id,
            old_quota: user.storage_quota_bytes,
            new_quota: cmd.new_quota_bytes,
            updated_by: cmd.updated_by,
            timestamp: Utc::now(),
        }).await?;
        
        Ok(())
    }
}
```

---

## API Endpoint

```http
PUT /api/admin/users/{user_id}/quota
Authorization: Bearer {super_admin_jwt_token}
Content-Type: application/json

Request Body:
{
  "quota_gb": 50
}

Response 200 OK:
{
  "success": true,
  "user_id": "usr_123",
  "new_quota_bytes": 53687091200,
  "current_usage_bytes": 5368709120
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
