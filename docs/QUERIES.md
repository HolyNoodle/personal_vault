# Queries Documentation

## Overview

Queries represent **read operations** that retrieve data without changing system state. They are part of the CQRS pattern and handled by query handlers in the Application Layer.

## ⚠️ Security-First Query Design

All queries MUST:
1. Check authorization before returning data
2. Filter results based on user permissions
3. Never expose sensitive data (passwords, encryption keys)
4. Log access to sensitive resources
5. Apply pagination to prevent resource exhaustion

---

## Query Structure

```rust
pub struct Query {
    // Query-specific criteria
}

#[async_trait]
pub trait QueryHandler<Q, R>: Send + Sync {
    async fn handle(&self, query: Q, context: QueryContext) -> Result<R, QueryError>;
}

pub struct QueryContext {
    pub user_id: UserId,
    pub roles: HashSet<Role>,
    pub ip_address: IpAddress,
}

pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}
```

---

## Session Queries

### GetSessionQuery

**Purpose:** Retrieve detailed information about a specific session.

**Input:**
```rust
pub struct GetSessionQuery {
    pub session_id: SessionId,
}
```

**Authorization:**
- User MUST own session OR be admin
- Returns `PermissionDenied` if user lacks access

**Business Logic:**
1. Load session from repository
2. Check user owns session OR has admin role
3. Load associated sandbox details
4. Load resource usage statistics
5. Return session DTO

**Output:**
```rust
pub struct SessionDto {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub resources: ResourceUsageDto,
}

pub struct ResourceUsageDto {
    pub cpu_percent: f32,
    pub memory_mb: u32,
    pub pid_count: u16,
}
```

**Errors:**
- `SessionNotFound` - Session doesn't exist
- `PermissionDenied` - User lacks access

**Security Considerations:**
- MUST verify user authorization before returning data
- MUST NOT expose sandbox internals (PIDs, filesystem paths)
- MUST log access to session details

**Example Usage:**
```rust
let query = GetSessionQuery {
    session_id: SessionId::from_string("abc123...")?,
};

let session = query_handler.handle(query, context).await?;
println!("Session state: {:?}", session.state);
```

---

### ListSessionsQuery

**Purpose:** Retrieve all sessions for the current user.

**Input:**
```rust
pub struct ListSessionsQuery {
    pub status_filter: Option<SessionState>,  // Filter by state
    pub page: usize,
    pub page_size: usize,  // Max 100
}
```

**Authorization:**
- Returns only sessions owned by current user
- Admins can list all sessions (separate admin query)

**Business Logic:**
1. Query sessions by user_id
2. Apply status filter if provided
3. Sort by created_at descending
4. Apply pagination
5. Return paginated result

**Output:**
```rust
pub struct ListSessionsResult {
    pub sessions: Vec<SessionSummaryDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

pub struct SessionSummaryDto {
    pub session_id: SessionId,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
```

**Errors:**
- `InvalidPageSize` - page_size > 100

**Security Considerations:**
- MUST filter by user_id automatically
- MUST enforce maximum page size
- MUST NOT expose other users' sessions

---

## File Queries

### ListFilesQuery

**Purpose:** Retrieve files accessible to the current user.

**Input:**
```rust
pub struct ListFilesQuery {
    pub path_prefix: Option<ResourcePath>,  // Filter by path
    pub permission_filter: Option<AccessLevel>,  // Filter by permission
    pub page: usize,
    pub page_size: usize,  // Max 100
}
```

**Authorization:**
- Returns only files user has permission to access
- Admins see all files

**Business Logic:**
1. Query permissions by user_id
2. Apply path prefix filter if provided
3. Apply permission filter if provided
4. Load file metadata for each permitted path
5. Sort by path
6. Apply pagination
7. Return paginated result

**Output:**
```rust
pub struct ListFilesResult {
    pub files: Vec<FileDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

pub struct FileDto {
    pub path: ResourcePath,
    pub size_bytes: u64,
    pub mime_type: String,
    pub permissions: HashSet<AccessLevel>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub checksum: String,  // SHA-256
}
```

**Errors:**
- `InvalidPageSize` - page_size > 100
- `InvalidPathPrefix` - Path outside storage root

**Security Considerations:**
- MUST filter by user permissions
- MUST NOT expose file contents
- MUST validate path prefix doesn't escape storage
- MUST log file listing requests

---

### GetFileMetadataQuery

**Purpose:** Retrieve metadata for a specific file.

**Input:**
```rust
pub struct GetFileMetadataQuery {
    pub path: ResourcePath,
}
```

**Authorization:**
- User MUST have at least Read permission to file
- Returns `PermissionDenied` if user lacks access

**Business Logic:**
1. Check user has Read permission for path
2. Load file metadata from storage
3. Return file DTO

**Output:**
```rust
pub struct FileMetadataDto {
    pub path: ResourcePath,
    pub size_bytes: u64,
    pub mime_type: String,
    pub permissions: HashSet<AccessLevel>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub checksum: String,
    pub encrypted: bool,
}
```

**Errors:**
- `FileNotFound` - File doesn't exist
- `PermissionDenied` - User lacks Read permission

**Security Considerations:**
- MUST verify user has Read permission
- MUST log file metadata access
- MUST NOT expose encryption keys

---

## User Queries

### GetUserQuery

**Purpose:** Retrieve user account details.

**Input:**
```rust
pub struct GetUserQuery {
    pub user_id: UserId,
}
```

**Authorization:**
- User can retrieve own account details
- Admins can retrieve any user's details

**Business Logic:**
1. Check user_id matches context.user_id OR user is admin
2. Load user from repository
3. Return user DTO (sanitized)

**Output:**
```rust
pub struct UserDto {
    pub user_id: UserId,
    pub username: String,
    pub email: Option<String>,
    pub roles: HashSet<Role>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    // Password hash NOT included
}
```

**Errors:**
- `UserNotFound` - User doesn't exist
- `PermissionDenied` - User lacks access

**Security Considerations:**
- MUST NOT return password hash
- MUST NOT return sensitive user data
- MUST verify authorization

---

## Audit Queries

### GetAuditLogsQuery

**Purpose:** Retrieve audit trail for current user.

**Input:**
```rust
pub struct GetAuditLogsQuery {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub event_type_filter: Option<EventType>,
    pub page: usize,
    pub page_size: usize,  // Max 100
}

pub enum EventType {
    Login,
    Logout,
    FileAccess,
    SessionCreated,
    SessionTerminated,
    PermissionGranted,
    PermissionRevoked,
}
```

**Authorization:**
- User can retrieve own audit logs
- Admins can retrieve any user's audit logs

**Business Logic:**
1. Query audit logs by user_id
2. Apply date range filter
3. Apply event type filter
4. Sort by timestamp descending
5. Apply pagination
6. Return paginated result

**Output:**
```rust
pub struct AuditLogsResult {
    pub logs: Vec<AuditLogDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

pub struct AuditLogDto {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub resource: Option<String>,
    pub action: String,
    pub result: EventResult,
    pub ip_address: IpAddress,
    pub user_agent: Option<String>,
}

pub enum EventResult {
    Success,
    Failure { reason: String },
}
```

**Errors:**
- `InvalidDateRange` - start_date > end_date
- `InvalidPageSize` - page_size > 100

**Security Considerations:**
- MUST filter by user_id automatically
- MUST NOT expose other users' audit logs
- MUST enforce date range limits (max 1 year)
- MUST sanitize sensitive data in logs

---

## Admin Queries

### ListAllUsersQuery (Admin Only)

**Purpose:** Retrieve all users (admin only).

**Input:**
```rust
pub struct ListAllUsersQuery {
    pub role_filter: Option<Role>,
    pub enabled_filter: Option<bool>,
    pub page: usize,
    pub page_size: usize,  // Max 100
}
```

**Authorization:**
- User MUST have Admin role
- Returns `PermissionDenied` otherwise

**Business Logic:**
1. Verify user has Admin role
2. Query all users
3. Apply role filter if provided
4. Apply enabled filter if provided
5. Sort by username
6. Apply pagination
7. Return paginated result

**Output:**
```rust
pub struct ListAllUsersResult {
    pub users: Vec<UserDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}
```

**Errors:**
- `PermissionDenied` - User is not admin

**Security Considerations:**
- MUST verify admin role
- MUST NOT return password hashes
- MUST log admin queries

---

### ListAllSessionsQuery (Admin Only)

**Purpose:** Retrieve all sessions across all users (admin only).

**Input:**
```rust
pub struct ListAllSessionsQuery {
    pub user_filter: Option<UserId>,
    pub status_filter: Option<SessionState>,
    pub page: usize,
    pub page_size: usize,  // Max 100
}
```

**Authorization:**
- User MUST have Admin role

**Business Logic:**
1. Verify user has Admin role
2. Query all sessions
3. Apply user filter if provided
4. Apply status filter if provided
5. Sort by created_at descending
6. Apply pagination
7. Return paginated result

**Output:**
```rust
pub struct ListAllSessionsResult {
    pub sessions: Vec<SessionDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}
```

**Errors:**
- `PermissionDenied` - User is not admin

**Security Considerations:**
- MUST verify admin role
- MUST log admin queries

---

## Query Handler Implementation Pattern

```rust
pub struct GetSessionQueryHandler {
    session_repository: Arc<dyn SessionRepository>,
    authorization_service: Arc<dyn AuthorizationService>,
    audit_service: Arc<dyn AuditService>,
}

#[async_trait]
impl QueryHandler<GetSessionQuery, SessionDto> for GetSessionQueryHandler {
    async fn handle(
        &self,
        query: GetSessionQuery,
        context: QueryContext,
    ) -> Result<SessionDto, QueryError> {
        // 1. Load session
        let session = self.session_repository
            .find_by_id(&query.session_id)
            .await
            .map_err(|_| QueryError::SessionNotFound)?;
        
        // 2. Check authorization
        self.authorization_service
            .authorize_session_access(&context.user_id, &session)
            .await?;
        
        // 3. Audit log (for sensitive queries)
        self.audit_service.log_session_access(
            &context.user_id,
            &query.session_id,
            context.ip_address,
        ).await?;
        
        // 4. Map to DTO
        let dto = SessionDto {
            session_id: session.id().clone(),
            user_id: session.user_id().clone(),
            state: session.state().clone(),
            created_at: session.created_at(),
            last_activity: session.last_activity(),
            expires_at: session.expires_at(),
            resources: ResourceUsageDto {
                cpu_percent: session.cpu_usage(),
                memory_mb: session.memory_usage(),
                pid_count: session.pid_count(),
            },
        };
        
        // 5. Return result
        Ok(dto)
    }
}
```

---

## Pagination Best Practices

```rust
pub const DEFAULT_PAGE_SIZE: usize = 20;
pub const MAX_PAGE_SIZE: usize = 100;

pub fn validate_pagination(page: usize, page_size: usize) -> Result<(), QueryError> {
    if page_size == 0 {
        return Err(QueryError::InvalidPageSize("Page size must be > 0"));
    }
    
    if page_size > MAX_PAGE_SIZE {
        return Err(QueryError::InvalidPageSize("Page size exceeds maximum"));
    }
    
    Ok(())
}
```

---

## Query Result Caching (Future)

Queries are prime candidates for caching:

```rust
pub trait QueryCache: Send + Sync {
    async fn get<T>(&self, key: &str) -> Option<T>;
    async fn set<T>(&self, key: &str, value: T, ttl: Duration);
    async fn invalidate(&self, key: &str);
}

// Cache query results
let cache_key = format!("session:{}", query.session_id);
if let Some(cached) = self.cache.get(&cache_key).await {
    return Ok(cached);
}

let result = self.execute_query(query).await?;
self.cache.set(&cache_key, result.clone(), Duration::from_secs(60)).await;
Ok(result)
```

**Cache Invalidation:**
- Invalidate on relevant commands (e.g., UpdateSession invalidates GetSession cache)
- Use TTL for time-sensitive data
- Consider read-through vs. cache-aside patterns

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-13  
**Related Documents:** [COMMANDS.md](COMMANDS.md), [DOMAIN_OBJECTS.md](DOMAIN_OBJECTS.md), [ARCHITECTURE.md](ARCHITECTURE.md)
