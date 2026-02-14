# CreateInvitationCommand

**Purpose:** Owner creates an invitation link for a specific user to access a file.

**Persona:** Owner

**Module:** `application::owner::commands::create_invitation`

---

## Command Structure

```rust
pub struct CreateInvitationCommand {
    pub owner_id: UserId,
    pub file_id: FileId,
    pub invitee_email: Email,
    pub granted_permissions: PermissionSet,
    pub expires_at: DateTime<Utc>,
    pub max_uses: u32,  // Typically 1 for invitations
}
```

---

## Validations

- ✅ file_id exists and belongs to owner_id
- ✅ invitee_email is valid email format
- ✅ expires_at is in the future
- ✅ granted_permissions are valid

---

## Acceptance Criteria

### AC1: Happy Path - Create Invitation
**GIVEN** an Owner has a file  
**WHEN** CreateInvitationCommand is executed with:
- file_id: fil_123
- invitee_email: "client@example.com"
- granted_permissions: ReadOnly
- expires_at: 7 days from now
- max_uses: 1

**THEN** Invitation is created with unique token  
**AND** Email sent to client@example.com with invitation link  
**AND** InvitationCreated event emitted  
**AND** HTTP response 201 Created with invitation_id

### AC2: Email Notification - Client Receives Invite
**GIVEN** an invitation is created  
**THEN** email is sent to invitee with:
- Subject: "You've been invited to view a file"
- Invitation link: `https://domain.com/invite/{token}`
- File name, owner name
- Expiration date

### AC3: Invitation Acceptance - Permission Auto-Created
**GIVEN** a client clicks invitation link and authenticates  
**WHEN** client accepts the invitation  
**THEN** Permission is automatically created  
**AND** Invitation status updated to Accepted  
**AND** Client can immediately start a session

### AC4: Expiration - Invitation Auto-Expires
**GIVEN** an invitation with expires_at = now + 7 days  
**WHEN** 7 days pass  
**THEN** invitation link becomes invalid  
**AND** acceptance attempts return 410 Gone

### AC5: Single-Use - Invitation Deactivated After Acceptance
**GIVEN** an invitation with max_uses=1  
**WHEN** client accepts the invitation  
**THEN** invitation is marked as used  
**AND** further acceptance attempts return 410 Gone

---

## Handler Implementation

```rust
impl CommandHandler<CreateInvitationCommand> for CreateInvitationCommandHandler {
    async fn handle(&self, cmd: CreateInvitationCommand) -> Result<InvitationId, DomainError> {
        // 1. Verify file ownership
        let file = self.file_repository
            .find_by_id(&cmd.file_id)
            .await?
            .ok_or(DomainError::FileNotFound)?;
        
        if file.owner_id != cmd.owner_id {
            return Err(DomainError::Unauthorized);
        }
        
        // 2. Validate expiration
        if cmd.expires_at <= Utc::now() {
            return Err(DomainError::InvalidExpiration);
        }
        
        // 3. Create invitation
        let invitation = Invitation::new(
            InvitationId::new(),
            cmd.owner_id.clone(),
            cmd.file_id.clone(),
            cmd.invitee_email.clone(),
            cmd.granted_permissions,
            cmd.expires_at,
            cmd.max_uses,
        )?;
        
        // 4. Persist
        self.invitation_repository.save(&invitation).await?;
        
        // 5. Send email
        let invitation_url = format!(
            "https://{}/invite/{}",
            self.config.domain,
            invitation.token
        );
        
        self.email_service.send_invitation_email(
            &cmd.invitee_email,
            &file.name,
            &invitation_url,
            &cmd.expires_at,
        ).await?;
        
        // 6. Emit event
        self.event_publisher.publish(DomainEvent::InvitationCreated {
            invitation_id: invitation.id.clone(),
            owner_id: cmd.owner_id,
            file_id: cmd.file_id,
            invitee_email: cmd.invitee_email,
            timestamp: Utc::now(),
        }).await?;
        
        Ok(invitation.id)
    }
}
```

---

## API Endpoint

```http
POST /api/owner/invitations
Authorization: Bearer {owner_jwt_token}
Content-Type: application/json

Request Body:
{
  "file_id": "fil_123",
  "invitee_email": "client@example.com",
  "granted_permissions": {
    "read": true,
    "write": false,
    "execute": false
  },
  "expires_at": "2026-02-21T00:00:00Z",
  "max_uses": 1
}

Response 201 Created:
{
  "invitation_id": "inv_789",
  "invitation_url": "https://domain.com/invite/abc123def456...",
  "invitee_email": "client@example.com",
  "expires_at": "2026-02-21T00:00:00Z",
  "created_at": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [RevokeInvitationCommand](revoke_invitation.md), [CreateShareCommand](create_share.md)
