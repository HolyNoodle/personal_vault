# DeleteFileCommand

**Purpose:** Owner deletes a file from their storage.

**Persona:** Owner

**Module:** `application::owner::commands::delete_file`

---

## Command Structure

```rust
pub struct DeleteFileCommand {
    pub file_id: FileId,
    pub owner_id: UserId,
    pub permanent: bool,  // If false, move to trash (30-day retention)
}
```

---

## Validations

- ✅ file_id exists
- ✅ owner_id matches file.owner_id
- ✅ file is not already deleted

---

## Acceptance Criteria

### AC1: Happy Path - Soft Delete File
**GIVEN** an Owner has a file  
**WHEN** DeleteFileCommand is executed with permanent=false  
**THEN** File is marked as deleted (is_deleted=true, deleted_at=now)  
**AND** File is moved to `/data/users/{owner_id}/.trash/{file_id}`  
**AND** Owner's storage_used is NOT updated (kept for 30 days)  
**AND** all active sessions for this file are terminated  
**AND** FileDeleted event emitted  
**AND** audit log entry created

### AC2: Permanent Delete - File Removed
**GIVEN** an Owner has a file  
**WHEN** DeleteFileCommand is executed with permanent=true  
**THEN** File is physically deleted from filesystem  
**AND** File metadata marked as permanently_deleted  
**AND** Owner's storage_used is decreased  
**AND** all permissions for this file are revoked  
**AND** FilePermanentlyDeleted event emitted

### AC3: Authorization - Owner Only
**GIVEN** a file belongs to user_A  
**WHEN** user_B executes DeleteFileCommand  
**THEN** command fails with `DomainError::Unauthorized`

### AC4: Active Sessions - All Terminated on Delete
**GIVEN** a file has 3 active client sessions  
**WHEN** DeleteFileCommand is executed  
**THEN** all 3 sessions are terminated immediately

---

## API Endpoint

```http
DELETE /api/owner/files/{file_id}?permanent=false
Authorization: Bearer {owner_jwt_token}

Response 200 OK:
{
  "success": true,
  "file_id": "fil_789",
  "deleted_at": "2026-02-14T10:30:00Z",
  "permanent": false,
  "retention_days": 30
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
