# DownloadFileCommand

**Purpose:** Owner downloads a file from their storage.

**Persona:** Owner

**Module:** `application::owner::commands::download_file`

---

## Command Structure

```rust
pub struct DownloadFileCommand {
    pub file_id: FileId,
    pub owner_id: UserId,
}
```

---

## Validations

- ✅ file_id exists
- ✅ owner_id matches file.owner_id
- ✅ file is not deleted

---

## Acceptance Criteria

### AC1: Happy Path - Download File Successfully
**GIVEN** an Owner has a file  
**WHEN** DownloadFileCommand is executed  
**THEN** File content is streamed from filesystem  
**AND** HTTP headers include Content-Disposition with filename  
**AND** Content-Type matches file's MIME type  
**AND** FileDownloaded event emitted  
**AND** audit log entry created  
**AND** HTTP response 200 OK with file content

### AC2: Authorization - Owner Only
**GIVEN** a file belongs to user_A  
**WHEN** user_B executes DownloadFileCommand  
**THEN** command fails with `DomainError::Unauthorized`  
**AND** no file content is served

### AC3: Streaming Download - Large Files Handled Efficiently
**GIVEN** an Owner downloads a 5GB file  
**WHEN** DownloadFileCommand is executed  
**THEN** file is streamed in chunks (not loaded into memory)  
**AND** memory usage stays below 100MB  
**AND** supports HTTP range requests for resume capability

### AC4: Checksum Validation - Integrity Verification
**GIVEN** an Owner downloads a file  
**WHEN** DownloadFileCommand is executed  
**THEN** response includes ETag header with file's SHA-256 checksum  
**AND** client can verify file integrity

### AC5: Deleted File - Download Rejected
**GIVEN** a file marked as deleted (in trash)  
**WHEN** DownloadFileCommand is executed  
**THEN** command fails with `DomainError::FileNotFound`  
**AND** no content is served

---

## Handler Implementation

```rust
impl CommandHandler<DownloadFileCommand> for DownloadFileCommandHandler {
    async fn handle(&self, cmd: DownloadFileCommand) -> Result<FileStream, DomainError> {
        // 1. Get file metadata
        let file = self.file_repository
            .find_by_id(&cmd.file_id)
            .await?
            .ok_or(DomainError::FileNotFound)?;
        
        // 2. Verify ownership
        if file.owner_id != cmd.owner_id {
            return Err(DomainError::Unauthorized);
        }
        
        // 3. Check if deleted
        if file.is_deleted {
            return Err(DomainError::FileNotFound);
        }
        
        // 4. Get file path
        let file_path = format!("/data/users/{}/files/{}", file.owner_id, file.id);
        
        // 5. Verify file exists on filesystem
        if !tokio::fs::try_exists(&file_path).await? {
            return Err(DomainError::FileNotFoundOnFilesystem);
        }
        
        // 6. Open file for streaming
        let file_handle = tokio::fs::File::open(&file_path).await?;
        
        // 7. Emit event (async, non-blocking)
        tokio::spawn({
            let event_publisher = self.event_publisher.clone();
            let file_id = file.id.clone();
            let owner_id = cmd.owner_id.clone();
            async move {
                let _ = event_publisher.publish(DomainEvent::FileDownloaded {
                    file_id,
                    downloaded_by: owner_id,
                    timestamp: Utc::now(),
                }).await;
            }
        });
        
        // 8. Return file stream with metadata
        Ok(FileStream {
            file_handle,
            file_name: file.name,
            file_size_bytes: file.size_bytes,
            content_type: file.content_type,
            checksum_sha256: file.checksum_sha256,
        })
    }
}
```

---

## API Endpoint

```http
GET /api/owner/files/{file_id}/download
Authorization: Bearer {owner_jwt_token}

Response 200 OK:
Content-Type: application/pdf
Content-Disposition: attachment; filename="report.pdf"
Content-Length: 5242880
ETag: "abc123def456..."
Accept-Ranges: bytes

<binary file content>
```

---

## HTTP Range Support (Resume Downloads)

```http
GET /api/owner/files/{file_id}/download
Authorization: Bearer {owner_jwt_token}
Range: bytes=1000000-

Response 206 Partial Content:
Content-Type: application/pdf
Content-Range: bytes 1000000-5242879/5242880
Content-Length: 4242880
ETag: "abc123def456..."

<partial file content>
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [UploadFileCommand](upload_file.md)
