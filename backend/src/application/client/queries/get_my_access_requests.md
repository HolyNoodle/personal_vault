# GetMyAccessRequestsQuery

**Purpose:** Client retrieves all their access requests (pending, approved, denied).

**Persona:** Client

**Module:** `application::client::queries::get_my_access_requests`

---

## Query Structure

```rust
pub struct GetMyAccessRequestsQuery {
    pub client_id: UserId,
    pub status_filter: Option<AccessRequestStatus>,
    pub sort_order: SortOrder,
    pub page: u32,
    pub page_size: u32,
}

pub enum AccessRequestStatus {
    Pending,
    Approved,
    Denied,
    Cancelled,
    Expired,
}
```

---

## Response Structure

```rust
pub struct GetMyAccessRequestsQueryResult {
    pub requests: Vec<AccessRequestSummary>,
    pub total_count: u64,
    pub page: u32,
    pub page_size: u32,
}

pub struct AccessRequestSummary {
    pub access_request_id: AccessRequestId,
    pub file_id: FileId,
    pub file_name: String,
    pub owner_email: String,
    pub requested_permissions: PermissionSet,
    pub requested_duration_seconds: u64,
    pub message: Option<String>,
    pub status: AccessRequestStatus,
    pub requested_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub permission_id: Option<PermissionId>,  // If approved
}
```

---

## Acceptance Criteria

### AC1: Happy Path - Get All Access Requests
**GIVEN** a Client has 5 access requests (2 pending, 2 approved, 1 denied)  
**WHEN** GetMyAccessRequestsQuery is executed  
**THEN** query returns all 5 requests  
**AND** sorted by requested_at desc (most recent first)

### AC2: Filter By Status - Pending Only
**GIVEN** multiple access requests with different statuses  
**WHEN** GetMyAccessRequestsQuery is executed with status_filter=Pending  
**THEN** query returns only pending requests

### AC3: Approved Request - Includes Permission ID
**GIVEN** an approved access request  
**WHEN** GetMyAccessRequestsQuery is executed  
**THEN** approved request includes permission_id  
**AND** client can use permission to start session

---

## API Endpoint

```http
GET /api/client/access-requests?status=Pending&page=1&page_size=20
Authorization: Bearer {client_jwt_token}

Response 200 OK:
{
  "requests": [
    {
      "access_request_id": "req_123",
      "file_id": "fil_789",
      "file_name": "contract.pdf",
      "owner_email": "owner@example.com",
      "requested_permissions": {
        "read": true,
        "write": false,
        "execute": false
      },
      "requested_duration_seconds": 3600,
      "message": "Need to review contract",
      "status": "Pending",
      "requested_at": "2026-02-14T09:00:00Z",
      "processed_at": null,
      "permission_id": null
    }
  ],
  "total_count": 5,
  "page": 1,
  "page_size": 20
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [RequestAccessCommand](../commands/request_access.md)
