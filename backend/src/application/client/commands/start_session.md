# StartSessionCommand

**Purpose:** Client starts a viewing session for a file they have permission to access.

**Persona:** Client

**Module:** `application::client::commands::start_session`

---

## Command Structure

```rust
pub struct StartSessionCommand {
    pub client_id: UserId,
    pub file_id: FileId,
    pub ip_address: IpAddr,
    pub user_agent: String,
}
```

---

## Validations

- ✅ client_id is authenticated Client
- ✅ file_id exists
- ✅ client has active (non-expired, non-revoked) permission for file
- ✅ client doesn't already have an active session for this file
- ✅ file is not deleted

---

## Acceptance Criteria

### AC1: Happy Path - Start Session Successfully
**GIVEN** a Client has permission to access a file  
**AND** no active session exists for this client+file  
**WHEN** StartSessionCommand is executed  
**THEN** Session is created with unique session_id  
**AND** Sandbox is created with Landlock LSM policy  
**AND** WebRTC connection is established  
**AND** Session state is Active  
**AND** SessionStarted event emitted  
**AND** Owner receives WebSocket notification  
**AND** audit log entry created  
**AND** HTTP response 201 Created with session_id and WebRTC SDP

### AC2: Permission Check - Unauthorized Access Rejected
**GIVEN** a Client does not have permission for a file  
**WHEN** StartSessionCommand is executed  
**THEN** command fails with `DomainError::PermissionDenied`  
**AND** no session is created  
**AND** audit log entry "UnauthorizedSessionAttempt" created

### AC3: Permission Expired - Access Rejected
**GIVEN** a Client had permission that expired 1 hour ago  
**WHEN** StartSessionCommand is executed  
**THEN** command fails with `DomainError::PermissionExpired`  
**AND** no session is created

### AC4: Permission Revoked - Access Rejected
**GIVEN** a Client's permission was revoked  
**WHEN** StartSessionCommand is executed  
**THEN** command fails with `DomainError::PermissionRevoked`  
**AND** no session is created

### AC5: Duplicate Session - Rejected
**GIVEN** a Client already has an active session for file_A  
**WHEN** StartSessionCommand is executed for file_A  
**THEN** command fails with `DomainError::SessionAlreadyActive`  
**AND** existing session remains active

### AC6: Sandbox Creation - Landlock Policy Applied
**GIVEN** a Client starts a session with read-only permission  
**WHEN** StartSessionCommand is executed  
**THEN** Sandbox is created with Landlock LSM policy:
- Allowed paths: `/data/users/{owner_id}/files/{file_id}`
- Allowed operations: READ_FILE
- Network: Disabled
- System calls: Restricted to file access only
**AND** sandbox_id is stored in session

### AC7: WebSocket Notification - Owner Informed
**GIVEN** a Client starts a session  
**WHEN** StartSessionCommand is executed  
**THEN** Owner receives WebSocket message:
```json
{
  "type": "SessionStarted",
  "session_id": "ses_123",
  "client_email": "client@example.com",
  "file_name": "report.pdf",
  "started_at": "2026-02-14T10:30:00Z",
  "ip_address": "192.168.1.50"
}
```

### AC8: Session Timeout - Auto-Termination Scheduled
**GIVEN** permission has max_duration_seconds=3600 (1 hour)  
**WHEN** StartSessionCommand is executed  
**THEN** Session is scheduled to auto-terminate after 1 hour  
**AND** session.expires_at = now + 1 hour

---

## Handler Implementation

```rust
impl CommandHandler<StartSessionCommand> for StartSessionCommandHandler {
    async fn handle(&self, cmd: StartSessionCommand) -> Result<SessionStartedResult, DomainError> {
        // 1. Get file
        let file = self.file_repository
            .find_by_id(&cmd.file_id)
            .await?
            .ok_or(DomainError::FileNotFound)?;
        
        if file.is_deleted {
            return Err(DomainError::FileNotFound);
        }
        
        // 2. Check permission
        let permission = self.permission_repository
            .find_active_by_client_and_file(&cmd.client_id, &cmd.file_id)
            .await?
            .ok_or(DomainError::PermissionDenied)?;
        
        if permission.is_expired() {
            return Err(DomainError::PermissionExpired);
        }
        
        if permission.is_revoked {
            return Err(DomainError::PermissionRevoked);
        }
        
        // 3. Check for duplicate session
        let existing_session = self.session_repository
            .find_active_by_client_and_file(&cmd.client_id, &cmd.file_id)
            .await?;
        
        if existing_session.is_some() {
            return Err(DomainError::SessionAlreadyActive);
        }
        
        // 4. Create session
        let session = Session::new(
            SessionId::new(),
            cmd.client_id.clone(),
            file.owner_id.clone(),
            cmd.file_id.clone(),
            permission.permissions.clone(),
            permission.max_duration_seconds,
            cmd.ip_address,
            cmd.user_agent.clone(),
        )?;
        
        // 5. Create sandbox with Landlock policy
        let sandbox_id = self.sandbox_service.create_sandbox(
            &session.id,
            &file.owner_id,
            &cmd.file_id,
            &permission.permissions,
        ).await?;
        
        session.set_sandbox_id(sandbox_id.clone());
        
        // 6. Initialize WebRTC connection
        let webrtc_offer = self.webrtc_service
            .create_offer(&session.id)
            .await?;
        
        // 7. Persist session
        self.session_repository.save(&session).await?;
        
        // 8. Schedule auto-termination
        self.scheduler.schedule_session_termination(
            &session.id,
            session.expires_at,
        ).await?;
        
        // 9. Emit event
        self.event_publisher.publish(DomainEvent::SessionStarted {
            session_id: session.id.clone(),
            client_id: cmd.client_id.clone(),
            owner_id: file.owner_id.clone(),
            file_id: cmd.file_id,
            started_at: session.started_at,
            ip_address: cmd.ip_address,
        }).await?;
        
        // 10. Notify owner via WebSocket
        self.websocket_service.notify_owner(
            &file.owner_id,
            WebSocketMessage::SessionStarted {
                session_id: session.id.clone(),
                client_email: self.get_client_email(&cmd.client_id).await?,
                file_name: file.name.clone(),
                started_at: session.started_at,
                ip_address: cmd.ip_address,
            }
        ).await?;
        
        Ok(SessionStartedResult {
            session_id: session.id,
            sandbox_id,
            webrtc_sdp_offer: webrtc_offer,
            expires_at: session.expires_at,
        })
    }
}
```

---

## API Endpoint

```http
POST /api/client/sessions
Authorization: Bearer {client_jwt_token}
Content-Type: application/json

Request Body:
{
  "file_id": "fil_123"
}

Response 201 Created:
{
  "session_id": "ses_789",
  "sandbox_id": "sandbox_abc123",
  "webrtc_sdp_offer": "v=0\r\no=- 123456 2 IN IP4...",
  "expires_at": "2026-02-14T11:30:00Z",
  "file_name": "report.pdf",
  "permissions": {
    "read": true,
    "write": false,
    "execute": false
  }
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [RequestSessionTerminationCommand](request_session_termination.md), [GetMyActiveSessionQuery](../queries/get_my_active_session.md)
