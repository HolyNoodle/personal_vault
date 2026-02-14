# RenameFileCommand

**Purpose:** Owner renames a file.

**Persona:** Owner

**Module:** `application::owner::commands::rename_file`

---

## Command Structure

```rust
pub struct RenameFileCommand {
    pub file_id: FileId,
    pub owner_id: UserId,
    pub new_file_name: String,
}
```

---

## Validations

- ✅ file_id exists and belongs to owner_id
- ✅ new_file_name is valid (no path traversal, max 255 chars)
- ✅ no duplicate new_file_name in same folder
- ✅ new_file_name != current file_name

---

## Acceptance Criteria

### AC1: Happy Path - Rename File Successfully
**GIVEN** an Owner has a file "draft.pdf"  
**WHEN** RenameFileCommand is executed with new_file_name="final.pdf"  
**THEN** File name is updated to "final.pdf"  
**AND** FileRenamed event emitted  
**AND** all active sessions continue (no disruption)

### AC2: Duplicate File Name - Rename Rejected
**GIVEN** folder has files "report_v1.pdf" and "report_v2.pdf"  
**WHEN** RenameFileCommand is executed to rename "report_v1.pdf" to "report_v2.pdf"  
**THEN** command fails with `DomainError::DuplicateFileName`

### AC3: Invalid Name - Rename Rejected
**GIVEN** a file exists  
**WHEN** RenameFileCommand is executed with new_file_name="../../../etc/passwd"  
**THEN** command fails with `DomainError::InvalidFileName`

---

## API Endpoint

```http
PATCH /api/owner/files/{file_id}/rename
Authorization: Bearer {owner_jwt_token}
Content-Type: application/json

Request Body:
{
  "new_file_name": "final_report.pdf"
}

Response 200 OK:
{
  "success": true,
  "file_id": "fil_789",
  "old_file_name": "draft.pdf",
  "new_file_name": "final_report.pdf",
  "renamed_at": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
