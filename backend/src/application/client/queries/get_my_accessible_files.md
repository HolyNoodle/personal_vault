# GetMyAccessibleFilesQuery

**Purpose:** Client retrieves all files they have permission to access.

**Persona:** Client

**Module:** `application::client::queries::get_my_accessible_files`

---

## Query Structure

```rust
pub struct GetMyAccessibleFilesQuery {
    pub client_id: UserId,
    pub status_filter: Option<PermissionStatus>,  // Active, Expired, Revoked
    pub search: Option<String>,                    // Search by filename
    pub sort_by: SortField,
    pub sort_order: SortOrder,
    pub page: u32,
    pub page_size: u32,
}

pub enum PermissionStatus {
    Active,   // Not expired, not revoked
    Expired,
    Revoked,
}

pub enum SortField {
    FileName,
    GrantedAt,
    ExpiresAt,
    OwnerEmail,
}
```

---

## Response Structure

```rust
pub struct GetMyAccessibleFilesQueryResult {
    pub files: Vec<AccessibleFileSummary>,
    pub total_count: u64,
    pub page: u32,
    pub page_size: u32,
}

pub struct AccessibleFileSummary {
    pub file_id: FileId,
    pub file_name: String,
    pub file_size_bytes: u64,
    pub content_type: String,
    pub owner_id: UserId,
    pub owner_email: String,
    pub permission_id: PermissionId,
    pub permissions: PermissionSet,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub is_revoked: bool,
    pub has_active_session: bool,
}
```

---

## Acceptance Criteria

### AC1: Happy Path - Get All Accessible Files
**GIVEN** a Client has permissions for 10 files  
**WHEN** GetMyAccessibleFilesQuery is executed  
**THEN** query returns all 10 files  
**AND** each includes: file name, owner, permissions, expiration

### AC2: Filter By Status - Active Only
**GIVEN** a Client has 10 active permissions and 5 expired  
**WHEN** GetMyAccessibleFilesQuery is executed with status_filter=Active  
**THEN** query returns only 10 active permissions

### AC3: Search By Filename
**GIVEN** accessible files include "report_2025.pdf" and "contract.pdf"  
**WHEN** GetMyAccessibleFilesQuery is executed with search="report"  
**THEN** query returns only "report_2025.pdf"

### AC4: Active Session Indicator
**GIVEN** a Client has an active session for file_A  
**WHEN** GetMyAccessibleFilesQuery is executed  
**THEN** file_A has has_active_session=true

### AC5: Sort By Expiration - Expiring Soon First
**GIVEN** files with various expiration dates  
**WHEN** GetMyAccessibleFilesQuery is executed with sort_by=ExpiresAt, sort_order=Asc  
**THEN** files expiring soonest appear first

---

## API Endpoint

```http
GET /api/client/files/accessible?status=Active&search=contract&sort_by=expires_at&sort_order=asc&page=1&page_size=20
Authorization: Bearer {client_jwt_token}

Response 200 OK:
{
  "files": [
    {
      "file_id": "fil_123",
      "file_name": "contract.pdf",
      "file_size_bytes": 5242880,
      "content_type": "application/pdf",
      "owner_id": "usr_789",
      "owner_email": "owner@example.com",
      "permission_id": "prm_456",
      "permissions": {
        "read": true,
        "write": false,
        "execute": false
      },
      "granted_at": "2026-02-10T10:00:00Z",
      "expires_at": "2026-02-17T10:00:00Z",
      "is_active": true,
      "is_revoked": false,
      "has_active_session": false
    }
  ],
  "total_count": 10,
  "page": 1,
  "page_size": 20
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [StartSessionCommand](../commands/start_session.md)
