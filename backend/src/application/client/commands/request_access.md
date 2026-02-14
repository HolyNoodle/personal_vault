# RequestAccessCommand

**Purpose:** Client requests permission to access a file owned by another user.

**Persona:** Client

**Module:** `application::client::commands::request_access`

---

## Command Structure

```rust
pub struct RequestAccessCommand {
    pub client_id: UserId,
    pub file_id: FileId,
    pub requested_permissions: PermissionSet,
    pub requested_duration_seconds: u64,
    pub message: Option<String>,  // Reason for access request
}
```

---

## Validations

- ✅ client_id is authenticated Client
- ✅ file_id exists
- ✅ client doesn't already have active permission for this file
- ✅ no pending access request exists for this client+file
- ✅ requested_duration_seconds <= system max (8 hours = 28800)
- ✅ message is <= 500 characters (if provided)
- ✅ file is not deleted

---

## Acceptance Criteria

### AC1: Happy Path - Submit Access Request
**GIVEN** a Client knows a file_id they want to access  
**AND** Client does not have permission for this file  
**WHEN** RequestAccessCommand is executed with:
- file_id: fil_123
- requested_permissions: ReadOnly
- requested_duration_seconds: 3600 (1 hour)
- message: "Need to review contract for legal approval"

**THEN** AccessRequest is created with status=Pending  
**AND** AccessRequest is persisted to database  
**AND** Owner receives WebSocket notification  
**AND** Owner receives email notification  
**AND** AccessRequestCreated event emitted  
**AND** audit log entry created  
**AND** HTTP response 201 Created with access_request_id

### AC2: Duplicate Request - Rejected
**GIVEN** a Client already has a pending access request for file_A  
**WHEN** RequestAccessCommand is executed for file_A  
**THEN** command fails with `DomainError::AccessRequestAlreadyPending`

### AC3: Already Has Permission - Rejected
**GIVEN** a Client already has active permission for a file  
**WHEN** RequestAccessCommand is executed  
**THEN** command fails with `DomainError::PermissionAlreadyExists`  
**AND** client should use existing permission

### AC4: WebSocket Notification - Owner Informed
**GIVEN** a Client submits an access request  
**WHEN** RequestAccessCommand is executed  
**THEN** Owner receives WebSocket message:
```json
{
  "type": "AccessRequestReceived",
  "access_request_id": "req_123",
  "requester_email": "client@example.com",
  "file_name": "contract.pdf",
  "requested_permissions": { "read": true, "write": false },
  "message": "Need to review contract for legal approval",
  "requested_at": "2026-02-14T10:30:00Z"
}
```

### AC5: Email Notification - Owner Notified
**GIVEN** a Client submits an access request  
**WHEN** RequestAccessCommand is executed  
**THEN** email is sent to file owner with:
- Subject: "Access request for contract.pdf"
- Requester email
- Requested permissions
- Client's message
- Link to approve/deny

### AC6: File Discovery Protection - No File Enumeration
**GIVEN** a file_id that doesn't exist or is deleted  
**WHEN** RequestAccessCommand is executed  
**THEN** command fails with `DomainError::FileNotFound`  
**AND** no information is leaked about file existence

---

## Handler Implementation

```rust
impl CommandHandler<RequestAccessCommand> for RequestAccessCommandHandler {
    async fn handle(&self, cmd: RequestAccessCommand) -> Result<AccessRequestId, DomainError> {
        // 1. Get file
        let file = self.file_repository
            .find_by_id(&cmd.file_id)
            .await?
            .ok_or(DomainError::FileNotFound)?;
        
        if file.is_deleted {
            return Err(DomainError::FileNotFound);
        }
        
        // 2. Check for existing permission
        let existing_permission = self.permission_repository
            .find_active_by_client_and_file(&cmd.client_id, &cmd.file_id)
            .await?;
        
        if existing_permission.is_some() {
            return Err(DomainError::PermissionAlreadyExists);
        }
        
        // 3. Check for pending request
        let pending_request = self.access_request_repository
            .find_pending_by_client_and_file(&cmd.client_id, &cmd.file_id)
            .await?;
        
        if pending_request.is_some() {
            return Err(DomainError::AccessRequestAlreadyPending);
        }
        
        // 4. Validate duration
        if cmd.requested_duration_seconds > 28800 {  // 8 hours
            return Err(DomainError::InvalidDuration);
        }
        
        // 5. Create access request
        let access_request = AccessRequest::new(
            AccessRequestId::new(),
            cmd.client_id.clone(),
            file.owner_id.clone(),
            cmd.file_id.clone(),
            cmd.requested_permissions,
            cmd.requested_duration_seconds,
            cmd.message,
        )?;
        
        // 6. Persist
        self.access_request_repository.save(&access_request).await?;
        
        // 7. Get client email for notifications
        let client = self.user_repository.find_by_id(&cmd.client_id).await?;
        let client_email = client.map(|c| c.email.to_string()).unwrap_or_default();
        
        // 8. Notify owner via WebSocket
        self.websocket_service.notify_owner(
            &file.owner_id,
            WebSocketMessage::AccessRequestReceived {
                access_request_id: access_request.id.clone(),
                requester_email: client_email.clone(),
                file_name: file.name.clone(),
                requested_permissions: cmd.requested_permissions,
                message: cmd.message.clone(),
                requested_at: access_request.requested_at,
            }
        ).await?;
        
        // 9. Send email to owner
        self.email_service.send_access_request_email(
            &file.owner_id,
            &client_email,
            &file.name,
            &cmd.requested_permissions,
            cmd.message.as_deref(),
            &access_request.id,
        ).await?;
        
        // 10. Emit event
        self.event_publisher.publish(DomainEvent::AccessRequestCreated {
            access_request_id: access_request.id.clone(),
            client_id: cmd.client_id,
            owner_id: file.owner_id,
            file_id: cmd.file_id,
            requested_at: access_request.requested_at,
        }).await?;
        
        Ok(access_request.id)
    }
}
```

---

## API Endpoint

```http
POST /api/client/access-requests
Authorization: Bearer {client_jwt_token}
Content-Type: application/json

Request Body:
{
  "file_id": "fil_123",
  "requested_permissions": {
    "read": true,
    "write": false,
    "execute": false
  },
  "requested_duration_seconds": 3600,
  "message": "Need to review contract for legal approval"
}

Response 201 Created:
{
  "access_request_id": "req_789",
  "status": "Pending",
  "file_name": "contract.pdf",
  "owner_email": "owner@example.com",
  "requested_at": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [CancelAccessRequestCommand](cancel_access_request.md), [ApproveAccessRequestCommand](../../owner/commands/approve_access_request.md)
