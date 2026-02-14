# RequestSessionTerminationCommand

**Purpose:** Client requests to end their active viewing session.

**Persona:** Client

**Module:** `application::client::commands::request_session_termination`

---

## Command Structure

```rust
pub struct RequestSessionTerminationCommand {
    pub session_id: SessionId,
    pub client_id: UserId,
}
```

---

## Validations

- ✅ session_id exists
- ✅ session belongs to client_id
- ✅ session is active

---

## Acceptance Criteria

### AC1: Happy Path - Terminate Session Successfully
**GIVEN** a Client has an active session  
**WHEN** RequestSessionTerminationCommand is executed  
**THEN** Session state is updated to Terminated  
**AND** Sandbox is destroyed  
**AND** Landlock policy is removed  
**AND** WebRTC connection is closed  
**AND** SessionTerminated event emitted  
**AND** Owner receives WebSocket notification  
**AND** audit log entry created  
**AND** HTTP response 200 OK

### AC2: Authorization - Client Can Only Terminate Own Session
**GIVEN** a session belongs to client_A  
**WHEN** client_B executes RequestSessionTerminationCommand  
**THEN** command fails with `DomainError::Unauthorized`

### AC3: WebSocket Notification - Owner Informed
**GIVEN** a Client terminates their session  
**WHEN** RequestSessionTerminationCommand is executed  
**THEN** Owner receives WebSocket message:
```json
{
  "type": "SessionTerminated",
  "session_id": "ses_123",
  "client_email": "client@example.com",
  "file_name": "report.pdf",
  "terminated_at": "2026-02-14T10:45:00Z",
  "duration_seconds": 900,
  "reason": "ClientRequest"
}
```

### AC4: Cleanup - All Resources Released
**GIVEN** a session with sandbox and WebRTC connection  
**WHEN** RequestSessionTerminationCommand is executed  
**THEN** sandbox container is stopped and removed  
**AND** Landlock policies are removed from kernel  
**AND** WebRTC peer connection is closed  
**AND** temporary files are deleted

---

## Handler Implementation

```rust
impl CommandHandler<RequestSessionTerminationCommand> for RequestSessionTerminationCommandHandler {
    async fn handle(&self, cmd: RequestSessionTerminationCommand) -> Result<(), DomainError> {
        // 1. Get session
        let mut session = self.session_repository
            .find_by_id(&cmd.session_id)
            .await?
            .ok_or(DomainError::SessionNotFound)?;
        
        // 2. Verify ownership
        if session.client_id != cmd.client_id {
            return Err(DomainError::Unauthorized);
        }
        
        // 3. Check if already terminated
        if session.state == SessionState::Terminated {
            return Err(DomainError::SessionAlreadyTerminated);
        }
        
        // 4. Destroy sandbox
        if let Some(sandbox_id) = &session.sandbox_id {
            self.sandbox_service.destroy_sandbox(sandbox_id).await?;
        }
        
        // 5. Close WebRTC connection
        self.webrtc_service.close_connection(&session.id).await?;
        
        // 6. Update session
        session.terminate(TerminationReason::ClientRequest)?;
        self.session_repository.save(&session).await?;
        
        // 7. Cancel auto-termination timer
        self.scheduler.cancel_session_termination(&session.id).await?;
        
        // 8. Get file for notification
        let file = self.file_repository.find_by_id(&session.file_id).await?;
        
        // 9. Emit event
        self.event_publisher.publish(DomainEvent::SessionTerminated {
            session_id: cmd.session_id.clone(),
            client_id: cmd.client_id.clone(),
            owner_id: session.owner_id.clone(),
            file_id: session.file_id.clone(),
            terminated_at: session.terminated_at.unwrap(),
            duration_seconds: session.duration_seconds(),
            reason: TerminationReason::ClientRequest,
        }).await?;
        
        // 10. Notify owner
        self.websocket_service.notify_owner(
            &session.owner_id,
            WebSocketMessage::SessionTerminated {
                session_id: cmd.session_id,
                client_email: self.get_client_email(&cmd.client_id).await?,
                file_name: file.map(|f| f.name).unwrap_or_default(),
                terminated_at: session.terminated_at.unwrap(),
                duration_seconds: session.duration_seconds(),
                reason: TerminationReason::ClientRequest,
            }
        ).await?;
        
        Ok(())
    }
}
```

---

## API Endpoint

```http
DELETE /api/client/sessions/{session_id}
Authorization: Bearer {client_jwt_token}

Response 200 OK:
{
  "success": true,
  "session_id": "ses_789",
  "terminated_at": "2026-02-14T10:45:00Z",
  "duration_seconds": 900
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [StartSessionCommand](start_session.md)
