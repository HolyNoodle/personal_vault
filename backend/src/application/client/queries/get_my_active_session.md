# GetMyActiveSessionQuery

**Purpose:** Client retrieves their current active session (if any).

**Persona:** Client

**Module:** `application::client::queries::get_my_active_session`

---

## Query Structure

```rust
pub struct GetMyActiveSessionQuery {
    pub client_id: UserId,
}
```

---

## Response Structure

```rust
pub struct GetMyActiveSessionQueryResult {
    pub session: Option<ActiveSessionDetails>,
}

pub struct ActiveSessionDetails {
    pub session_id: SessionId,
    pub file_id: FileId,
    pub file_name: String,
    pub owner_email: String,
    pub permissions: PermissionSet,
    pub started_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub duration_seconds: u64,
    pub remaining_seconds: u64,
    pub sandbox_id: String,
    pub webrtc_connected: bool,
}
```

---

## Acceptance Criteria

### AC1: Active Session - Return Details
**GIVEN** a Client has an active session  
**WHEN** GetMyActiveSessionQuery is executed  
**THEN** query returns session details  
**AND** includes: file name, duration, remaining time, WebRTC status

### AC2: No Active Session - Return None
**GIVEN** a Client has no active session  
**WHEN** GetMyActiveSessionQuery is executed  
**THEN** query returns session=null

### AC3: Real-Time Calculations
**GIVEN** a session started 30 minutes ago with 1 hour max duration  
**WHEN** GetMyActiveSessionQuery is executed  
**THEN** duration_seconds = 1800  
**AND** remaining_seconds = 1800

---

## API Endpoint

```http
GET /api/client/sessions/active
Authorization: Bearer {client_jwt_token}

Response 200 OK (with active session):
{
  "session": {
    "session_id": "ses_123",
    "file_id": "fil_789",
    "file_name": "contract.pdf",
    "owner_email": "owner@example.com",
    "permissions": {
      "read": true,
      "write": false,
      "execute": false
    },
    "started_at": "2026-02-14T10:00:00Z",
    "expires_at": "2026-02-14T11:00:00Z",
    "duration_seconds": 1800,
    "remaining_seconds": 1800,
    "sandbox_id": "sandbox_abc123",
    "webrtc_connected": true
  }
}

Response 200 OK (no active session):
{
  "session": null
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [StartSessionCommand](../commands/start_session.md)
