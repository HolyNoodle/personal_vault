# GetSystemStatsQuery

**Purpose:** Super Admin retrieves system-wide statistics and metrics.

**Persona:** Super Admin

**Module:** `application::super_admin::queries::get_system_stats`

---

## Query Structure

```rust
pub struct GetSystemStatsQuery {
    // No parameters - returns current system stats
}
```

---

## Response Structure

```rust
pub struct GetSystemStatsQueryResult {
    pub users: UserStats,
    pub files: FileStats,
    pub sessions: SessionStats,
    pub storage: StorageStats,
    pub system: SystemHealthStats,
    pub timestamp: DateTime<Utc>,
}

pub struct UserStats {
    pub total_users: u64,
    pub total_owners: u64,
    pub total_clients: u64,
    pub active_users: u64,            // Not disabled/deleted
    pub disabled_users: u64,
    pub deleted_users: u64,
    pub users_created_last_24h: u64,
    pub users_created_last_7d: u64,
}

pub struct FileStats {
    pub total_files: u64,
    pub total_size_bytes: u64,
    pub files_uploaded_last_24h: u64,
    pub files_deleted_last_24h: u64,
    pub average_file_size_bytes: u64,
    pub largest_file_bytes: u64,
}

pub struct SessionStats {
    pub total_sessions_all_time: u64,
    pub active_sessions: u64,
    pub sessions_started_last_24h: u64,
    pub average_session_duration_seconds: u64,
    pub longest_session_duration_seconds: u64,
}

pub struct StorageStats {
    pub total_allocated_quota_bytes: u64,
    pub total_used_bytes: u64,
    pub total_available_bytes: u64,
    pub utilization_percentage: f64,
    pub trash_size_bytes: u64,
}

pub struct SystemHealthStats {
    pub cpu_usage_percentage: f64,
    pub memory_usage_percentage: f64,
    pub disk_usage_percentage: f64,
    pub active_sandboxes: u32,
    pub uptime_seconds: u64,
}
```

---

## Acceptance Criteria

### AC1: Happy Path - Get System Stats
**GIVEN** a Super Admin is authenticated  
**AND** system has:
- 150 users (100 Owners, 50 Clients)
- 10,000 files
- 25 active sessions
**WHEN** GetSystemStatsQuery is executed  
**THEN** query returns complete statistics  
**AND** all counts are accurate  
**AND** query completes within 500ms

### AC2: Real-Time Active Sessions Count
**GIVEN** 30 active sessions exist  
**WHEN** GetSystemStatsQuery is executed  
**THEN** active_sessions = 30  
**AND** count reflects current state (not cached)

### AC3: Storage Utilization Calculated
**GIVEN** total allocated quota = 1TB  
**AND** total used = 500GB  
**WHEN** GetSystemStatsQuery is executed  
**THEN** utilization_percentage = 50.0

### AC4: Authorization - Only Super Admin
**GIVEN** a regular Owner user is authenticated  
**WHEN** GetSystemStatsQuery is executed  
**THEN** query fails with `DomainError::Unauthorized`

---

## Query Implementation

```rust
impl QueryHandler<GetSystemStatsQuery> for GetSystemStatsQueryHandler {
    async fn handle(&self, _query: GetSystemStatsQuery) -> Result<GetSystemStatsQueryResult, DomainError> {
        // Execute queries in parallel for performance
        let (user_stats, file_stats, session_stats, storage_stats, system_stats) = tokio::join!(
            self.get_user_stats(),
            self.get_file_stats(),
            self.get_session_stats(),
            self.get_storage_stats(),
            self.get_system_health(),
        );
        
        Ok(GetSystemStatsQueryResult {
            users: user_stats?,
            files: file_stats?,
            sessions: session_stats?,
            storage: storage_stats?,
            system: system_stats?,
            timestamp: Utc::now(),
        })
    }
    
    async fn get_user_stats(&self) -> Result<UserStats, DomainError> {
        let stats = sqlx::query_as!(
            UserStats,
            r#"
            SELECT 
                COUNT(*) as total_users,
                COUNT(*) FILTER (WHERE role = 'Owner') as total_owners,
                COUNT(*) FILTER (WHERE role = 'Client') as total_clients,
                COUNT(*) FILTER (WHERE is_disabled = false AND is_deleted = false) as active_users,
                COUNT(*) FILTER (WHERE is_disabled = true) as disabled_users,
                COUNT(*) FILTER (WHERE is_deleted = true) as deleted_users,
                COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '24 hours') as users_created_last_24h,
                COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '7 days') as users_created_last_7d
            FROM users
            "#
        )
        .fetch_one(&self.db)
        .await?;
        
        Ok(stats)
    }
}
```

---

## API Endpoint

```http
GET /api/admin/stats
Authorization: Bearer {super_admin_jwt_token}

Response 200 OK:
{
  "users": {
    "total_users": 150,
    "total_owners": 100,
    "total_clients": 50,
    "active_users": 140,
    "disabled_users": 8,
    "deleted_users": 2,
    "users_created_last_24h": 5,
    "users_created_last_7d": 23
  },
  "files": {
    "total_files": 10000,
    "total_size_bytes": 536870912000,
    "files_uploaded_last_24h": 150,
    "files_deleted_last_24h": 10,
    "average_file_size_bytes": 53687091,
    "largest_file_bytes": 1073741824
  },
  "sessions": {
    "total_sessions_all_time": 5000,
    "active_sessions": 25,
    "sessions_started_last_24h": 80,
    "average_session_duration_seconds": 1800,
    "longest_session_duration_seconds": 7200
  },
  "storage": {
    "total_allocated_quota_bytes": 1099511627776,
    "total_used_bytes": 536870912000,
    "total_available_bytes": 562640715776,
    "utilization_percentage": 48.8,
    "trash_size_bytes": 10737418240
  },
  "system": {
    "cpu_usage_percentage": 35.2,
    "memory_usage_percentage": 62.8,
    "disk_usage_percentage": 71.5,
    "active_sandboxes": 25,
    "uptime_seconds": 864000
  },
  "timestamp": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [GetActiveSessionsQuery](get_active_sessions.md)
