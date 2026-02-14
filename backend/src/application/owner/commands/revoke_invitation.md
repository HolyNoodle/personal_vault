# RevokeInvitationCommand

**Purpose:** Owner revokes an invitation before it's accepted or expires.

**Persona:** Owner

**Module:** `application::owner::commands::revoke_invitation`

---

## Command Structure

```rust
pub struct RevokeInvitationCommand {
    pub invitation_id: InvitationId,
    pub owner_id: UserId,
}
```

---

## Validations

- ✅ invitation_id exists and belongs to owner_id
- ✅ invitation is not already accepted or revoked

---

## Acceptance Criteria

### AC1: Happy Path - Revoke Invitation
**GIVEN** an active invitation exists  
**WHEN** RevokeInvitationCommand is executed  
**THEN** Invitation status is updated to Revoked  
**AND** Invitation link becomes invalid immediately  
**AND** InvitationRevoked event emitted  
**AND** acceptance attempts return 404 Not Found

### AC2: Already Accepted - Cannot Revoke
**GIVEN** an invitation that has been accepted  
**WHEN** RevokeInvitationCommand is executed  
**THEN** command fails with `DomainError::InvitationAlreadyAccepted`  
**AND** permission remains active (use RevokePermissionCommand instead)

---

## API Endpoint

```http
DELETE /api/owner/invitations/{invitation_id}
Authorization: Bearer {owner_jwt_token}

Response 200 OK:
{
  "success": true,
  "invitation_id": "inv_789",
  "revoked_at": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
