# ForceTerminateSessionCommand

**Purpose:** Super Admin forcefully terminates any user's active session.

**Persona:** Super Admin

**Module:** `application::super_admin::commands::force_terminate_session`

---

## Command Structure

```rust
pub struct ForceTerminateSessionCommand {
    pub session_id: SessionId,
    pub terminated_by: UserId,   // Super Admin
    pub reason: String,
}
```

---

## Validations

- ✅ session_id exists
- ✅ terminated_by has SuperAdmin role
- ✅ session is active
- ✅ reason is not empty

---

## Acceptance Criteria

### AC1: Happy Path - Force Terminate Any Session
**GIVEN** a Super Admin is authenticated  
**AND** a Client has an active session  
**WHEN** ForceTerminateSessionCommand is executed  
**THEN** Session is terminated immediately  
**AND** Sandbox is destroyed  
**AND** WebRTC connection closed  
**AND** SessionTerminated event emitted with reason "AdminAction"  
**AND** Client is notified via WebSocket  
**AND** audit log entry created

### AC2: Authorization - Only Super Admin Can Force Terminate
**GIVEN** a regular Owner user  
**WHEN** ForceTerminateSessionCommand is executed  
**THEN** command fails with `DomainError::Unauthorized`

---

## API Endpoint

```http
DELETE /api/admin/sessions/{session_id}
Authorization: Bearer {super_admin_jwt_token}
Content-Type: application/json

Request Body:
{
  "reason": "Security incident - suspected data exfiltration"
}

Response 200 OK:
{
  "success": true,
  "session_id": "ses_123",
  "terminated_at": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
