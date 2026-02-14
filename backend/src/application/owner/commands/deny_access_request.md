# DenyAccessRequestCommand

**Purpose:** Owner denies a client's request to access a file.

**Persona:** Owner

**Module:** `application::owner::commands::deny_access_request`

---

## Command Structure

```rust
pub struct DenyAccessRequestCommand {
    pub access_request_id: AccessRequestId,
    pub owner_id: UserId,
    pub reason: Option<String>,
}
```

---

## Validations

- ✅ access_request_id exists
- ✅ access request belongs to file owned by owner_id
- ✅ access request status is Pending

---

## Acceptance Criteria

### AC1: Happy Path - Deny Access Request
**GIVEN** a client has submitted an access request  
**WHEN** DenyAccessRequestCommand is executed  
**THEN** Access request status is updated to Denied  
**AND** No permission is created  
**AND** AccessRequestDenied event emitted  
**AND** audit log entry created

---

## API Endpoint

```http
POST /api/owner/access-requests/{access_request_id}/deny
Authorization: Bearer {owner_jwt_token}
Content-Type: application/json

Request Body:
{
  "reason": "File no longer available"
}

Response 200 OK:
{
  "success": true,
  "access_request_id": "req_123",
  "denied_at": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
