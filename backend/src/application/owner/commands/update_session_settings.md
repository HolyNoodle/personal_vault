# UpdateSessionSettingsCommand

**Purpose:** Owner modifies session settings (time limits, permissions during active session).

**Persona:** Owner

**Module:** `application::owner::commands::update_session_settings`

---

## Command Structure

```rust
pub struct UpdateSessionSettingsCommand {
    pub session_id: SessionId,
    pub owner_id: UserId,
    pub new_max_duration_seconds: Option<u64>,  // Extend/reduce session time
    pub new_permissions: Option<PermissionSet>, // Modify access level
}
```

---

## Validations

- ✅ session_id exists and belongs to owner's file
- ✅ session is active
- ✅ new_max_duration_seconds is reasonable (max 8 hours)

---

## Acceptance Criteria

### AC1: Happy Path - Extend Session Duration
**GIVEN** a session with max_duration=1 hour and 30 minutes remaining  
**WHEN** UpdateSessionSettingsCommand is executed with new_max_duration_seconds=7200 (2 hours)  
**THEN** Session max duration is updated  
**AND** Session continues without interruption  
**AND** SessionSettingsUpdated event emitted

### AC2: Reduce Duration - Session May Terminate
**GIVEN** a session running for 90 minutes with max_duration=2 hours  
**WHEN** UpdateSessionSettingsCommand is executed with new_max_duration_seconds=3600 (1 hour)  
**THEN** Session is terminated immediately (exceeded new limit)  
**AND** Client is notified

### AC3: Update Permissions - Landlock Policy Updated
**GIVEN** a session with read-only access  
**WHEN** UpdateSessionSettingsCommand is executed with new_permissions=ReadWrite  
**THEN** Landlock LSM policy is updated (~2 seconds)  
**AND** Client can now write files

---

## API Endpoint

```http
PATCH /api/owner/sessions/{session_id}/settings
Authorization: Bearer {owner_jwt_token}
Content-Type: application/json

Request Body:
{
  "max_duration_seconds": 7200,
  "permissions": {
    "read": true,
    "write": true,
    "execute": false
  }
}

Response 200 OK:
{
  "success": true,
  "session_id": "ses_123",
  "updated_at": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
