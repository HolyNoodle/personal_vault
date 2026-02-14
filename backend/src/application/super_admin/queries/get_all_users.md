# GetAllUsersQuery

**Purpose:** Super Admin retrieves a list of all users with filtering and pagination.

**Persona:** Super Admin

**Module:** `application::super_admin::queries::get_all_users`

---

## Query Structure

```rust
pub struct GetAllUsersQuery {
    pub role_filter: Option<UserRole>,        // Filter by role (Owner, Client)
    pub status_filter: Option<UserStatus>,    // Active, Disabled, Deleted
    pub search: Option<String>,                // Search by email
    pub sort_by: SortField,                    // Email, CreatedAt, StorageUsage
    pub sort_order: SortOrder,                 // Asc, Desc
    pub page: u32,                             // Page number (1-based)
    pub page_size: u32,                        // Items per page (max 100)
}

pub enum UserStatus {
    Active,
    Disabled,
    Deleted,
}

pub enum SortField {
    Email,
    CreatedAt,
    StorageUsage,
    LastActive,
}
```

---

## Response Structure

```rust
pub struct GetAllUsersQueryResult {
    pub users: Vec<UserSummary>,
    pub total_count: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

pub struct UserSummary {
    pub user_id: UserId,
    pub email: Email,
    pub role: UserRole,
    pub is_disabled: bool,
    pub is_deleted: bool,
    pub storage_quota_bytes: u64,
    pub storage_used_bytes: u64,
    pub created_at: DateTime<Utc>,
    pub last_active_at: Option<DateTime<Utc>>,
    pub active_sessions_count: u32,
}
```

---

## Acceptance Criteria

### AC1: Happy Path - Get All Users
**GIVEN** a Super Admin is authenticated  
**AND** database has 150 users  
**WHEN** GetAllUsersQuery is executed with page=1, page_size=50  
**THEN** query returns 50 users  
**AND** total_count = 150  
**AND** total_pages = 3  
**AND** each user has: id, email, role, status, storage info, created_at

### AC2: Filter By Role
**GIVEN** database has 100 Owners and 200 Clients  
**WHEN** GetAllUsersQuery is executed with role_filter=Owner  
**THEN** query returns only Owner users  
**AND** total_count = 100

### AC3: Filter By Status
**GIVEN** database has 50 disabled users  
**WHEN** GetAllUsersQuery is executed with status_filter=Disabled  
**THEN** query returns only disabled users  
**AND** total_count = 50

### AC4: Search By Email
**GIVEN** database has users with various emails  
**WHEN** GetAllUsersQuery is executed with search="john"  
**THEN** query returns users where email contains "john" (case-insensitive)

### AC5: Sort By Storage Usage
**GIVEN** database has users with different storage usage  
**WHEN** GetAllUsersQuery is executed with sort_by=StorageUsage, sort_order=Desc  
**THEN** query returns users ordered by storage_used_bytes descending

### AC6: Authorization - Only Super Admin
**GIVEN** a regular Owner user is authenticated  
**WHEN** GetAllUsersQuery is executed  
**THEN** query fails with `DomainError::Unauthorized`

### AC7: Performance - Query Within 200ms
**GIVEN** database has 10,000 users  
**WHEN** GetAllUsersQuery is executed  
**THEN** query completes within 200ms  
**AND** uses database indexes for filtering and sorting

---

## Query Implementation

```rust
impl QueryHandler<GetAllUsersQuery> for GetAllUsersQueryHandler {
    async fn handle(&self, query: GetAllUsersQuery) -> Result<GetAllUsersQueryResult, DomainError> {
        // 1. Validate page_size
        let page_size = query.page_size.min(100);
        let offset = (query.page - 1) * page_size;
        
        // 2. Build query with filters
        let mut sql = "SELECT u.*, 
                              COALESCE(SUM(f.file_size_bytes), 0) as storage_used,
                              COUNT(DISTINCT s.id) as active_sessions
                       FROM users u
                       LEFT JOIN files f ON f.owner_id = u.id AND f.is_deleted = false
                       LEFT JOIN sessions s ON s.user_id = u.id AND s.state = 'Active'
                       WHERE 1=1".to_string();
        
        if let Some(role) = query.role_filter {
            sql.push_str(&format!(" AND u.role = '{:?}'", role));
        }
        
        if let Some(status) = query.status_filter {
            match status {
                UserStatus::Active => sql.push_str(" AND u.is_disabled = false AND u.is_deleted = false"),
                UserStatus::Disabled => sql.push_str(" AND u.is_disabled = true"),
                UserStatus::Deleted => sql.push_str(" AND u.is_deleted = true"),
            }
        }
        
        if let Some(search) = query.search {
            sql.push_str(&format!(" AND u.email ILIKE '%{}%'", search));
        }
        
        sql.push_str(" GROUP BY u.id");
        
        // 3. Apply sorting
        let order_by = match query.sort_by {
            SortField::Email => "u.email",
            SortField::CreatedAt => "u.created_at",
            SortField::StorageUsage => "storage_used",
            SortField::LastActive => "u.last_active_at",
        };
        sql.push_str(&format!(" ORDER BY {} {}", order_by, query.sort_order));
        
        // 4. Apply pagination
        sql.push_str(&format!(" LIMIT {} OFFSET {}", page_size, offset));
        
        // 5. Execute query
        let users = self.db.query(&sql).await?;
        
        // 6. Get total count
        let total_count = self.user_repository.count_with_filters(
            query.role_filter,
            query.status_filter,
            query.search,
        ).await?;
        
        Ok(GetAllUsersQueryResult {
            users,
            total_count,
            page: query.page,
            page_size,
            total_pages: (total_count as f64 / page_size as f64).ceil() as u32,
        })
    }
}
```

---

## API Endpoint

```http
GET /api/admin/users?role=Owner&status=Active&search=john&sort_by=created_at&sort_order=desc&page=1&page_size=50
Authorization: Bearer {super_admin_jwt_token}

Response 200 OK:
{
  "users": [
    {
      "user_id": "usr_123",
      "email": "john@example.com",
      "role": "Owner",
      "is_disabled": false,
      "is_deleted": false,
      "storage_quota_bytes": 10737418240,
      "storage_used_bytes": 5368709120,
      "created_at": "2026-01-15T10:30:00Z",
      "last_active_at": "2026-02-14T09:00:00Z",
      "active_sessions_count": 0
    }
  ],
  "total_count": 150,
  "page": 1,
  "page_size": 50,
  "total_pages": 3
}
```

---

## Database Indexes

```sql
CREATE INDEX idx_users_role ON users(role);
CREATE INDEX idx_users_disabled ON users(is_disabled);
CREATE INDEX idx_users_deleted ON users(is_deleted);
CREATE INDEX idx_users_email_search ON users(email text_pattern_ops);
CREATE INDEX idx_users_created_at ON users(created_at DESC);
CREATE INDEX idx_users_last_active ON users(last_active_at DESC NULLS LAST);
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [GetUserDetailsQuery](get_user_details.md)
