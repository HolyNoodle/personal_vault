# GetMyFilesQuery

**Purpose:** Owner retrieves a list of all their files with filtering and search.

**Persona:** Owner

**Module:** `application::owner::queries::get_my_files`

---

## Query Structure

```rust
pub struct GetMyFilesQuery {
    pub owner_id: UserId,
    pub folder_id: Option<FolderId>,          // Filter by folder (None = all files)
    pub search: Option<String>,                // Search by filename
    pub file_type_filter: Option<Vec<String>>, // e.g., ["pdf", "docx"]
    pub sort_by: FileSortField,
    pub sort_order: SortOrder,
    pub page: u32,
    pub page_size: u32,
}

pub enum FileSortField {
    FileName,
    FileSize,
    CreatedAt,
    LastModified,
}
```

---

## Response Structure

```rust
pub struct GetMyFilesQueryResult {
    pub files: Vec<FileSummary>,
    pub total_count: u64,
    pub total_size_bytes: u64,
    pub page: u32,
    pub page_size: u32,
}

pub struct FileSummary {
    pub file_id: FileId,
    pub file_name: String,
    pub file_size_bytes: u64,
    pub content_type: String,
    pub parent_folder_id: Option<FolderId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active_sessions_count: u32,
    pub permissions_count: u32,
    pub checksum_sha256: String,
}
```

---

## Acceptance Criteria

### AC1: Happy Path - Get All Files
**GIVEN** an Owner has 50 files  
**WHEN** GetMyFilesQuery is executed with page=1, page_size=20  
**THEN** query returns first 20 files  
**AND** total_count = 50  
**AND** total_size_bytes = sum of all file sizes

### AC2: Filter By Folder
**GIVEN** an Owner has files in multiple folders  
**WHEN** GetMyFilesQuery is executed with folder_id=fld_123  
**THEN** query returns only files in that folder

### AC3: Search By Filename
**GIVEN** files named "report_2025.pdf", "report_2026.pdf", "notes.txt"  
**WHEN** GetMyFilesQuery is executed with search="report"  
**THEN** query returns only files containing "report" (case-insensitive)

### AC4: Filter By File Type
**GIVEN** files with various extensions  
**WHEN** GetMyFilesQuery is executed with file_type_filter=["pdf", "docx"]  
**THEN** query returns only PDF and DOCX files

### AC5: Sort By File Size
**GIVEN** files with different sizes  
**WHEN** GetMyFilesQuery is executed with sort_by=FileSize, sort_order=Desc  
**THEN** query returns files ordered by size (largest first)

---

## API Endpoint

```http
GET /api/owner/files?folder_id=fld_123&search=report&file_type=pdf&sort_by=created_at&sort_order=desc&page=1&page_size=20
Authorization: Bearer {owner_jwt_token}

Response 200 OK:
{
  "files": [
    {
      "file_id": "fil_123",
      "file_name": "report_2026.pdf",
      "file_size_bytes": 5242880,
      "content_type": "application/pdf",
      "parent_folder_id": "fld_123",
      "created_at": "2026-02-10T10:30:00Z",
      "updated_at": "2026-02-10T10:30:00Z",
      "active_sessions_count": 2,
      "permissions_count": 5,
      "checksum_sha256": "abc123..."
    }
  ],
  "total_count": 50,
  "total_size_bytes": 536870912,
  "page": 1,
  "page_size": 20
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
