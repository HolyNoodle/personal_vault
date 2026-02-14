# UploadFileCommand

**Purpose:** Owner uploads a file to their storage.

**Persona:** Owner

**Module:** `application::owner::commands::upload_file`

---

## Command Structure

```rust
pub struct UploadFileCommand {
    pub owner_id: UserId,
    pub file_name: String,
    pub file_size_bytes: u64,
    pub parent_folder_id: Option<FolderId>,  // None = root
    pub content_type: String,
    pub file_stream: tokio::io::AsyncRead,   // Streaming upload
    pub checksum_sha256: String,             // Client-computed hash
}
```

---

## Validations

- ✅ owner_id is authenticated Owner
- ✅ file_name is valid (no path traversal, max 255 chars)
- ✅ file_size_bytes > 0 and <= max_file_size (10GB)
- ✅ owner has available quota (current_usage + file_size <= quota)
- ✅ parent_folder_id exists and belongs to owner (if provided)
- ✅ no duplicate file_name in same folder
- ✅ content_type is valid MIME type
- ✅ checksum matches uploaded content

---

## Acceptance Criteria

### AC1: Happy Path - Upload File Successfully
**GIVEN** an Owner is authenticated  
**AND** Owner has 50GB quota with 10GB used  
**WHEN** UploadFileCommand is executed with:
- file_name: "report.pdf"
- file_size_bytes: 5MB
- parent_folder_id: fld_123

**THEN** File is saved to filesystem at `/data/users/{owner_id}/files/{file_id}`  
**AND** File metadata saved to database  
**AND** Owner's storage_used updated (10GB → 10.005GB)  
**AND** FileUploaded event emitted  
**AND** audit log entry created  
**AND** HTTP response 201 Created with file_id

### AC2: Quota Enforcement - Upload Rejected When Over Quota
**GIVEN** an Owner with 10GB quota and 9.5GB used  
**WHEN** UploadFileCommand is executed with 1GB file  
**THEN** command fails with `DomainError::StorageQuotaExceeded`  
**AND** file is NOT saved  
**AND** HTTP response 413 Payload Too Large

### AC3: Duplicate File Name - Upload Rejected
**GIVEN** a file "report.pdf" exists in folder fld_123  
**WHEN** UploadFileCommand is executed with file_name="report.pdf" in same folder  
**THEN** command fails with `DomainError::DuplicateFileName`  
**AND** HTTP response 409 Conflict

### AC4: Checksum Validation - Upload Rejected on Mismatch
**GIVEN** an Owner uploads a file  
**WHEN** UploadFileCommand is executed with checksum_sha256="abc123"  
**AND** actual uploaded content has checksum="def456"  
**THEN** command fails with `DomainError::ChecksumMismatch`  
**AND** uploaded file is deleted  
**AND** storage quota is NOT updated

### AC5: Streaming Upload - Large File Handled Efficiently
**GIVEN** an Owner uploads a 5GB file  
**WHEN** UploadFileCommand is executed with streaming upload  
**THEN** file is written to disk in chunks (not buffered in memory)  
**AND** memory usage stays below 100MB  
**AND** upload progress is tracked

### AC6: Parent Folder Validation - Folder Must Exist
**GIVEN** parent_folder_id does not exist  
**WHEN** UploadFileCommand is executed  
**THEN** command fails with `DomainError::FolderNotFound`

### AC7: Parent Folder Authorization - Must Belong to Owner
**GIVEN** parent_folder_id belongs to a different user  
**WHEN** UploadFileCommand is executed  
**THEN** command fails with `DomainError::Unauthorized`

---

## Handler Implementation

```rust
impl CommandHandler<UploadFileCommand> for UploadFileCommandHandler {
    async fn handle(&self, cmd: UploadFileCommand) -> Result<FileId, DomainError> {
        // 1. Get owner
        let owner = self.user_repository
            .find_by_id(&cmd.owner_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;
        
        // 2. Check quota
        let current_usage = self.file_repository
            .calculate_storage_usage(&cmd.owner_id)
            .await?;
        
        if current_usage + cmd.file_size_bytes > owner.storage_quota_bytes {
            return Err(DomainError::StorageQuotaExceeded {
                current: current_usage,
                quota: owner.storage_quota_bytes,
                requested: cmd.file_size_bytes,
            });
        }
        
        // 3. Validate parent folder
        if let Some(parent_id) = &cmd.parent_folder_id {
            let folder = self.folder_repository
                .find_by_id(parent_id)
                .await?
                .ok_or(DomainError::FolderNotFound)?;
            
            if folder.owner_id != cmd.owner_id {
                return Err(DomainError::Unauthorized);
            }
        }
        
        // 4. Check duplicate filename
        let duplicate = self.file_repository
            .find_by_name(&cmd.owner_id, &cmd.parent_folder_id, &cmd.file_name)
            .await?;
        
        if duplicate.is_some() {
            return Err(DomainError::DuplicateFileName);
        }
        
        // 5. Create file entity
        let file = File::new(
            FileId::new(),
            cmd.owner_id.clone(),
            cmd.file_name.clone(),
            cmd.file_size_bytes,
            cmd.parent_folder_id.clone(),
            cmd.content_type.clone(),
        )?;
        
        // 6. Stream file to filesystem
        let file_path = format!("/data/users/{}/files/{}", cmd.owner_id, file.id);
        let mut hasher = Sha256::new();
        let mut bytes_written = 0u64;
        
        let mut file_writer = tokio::fs::File::create(&file_path).await?;
        let mut reader = cmd.file_stream;
        let mut buffer = vec![0u8; 65536]; // 64KB chunks
        
        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 { break; }
            
            file_writer.write_all(&buffer[..n]).await?;
            hasher.update(&buffer[..n]);
            bytes_written += n as u64;
        }
        
        // 7. Verify checksum
        let computed_hash = format!("{:x}", hasher.finalize());
        if computed_hash != cmd.checksum_sha256 {
            // Delete uploaded file
            tokio::fs::remove_file(&file_path).await?;
            return Err(DomainError::ChecksumMismatch {
                expected: cmd.checksum_sha256,
                actual: computed_hash,
            });
        }
        
        // 8. Verify file size
        if bytes_written != cmd.file_size_bytes {
            tokio::fs::remove_file(&file_path).await?;
            return Err(DomainError::FileSizeMismatch);
        }
        
        // 9. Persist metadata
        self.file_repository.save(&file).await?;
        
        // 10. Emit event
        self.event_publisher.publish(DomainEvent::FileUploaded {
            file_id: file.id.clone(),
            owner_id: cmd.owner_id,
            file_name: cmd.file_name,
            file_size_bytes: cmd.file_size_bytes,
            timestamp: Utc::now(),
        }).await?;
        
        Ok(file.id)
    }
}
```

---

## API Endpoint

```http
POST /api/owner/files
Authorization: Bearer {owner_jwt_token}
Content-Type: multipart/form-data

Form Data:
- file: <binary data>
- parent_folder_id: fld_123 (optional)
- checksum_sha256: abc123...

Response 201 Created:
{
  "file_id": "fil_789",
  "file_name": "report.pdf",
  "file_size_bytes": 5242880,
  "checksum_sha256": "abc123...",
  "uploaded_at": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [DeleteFileCommand](delete_file.md), [MoveFileCommand](move_file.md)
