# CreateFolderCommand

**Purpose:** Owner creates a folder to organize files.

**Persona:** Owner

**Module:** `application::owner::commands::create_folder`

---

## Command Structure

```rust
pub struct CreateFolderCommand {
    pub owner_id: UserId,
    pub folder_name: String,
    pub parent_folder_id: Option<FolderId>,  // None = root level
}
```

---

## Validations

- ✅ owner_id is authenticated Owner
- ✅ folder_name is valid (no path traversal, max 255 chars)
- ✅ parent_folder_id exists and belongs to owner_id (if provided)
- ✅ no duplicate folder_name in same parent

---

## Acceptance Criteria

### AC1: Happy Path - Create Folder Successfully
**GIVEN** an Owner is authenticated  
**WHEN** CreateFolderCommand is executed with folder_name="Documents"  
**THEN** Folder is created with id, owner_id, name  
**AND** Folder metadata saved to database  
**AND** FolderCreated event emitted  
**AND** audit log entry created  
**AND** HTTP response 201 Created with folder_id

### AC2: Nested Folder - Create Subfolder
**GIVEN** a folder "Projects" exists (fld_123)  
**WHEN** CreateFolderCommand is executed with:
- folder_name: "2026"
- parent_folder_id: fld_123

**THEN** Subfolder "2026" is created under "Projects"  
**AND** parent_folder_id = fld_123

### AC3: Duplicate Folder Name - Rejected
**GIVEN** a folder "Documents" already exists in root  
**WHEN** CreateFolderCommand is executed with folder_name="Documents" in root  
**THEN** command fails with `DomainError::DuplicateFolderName`

---

## API Endpoint

```http
POST /api/owner/folders
Authorization: Bearer {owner_jwt_token}
Content-Type: application/json

Request Body:
{
  "folder_name": "Documents",
  "parent_folder_id": null
}

Response 201 Created:
{
  "folder_id": "fld_123",
  "folder_name": "Documents",
  "parent_folder_id": null,
  "created_at": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
