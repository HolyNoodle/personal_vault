# TerminateSessionCommand

**Purpose:** Owner forcefully terminates a client's active session.

**Persona:** Owner

**Module:** `application::owner::commands::terminate_session`

---

## Command Structure

```rust
pub struct TerminateSessionCommand {
    pub session_id: SessionId,
    pub owner_id: UserId,
    pub reason: TerminationReason,
}

pub enum TerminationReason {
    OwnerRequest,
    SecurityConcern,
    FileDeleted,
    UserDisabled,
    PermissionRevoked,
}
```

---

## Validations

- ✅ session_id exists
- ✅ session belongs to a file owned by owner_id
- ✅ session is active

---

## Acceptance Criteria

### AC1: Happy Path - Terminate Session
**GIVEN** an Owner has a file with an active client session  
**WHEN** TerminateSessionCommand is executed  
**THEN** Session is terminated immediately  
**AND** Sandbox is destroyed  
**AND** Landlock policy removed  
**AND** WebRTC connection closed  
**AND** SessionTerminated event emitted  
**AND** audit log entry created

### AC2: Authorization - Owner Only
**GIVEN** a session accessing user_A's file  
**WHEN** user_B executes TerminateSessionCommand  
**THEN** command fails with `DomainError::Unauthorized`

---

## API Endpoint

```http
DELETE /api/owner/sessions/{session_id}
Authorization: Bearer {owner_jwt_token}
Content-Type: application/json

Request Body:
{
  "reason": "OwnerRequest"
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
**Related:** [ForceTerminateSessionCommand](../../super_admin/commands/force_terminate_session.md)
