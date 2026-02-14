# GetStorageUsageQuery

**Purpose:** Owner retrieves detailed storage usage breakdown.

**Persona:** Owner

**Module:** `application::owner::queries::get_storage_usage`

---

## Query Structure

```rust
pub struct GetStorageUsageQuery {
    pub owner_id: UserId,
}
```

---

## Response Structure

```rust
pub struct GetStorageUsageQueryResult {
    pub total_quota_bytes: u64,
    pub total_used_bytes: u64,
    pub total_available_bytes: u64,
    pub utilization_percentage: f64,
    pub file_count: u64,
    pub folder_count: u64,
    pub trash_size_bytes: u64,
    pub by_folder: Vec<FolderUsage>,
    pub by_file_type: Vec<FileTypeUsage>,
    pub largest_files: Vec<LargeFile>,
}

pub struct FolderUsage {
    pub folder_id: Option<FolderId>,  // None = root
    pub folder_name: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub percentage_of_total: f64,
}

pub struct FileTypeUsage {
    pub content_type: String,
    pub extension: String,
    pub file_count: u64,
    pub total_size_bytes: u64,
    pub percentage_of_total: f64,
}

pub struct LargeFile {
    pub file_id: FileId,
    pub file_name: String,
    pub file_size_bytes: u64,
    pub created_at: DateTime<Utc>,
}
```

---

## Acceptance Criteria

### AC1: Happy Path - Get Storage Usage
**GIVEN** an Owner has 50GB quota with 30GB used  
**WHEN** GetStorageUsageQuery is executed  
**THEN** query returns:
- total_quota_bytes = 53687091200
- total_used_bytes = 32212254720
- total_available_bytes = 21474836480
- utilization_percentage = 60.0

### AC2: Breakdown By Folder
**GIVEN** files distributed across multiple folders  
**WHEN** GetStorageUsageQuery is executed  
**THEN** by_folder includes each folder with:
- folder_name
- size_bytes
- file_count
- percentage_of_total

### AC3: Breakdown By File Type
**GIVEN** files of various types  
**WHEN** GetStorageUsageQuery is executed  
**THEN** by_file_type includes:
- PDFs: 15GB (50%)
- Images: 10GB (33%)
- Videos: 5GB (17%)

### AC4: Largest Files - Top 10
**GIVEN** an Owner has many files  
**WHEN** GetStorageUsageQuery is executed  
**THEN** largest_files returns top 10 files by size

### AC5: Trash Size - Separate Calculation
**GIVEN** trash folder contains 5GB of deleted files  
**WHEN** GetStorageUsageQuery is executed  
**THEN** trash_size_bytes = 5368709120  
**AND** trash size is NOT included in total_used_bytes

---

## API Endpoint

```http
GET /api/owner/storage/usage
Authorization: Bearer {owner_jwt_token}

Response 200 OK:
{
  "total_quota_bytes": 53687091200,
  "total_used_bytes": 32212254720,
  "total_available_bytes": 21474836480,
  "utilization_percentage": 60.0,
  "file_count": 523,
  "folder_count": 15,
  "trash_size_bytes": 5368709120,
  "by_folder": [
    {
      "folder_id": "fld_123",
      "folder_name": "Documents",
      "size_bytes": 16106127360,
      "file_count": 234,
      "percentage_of_total": 50.0
    }
  ],
  "by_file_type": [
    {
      "content_type": "application/pdf",
      "extension": "pdf",
      "file_count": 150,
      "total_size_bytes": 16106127360,
      "percentage_of_total": 50.0
    }
  ],
  "largest_files": [
    {
      "file_id": "fil_999",
      "file_name": "presentation.pptx",
      "file_size_bytes": 1073741824,
      "created_at": "2026-02-01T10:00:00Z"
    }
  ]
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14
