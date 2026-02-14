# ApproveAccessRequestCommand

**Purpose:** Owner approves a client's request to access a file.

**Persona:** Owner

**Module:** `application::owner::commands::approve_access_request`

---

## Command Structure

```rust
pub struct ApproveAccessRequestCommand {
    pub access_request_id: AccessRequestId,
    pub owner_id: UserId,
    pub granted_permissions: PermissionSet,
    pub max_duration_seconds: u64,
}
```

---

## Validations

- ✅ access_request_id exists
- ✅ access request belongs to file owned by owner_id
- ✅ access request status is Pending
- ✅ granted_permissions are valid
- ✅ max_duration_seconds <= system max (8 hours)

---

## Acceptance Criteria

### AC1: Happy Path - Approve Access Request
**GIVEN** a client has submitted an access request  
**WHEN** ApproveAccessRequestCommand is executed  
**THEN** Access request status is updated to Approved  
**AND** Permission is created for the client  
**AND** Client can now start a session  
**AND** AccessRequestApproved event emitted  
**AND** audit log entry created

### AC2: Permission Created Automatically
**GIVEN** an approved access request  
**THEN** Permission entity is created with:
- client_id = access_request.requester_id
- file_id = access_request.file_id
- permissions = granted_permissions
- expires_at = now + max_duration_seconds

---

## API Endpoint

```http
POST /api/owner/access-requests/{access_request_id}/approve
Authorization: Bearer {owner_jwt_token}
Content-Type: application/json

Request Body:
{
  "granted_permissions": {
    "read": true,
    "write": false,
    "execute": false
  },
  "max_duration_seconds": 3600
}

Response 200 OK:
{
  "success": true,
  "access_request_id": "req_123",
  "approved_at": "2026-02-14T10:30:00Z",
  "permission_id": "prm_456"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [DenyAccessRequestCommand](deny_access_request.md), [GrantPermissionCommand](grant_permission.md)
