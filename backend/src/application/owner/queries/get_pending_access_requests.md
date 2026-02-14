# GetPendingAccessRequestsQuery

**Purpose:** Owner retrieves all pending access requests requiring approval.

**Persona:** Owner

**Module:** `application::owner::queries::get_pending_access_requests`

---

## Query Structure

```rust
pub struct GetPendingAccessRequestsQuery {
    pub owner_id: UserId,
    pub file_id_filter: Option<FileId>,
}
```

---

## Response Structure

```rust
pub struct GetPendingAccessRequestsQueryResult {
    pub requests: Vec<AccessRequestSummary>,
    pub total_count: u64,
}

pub struct AccessRequestSummary {
    pub access_request_id: AccessRequestId,
    pub requester_id: UserId,
    pub requester_email: String,
    pub file_id: FileId,
    pub file_name: String,
    pub requested_permissions: PermissionSet,
    pub requested_duration_seconds: u64,
    pub message: Option<String>,        // Client's reason for access
    pub requested_at: DateTime<Utc>,
    pub status: AccessRequestStatus,
}

pub enum AccessRequestStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}
```

---

## Acceptance Criteria

### AC1: Happy Path - Get Pending Requests
**GIVEN** an Owner has 5 pending access requests  
**WHEN** GetPendingAccessRequestsQuery is executed  
**THEN** query returns 5 pending requests  
**AND** each includes requester info, file name, requested permissions, message

### AC2: Filter By File
**GIVEN** pending requests for multiple files  
**WHEN** GetPendingAccessRequestsQuery is executed with file_id_filter=fil_123  
**THEN** query returns only requests for that file

### AC3: Sorted By Date - Oldest First
**GIVEN** pending requests from different dates  
**THEN** query returns requests sorted by requested_at ASC (oldest first for review)

---

## API Endpoint

```http
GET /api/owner/access-requests/pending?file_id=fil_123
Authorization: Bearer {owner_jwt_token}

Response 200 OK:
{
  "requests": [
    {
      "access_request_id": "req_123",
      "requester_id": "usr_456",
      "requester_email": "client@example.com",
      "file_id": "fil_789",
      "file_name": "contract.pdf",
      "requested_permissions": {
        "read": true,
        "write": false,
        "execute": false
      },
      "requested_duration_seconds": 3600,
      "message": "Need to review contract for legal approval",
      "requested_at": "2026-02-13T14:30:00Z",
      "status": "Pending"
    }
  ],
  "total_count": 5
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [ApproveAccessRequestCommand](../commands/approve_access_request.md)
