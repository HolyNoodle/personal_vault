# EnableUserCommand

**Purpose:** Super Admin re-enables a previously disabled user account.

**Persona:** Super Admin

**Module:** `application::super_admin::commands::enable_user`

---

## Command Structure

```rust
pub struct EnableUserCommand {
    pub user_id: UserId,
    pub enabled_by: UserId,  // Super Admin
}
```

---

## Validations

- ✅ user_id exists
- ✅ enabled_by has SuperAdmin role
- ✅ user is currently disabled

---

## Acceptance Criteria

### AC1: Happy Path - Enable User Successfully
**GIVEN** a Super Admin is authenticated  
**AND** a disabled user exists  
**WHEN** EnableUserCommand is executed  
**THEN** User account is marked as enabled (is_disabled=false, enabled_at=now)  
**AND** User is persisted to database  
**AND** UserEnabled event is emitted  
**AND** audit log entry created

### AC2: Validation - User Not Disabled
**GIVEN** a user that is already enabled  
**WHEN** EnableUserCommand is executed  
**THEN** command fails with `DomainError::UserNotDisabled`  
**AND** HTTP response 400 Bad Request

---

## API Endpoint

```http
POST /api/admin/users/{user_id}/enable
Authorization: Bearer {super_admin_jwt_token}

Response 200 OK:
{
  "success": true,
  "user_id": "usr_123",
  "enabled_at": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
