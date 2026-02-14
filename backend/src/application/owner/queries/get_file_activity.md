# GetFileActivityQuery

**Purpose:** Owner retrieves activity log for a specific file (who accessed, when, actions).

**Persona:** Owner

**Module:** `application::owner::queries::get_file_activity`

---

## Query Structure

```rust
pub struct GetFileActivityQuery {
    pub file_id: FileId,
    pub owner_id: UserId,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub activity_type_filter: Option<Vec<FileActivityType>>,
    pub page: u32,
    pub page_size: u32,
}

pub enum FileActivityType {
    SessionStarted,
    SessionEnded,
    PermissionGranted,
    PermissionRevoked,
    FileRenamed,
    FileMoved,
    ShareCreated,
    AccessRequested,
}
```

---

## Response Structure

```rust
pub struct GetFileActivityQueryResult {
    pub activities: Vec<FileActivity>,
    pub total_count: u64,
    pub page: u32,
    pub page_size: u32,
}

pub struct FileActivity {
    pub activity_id: String,
    pub timestamp: DateTime<Utc>,
    pub activity_type: FileActivityType,
    pub actor_id: Option<UserId>,
    pub actor_email: Option<String>,
    pub description: String,
    pub metadata: serde_json::Value,
}
```

---

## Acceptance Criteria

### AC1: Happy Path - Get File Activity
**GIVEN** a file with various activities  
**WHEN** GetFileActivityQuery is executed  
**THEN** query returns activities ordered by timestamp desc (most recent first)  
**AND** each activity includes: type, actor, description, metadata

### AC2: Filter By Date Range
**GIVEN** file activity over past 30 days  
**WHEN** GetFileActivityQuery is executed with start_date=7 days ago  
**THEN** query returns only activities from last 7 days

### AC3: Filter By Activity Type
**GIVEN** various activity types  
**WHEN** GetFileActivityQuery is executed with activity_type_filter=[SessionStarted, SessionEnded]  
**THEN** query returns only session-related activities

### AC4: Authorization - Owner Only
**GIVEN** a file belongs to user_A  
**WHEN** user_B executes GetFileActivityQuery  
**THEN** query fails with `DomainError::Unauthorized`

---

## API Endpoint

```http
GET /api/owner/files/{file_id}/activity?start_date=2026-02-07T00:00:00Z&activity_type=SessionStarted&page=1&page_size=50
Authorization: Bearer {owner_jwt_token}

Response 200 OK:
{
  "activities": [
    {
      "activity_id": "act_123",
      "timestamp": "2026-02-14T09:30:00Z",
      "activity_type": "SessionStarted",
      "actor_id": "usr_456",
      "actor_email": "client@example.com",
      "description": "Client started viewing session",
      "metadata": {
        "session_id": "ses_789",
        "ip_address": "192.168.1.50",
        "permissions": { "read": true, "write": false }
      }
    }
  ],
  "total_count": 150,
  "page": 1,
  "page_size": 50
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
