# GetActiveSessionsQuery

**Purpose:** Owner retrieves all active sessions accessing their files.

**Persona:** Owner

**Module:** `application::owner::queries::get_active_sessions`

---

## Query Structure

```rust
pub struct GetActiveSessionsQuery {
    pub owner_id: UserId,
    pub file_id_filter: Option<FileId>,  // Filter by specific file
    pub sort_by: SessionSortField,
    pub sort_order: SortOrder,
}

pub enum SessionSortField {
    StartedAt,
    Duration,
    FileName,
}
```

---

## Response Structure

```rust
pub struct GetActiveSessionsQueryResult {
    pub sessions: Vec<ActiveSessionSummary>,
    pub total_count: u64,
}

pub struct ActiveSessionSummary {
    pub session_id: SessionId,
    pub client_id: UserId,
    pub client_email: String,
    pub file_id: FileId,
    pub file_name: String,
    pub started_at: DateTime<Utc>,
    pub duration_seconds: u64,
    pub permissions: PermissionSet,
    pub ip_address: IpAddr,
    pub webrtc_connected: bool,
}
```

---

## Acceptance Criteria

### AC1: Happy Path - Get All Active Sessions
**GIVEN** an Owner has files with 10 active sessions  
**WHEN** GetActiveSessionsQuery is executed  
**THEN** query returns all 10 sessions  
**AND** each includes client info, file name, duration, permissions

### AC2: Filter By File
**GIVEN** 10 active sessions (3 for file fil_123)  
**WHEN** GetActiveSessionsQuery is executed with file_id_filter=fil_123  
**THEN** query returns only 3 sessions for that file

### AC3: Real-Time Duration
**GIVEN** a session started 30 minutes ago  
**WHEN** GetActiveSessionsQuery is executed  
**THEN** duration_seconds = 1800 (current time - started_at)

---

## API Endpoint

```http
GET /api/owner/sessions/active?file_id=fil_123&sort_by=duration&sort_order=desc
Authorization: Bearer {owner_jwt_token}

Response 200 OK:
{
  "sessions": [
    {
      "session_id": "ses_123",
      "client_id": "usr_456",
      "client_email": "client@example.com",
      "file_id": "fil_789",
      "file_name": "report.pdf",
      "started_at": "2026-02-14T09:00:00Z",
      "duration_seconds": 5400,
      "permissions": {
        "read": true,
        "write": false,
        "execute": false
      },
      "ip_address": "192.168.1.50",
      "webrtc_connected": true
    }
  ],
  "total_count": 10
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
