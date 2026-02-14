# GetFilePermissionsQuery

**Purpose:** Owner retrieves all permissions granted for a specific file.

**Persona:** Owner

**Module:** `application::owner::queries::get_file_permissions`

---

## Query Structure

```rust
pub struct GetFilePermissionsQuery {
    pub file_id: FileId,
    pub owner_id: UserId,
    pub include_expired: bool,  // Include expired permissions
}
```

---

## Response Structure

```rust
pub struct GetFilePermissionsQueryResult {
    pub permissions: Vec<PermissionSummary>,
    pub total_count: u64,
}

pub struct PermissionSummary {
    pub permission_id: PermissionId,
    pub client_id: UserId,
    pub client_email: String,
    pub permissions: PermissionSet,
    pub granted_at: DateTime<Utc>,
    pub granted_by: UserId,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub revoked_at: Option<DateTime<Utc>>,
    pub current_active_sessions: u32,
}
```

---

## Acceptance Criteria

### AC1: Happy Path - Get File Permissions
**GIVEN** a file has 5 active permissions and 2 revoked  
**WHEN** GetFilePermissionsQuery is executed with include_expired=false  
**THEN** query returns 5 active permissions  
**AND** each includes client info, permissions, expiration

### AC2: Include Expired - Show All Permissions
**GIVEN** a file has permissions (active, expired, revoked)  
**WHEN** GetFilePermissionsQuery is executed with include_expired=true  
**THEN** query returns all permissions regardless of status

### AC3: Authorization - Owner Only
**GIVEN** a file belongs to user_A  
**WHEN** user_B executes GetFilePermissionsQuery  
**THEN** query fails with `DomainError::Unauthorized`

---

## API Endpoint

```http
GET /api/owner/files/{file_id}/permissions?include_expired=false
Authorization: Bearer {owner_jwt_token}

Response 200 OK:
{
  "permissions": [
    {
      "permission_id": "prm_123",
      "client_id": "usr_456",
      "client_email": "client@example.com",
      "permissions": {
        "read": true,
        "write": false,
        "execute": false
      },
      "granted_at": "2026-02-10T10:00:00Z",
      "granted_by": "usr_789",
      "expires_at": "2026-02-17T10:00:00Z",
      "is_active": true,
      "revoked_at": null,
      "current_active_sessions": 1
    }
  ],
  "total_count": 5
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
