# CancelAccessRequestCommand

**Purpose:** Client cancels their pending access request.

**Persona:** Client

**Module:** `application::client::commands::cancel_access_request`

---

## Command Structure

```rust
pub struct CancelAccessRequestCommand {
    pub access_request_id: AccessRequestId,
    pub client_id: UserId,
}
```

---

## Validations

- ✅ access_request_id exists
- ✅ access_request belongs to client_id
- ✅ access_request status is Pending

---

## Acceptance Criteria

### AC1: Happy Path - Cancel Access Request
**GIVEN** a Client has a pending access request  
**WHEN** CancelAccessRequestCommand is executed  
**THEN** AccessRequest status is updated to Cancelled  
**AND** Owner receives WebSocket notification  
**AND** AccessRequestCancelled event emitted  
**AND** audit log entry created  
**AND** HTTP response 200 OK

### AC2: Already Approved - Cannot Cancel
**GIVEN** an access request that has been approved  
**WHEN** CancelAccessRequestCommand is executed  
**THEN** command fails with `DomainError::AccessRequestAlreadyProcessed`  
**AND** status remains Approved

### AC3: Authorization - Client Can Only Cancel Own Request
**GIVEN** an access request belongs to client_A  
**WHEN** client_B executes CancelAccessRequestCommand  
**THEN** command fails with `DomainError::Unauthorized`

---

## API Endpoint

```http
DELETE /api/client/access-requests/{access_request_id}
Authorization: Bearer {client_jwt_token}

Response 200 OK:
{
  "success": true,
  "access_request_id": "req_789",
  "cancelled_at": "2026-02-14T10:35:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
