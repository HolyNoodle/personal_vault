# GetActiveSessionsQuery

**Purpose:** Super Admin retrieves all active sessions across all users.

**Persona:** Super Admin

**Module:** `application::super_admin::queries::get_active_sessions`

---

## Query Structure

```rust
pub struct GetActiveSessionsQuery {
    pub user_id_filter: Option<UserId>,     // Filter by specific user
    pub owner_id_filter: Option<UserId>,    // Filter by file owner
    pub sort_by: SessionSortField,          // StartedAt, Duration, UserId
    pub sort_order: SortOrder,
    pub page: u32,
    pub page_size: u32,
}

pub enum SessionSortField {
    StartedAt,
    Duration,
    UserId,
    OwnerId,
}
```

---

## Response Structure

```rust
pub struct GetActiveSessionsQueryResult {
    pub sessions: Vec<ActiveSessionSummary>,
    pub total_count: u64,
    pub page: u32,
    pub page_size: u32,
}

pub struct ActiveSessionSummary {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub user_email: String,
    pub owner_id: UserId,
    pub owner_email: String,
    pub file_id: FileId,
    pub file_path: String,
    pub started_at: DateTime<Utc>,
    pub duration_seconds: u64,
    pub sandbox_id: String,
    pub webrtc_connected: bool,
    pub ip_address: IpAddr,
}
```

---

## Acceptance Criteria

### AC1: Happy Path - Get All Active Sessions
**GIVEN** a Super Admin is authenticated  
**AND** 25 active sessions exist  
**WHEN** GetActiveSessionsQuery is executed  
**THEN** query returns all 25 active sessions  
**AND** each session includes: user, owner, file, duration, sandbox ID

### AC2: Filter By User
**GIVEN** 25 active sessions (5 for user usr_123)  
**WHEN** GetActiveSessionsQuery is executed with user_id_filter=usr_123  
**THEN** query returns 5 sessions for that user

### AC3: Sort By Duration
**GIVEN** active sessions with various durations  
**WHEN** GetActiveSessionsQuery is executed with sort_by=Duration, sort_order=Desc  
**THEN** query returns sessions ordered by longest running first

---

## API Endpoint

```http
GET /api/admin/sessions/active?user_id=usr_123&sort_by=duration&sort_order=desc&page=1&page_size=50
Authorization: Bearer {super_admin_jwt_token}

Response 200 OK:
{
  "sessions": [
    {
      "session_id": "ses_123",
      "user_id": "usr_456",
      "user_email": "client@example.com",
      "owner_id": "usr_123",
      "owner_email": "owner@example.com",
      "file_id": "fil_789",
      "file_path": "/Documents/report.pdf",
      "started_at": "2026-02-14T08:00:00Z",
      "duration_seconds": 9000,
      "sandbox_id": "sandbox_abc123",
      "webrtc_connected": true,
      "ip_address": "192.168.1.50"
    }
  ],
  "total_count": 25,
  "page": 1,
  "page_size": 50
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
