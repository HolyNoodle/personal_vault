# MoveFileCommand

**Purpose:** Owner moves a file to a different folder.

**Persona:** Owner

**Module:** `application::owner::commands::move_file`

---

## Command Structure

```rust
pub struct MoveFileCommand {
    pub file_id: FileId,
    pub owner_id: UserId,
    pub new_parent_folder_id: Option<FolderId>,  // None = root
}
```

---

## Validations

- ✅ file_id exists and belongs to owner_id
- ✅ new_parent_folder_id exists and belongs to owner_id (if provided)
- ✅ no duplicate file_name in destination folder
- ✅ file is not already in destination folder

---

## Acceptance Criteria

### AC1: Happy Path - Move File Successfully
**GIVEN** an Owner has a file in folder_A  
**WHEN** MoveFileCommand is executed with new_parent_folder_id=folder_B  
**THEN** File's parent_folder_id is updated to folder_B  
**AND** File path updated in database  
**AND** FileMoved event emitted  
**AND** audit log entry created

### AC2: Duplicate File Name - Move Rejected
**GIVEN** destination folder already has "report.pdf"  
**WHEN** MoveFileCommand is executed to move another "report.pdf"  
**THEN** command fails with `DomainError::DuplicateFileName`

### AC3: Move to Root - Parent Set to None
**GIVEN** a file in folder_A  
**WHEN** MoveFileCommand is executed with new_parent_folder_id=None  
**THEN** File is moved to root folder  
**AND** parent_folder_id = null

---

## API Endpoint

```http
PATCH /api/owner/files/{file_id}/move
Authorization: Bearer {owner_jwt_token}
Content-Type: application/json

Request Body:
{
  "new_parent_folder_id": "fld_456"
}

Response 200 OK:
{
  "success": true,
  "file_id": "fil_789",
  "new_parent_folder_id": "fld_456",
  "moved_at": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
