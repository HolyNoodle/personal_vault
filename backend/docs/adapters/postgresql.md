# PostgreSQL Adapter (Database Persistence)

**Purpose**: Persist and retrieve application data using PostgreSQL relational database.

**Technology**: PostgreSQL 14+ with sqlx (async Rust SQL toolkit)

**Layer**: Adapters (Secondary/Driven Adapter)

---

## Responsibilities

- Persist domain aggregates (User, File, Permission, Session, etc.)
- Execute queries for application layer
- Manage database schema via migrations
- Handle transactions and consistency
- Connection pooling and performance optimization
- Ensure data integrity with constraints

---

## Dependencies

### Required Crates
```toml
[dependencies]
# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "migrate", "chrono", "uuid", "json"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Date/Time
chrono = { version = "0.4", features = ["serde"] }

# UUID
uuid = { version = "1.6", features = ["v4", "serde"] }

# Async runtime
tokio = { version = "1.35", features = ["full"] }
```

---

## Database Schema

### Users Table
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    role VARCHAR(50) NOT NULL CHECK (role IN ('SuperAdmin', 'Owner', 'Client')),
    storage_quota_bytes BIGINT NOT NULL DEFAULT 107374182400, -- 100GB
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_role ON users(role);
```

### Files Table
```sql
CREATE TABLE files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_folder_id UUID REFERENCES files(id) ON DELETE CASCADE,
    file_name VARCHAR(255) NOT NULL,
    file_path TEXT NOT NULL UNIQUE,
    content_type VARCHAR(255),
    size_bytes BIGINT NOT NULL DEFAULT 0,
    checksum_sha256 VARCHAR(64),
    is_folder BOOLEAN NOT NULL DEFAULT false,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_files_owner ON files(owner_id);
CREATE INDEX idx_files_parent ON files(parent_folder_id);
CREATE INDEX idx_files_path ON files(file_path);
CREATE INDEX idx_files_deleted ON files(is_deleted) WHERE is_deleted = false;
```

### Permissions Table
```sql
CREATE TABLE permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    client_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    can_read BOOLEAN NOT NULL DEFAULT true,
    can_write BOOLEAN NOT NULL DEFAULT false,
    can_execute BOOLEAN NOT NULL DEFAULT false,
    max_duration_seconds INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    UNIQUE(file_id, client_id)
);

CREATE INDEX idx_permissions_file ON permissions(file_id);
CREATE INDEX idx_permissions_client ON permissions(client_id);
CREATE INDEX idx_permissions_active ON permissions(file_id, client_id) WHERE revoked_at IS NULL;
```

### Sessions Table
```sql
CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    client_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    terminated_at TIMESTAMPTZ,
    termination_reason VARCHAR(255),
    landlock_sandbox_id VARCHAR(255) UNIQUE,
    webrtc_peer_id VARCHAR(255)
);

CREATE INDEX idx_sessions_file ON sessions(file_id);
CREATE INDEX idx_sessions_client ON sessions(client_id);
CREATE INDEX idx_sessions_owner ON sessions(owner_id);
CREATE INDEX idx_sessions_active ON sessions(terminated_at) WHERE terminated_at IS NULL;
```

### Access Requests Table
```sql
CREATE TABLE access_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    requester_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL CHECK (status IN ('Pending', 'Approved', 'Denied', 'Cancelled')) DEFAULT 'Pending',
    reason TEXT,
    requested_permissions JSONB NOT NULL DEFAULT '{"read": true, "write": false}',
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    responded_at TIMESTAMPTZ,
    response_reason TEXT
);

CREATE INDEX idx_access_requests_file ON access_requests(file_id);
CREATE INDEX idx_access_requests_requester ON access_requests(requester_id);
CREATE INDEX idx_access_requests_owner ON access_requests(owner_id);
CREATE INDEX idx_access_requests_status ON access_requests(status);
```

### Invitations Table
```sql
CREATE TABLE invitations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    invitee_email VARCHAR(255) NOT NULL,
    file_id UUID REFERENCES files(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL CHECK (role IN ('Client')),
    permissions JSONB NOT NULL DEFAULT '{"read": true, "write": false}',
    token VARCHAR(255) UNIQUE NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invitations_token ON invitations(token);
CREATE INDEX idx_invitations_email ON invitations(invitee_email);
CREATE INDEX idx_invitations_owner ON invitations(owner_id);
```

### WebAuthn Credentials Table
```sql
CREATE TABLE webauthn_credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id BYTEA UNIQUE NOT NULL,
    public_key BYTEA NOT NULL,
    sign_count BIGINT NOT NULL DEFAULT 0,
    aaguid VARCHAR(255),
    credential_name VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

CREATE INDEX idx_webauthn_user ON webauthn_credentials(user_id);
CREATE INDEX idx_webauthn_credential_id ON webauthn_credentials(credential_id);
```

### Audit Logs Table
```sql
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type VARCHAR(100) NOT NULL,
    actor_id UUID REFERENCES users(id),
    actor_email VARCHAR(255),
    target_type VARCHAR(100),
    target_id UUID,
    action VARCHAR(100) NOT NULL,
    details JSONB,
    ip_address INET,
    user_agent TEXT,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_actor ON audit_logs(actor_id);
CREATE INDEX idx_audit_occurred ON audit_logs(occurred_at DESC);
CREATE INDEX idx_audit_event_type ON audit_logs(event_type);
CREATE INDEX idx_audit_target ON audit_logs(target_type, target_id);
```

---

## Repository Implementation

### User Repository
```rust
use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub storage_quota_bytes: i64,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    pub async fn find_by_id(&self, user_id: Uuid) -> Result<Option<UserRow>, sqlx::Error> {
        sqlx::query_as::<_, UserRow>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }
    
    pub async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>, sqlx::Error> {
        sqlx::query_as::<_, UserRow>(
            "SELECT * FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }
    
    pub async fn create(&self, email: &str, role: &str, quota: i64) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            INSERT INTO users (email, role, storage_quota_bytes)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            email,
            role,
            quota
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(row.id)
    }
    
    pub async fn update_quota(&self, user_id: Uuid, new_quota: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE users SET storage_quota_bytes = $1, updated_at = NOW() WHERE id = $2",
            new_quota,
            user_id
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn disable(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE users SET is_enabled = false, updated_at = NOW() WHERE id = $1",
            user_id
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}
```

---

## Migrations

Using sqlx-cli for database migrations:

```bash
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Create migration
sqlx migrate add create_users_table

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

Migration file example (`migrations/001_create_users.sql`):
```sql
-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    role VARCHAR(50) NOT NULL,
    storage_quota_bytes BIGINT NOT NULL DEFAULT 107374182400,
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

## Connection Pool Configuration

```rust
use sqlx::postgres::{PgPoolOptions, PgConnectOptions};
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let options = database_url
        .parse::<PgConnectOptions>()?
        .application_name("sandbox-server");
    
    PgPoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect_with(options)
        .await
}
```

---

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[sqlx::test]
    async fn test_create_user(pool: PgPool) {
        let repo = PostgresUserRepository::new(pool);
        let user_id = repo.create("test@example.com", "Owner", 100_000_000_000)
            .await
            .unwrap();
        
        let user = repo.find_by_id(user_id).await.unwrap().unwrap();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.role, "Owner");
    }
}
```

### Integration Tests
Use **testcontainers** to spin up real PostgreSQL instance:

```rust
use testcontainers::{clients, images::postgres::Postgres};

#[tokio::test]
async fn test_user_repository_integration() {
    let docker = clients::Cli::default();
    let postgres = docker.run(Postgres::default());
    let connection_string = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        postgres.get_host_port_ipv4(5432)
    );
    
    let pool = create_pool(&connection_string).await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    
    // Test repository operations
    let repo = PostgresUserRepository::new(pool);
    // ...
}
```

---

## Configuration

```toml
# .env
DATABASE_URL=postgres://sandbox_user:password@localhost:5432/sandbox_db
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=5
```

```rust
use dotenvy::dotenv;
use std::env;

pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

impl DatabaseConfig {
    pub fn from_env() -> Self {
        dotenv().ok();
        
        Self {
            url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .unwrap(),
            min_connections: env::var("DATABASE_MIN_CONNECTIONS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap(),
        }
    }
}
```

---

## Security Considerations

1. **SQL Injection Prevention**: Always use parameterized queries (sqlx compile-time checking)
2. **Prepared Statements**: sqlx uses prepared statements automatically
3. **Row-Level Security**: Use PostgreSQL RLS for multi-tenant isolation
4. **Connection Encryption**: TLS/SSL for database connections (require SSL in production)
5. **Credentials Management**: Never hardcode credentials, use environment variables
6. **Audit Logging**: Log all data mutations for compliance

---

## Performance Optimization

1. **Indexes**: Add indexes on frequently queried columns (email, file_path, etc.)
2. **Connection Pooling**: Reuse connections, avoid creating new connections per request
3. **Query Optimization**: Use EXPLAIN ANALYZE to identify slow queries
4. **Batch Operations**: Use `INSERT ... VALUES (...)` for bulk inserts
5. **Pagination**: Always paginate large result sets (LIMIT/OFFSET or cursor-based)

Example pagination:
```sql
-- Limit/Offset (simpler but slower for deep pages)
SELECT * FROM files WHERE owner_id = $1 ORDER BY created_at DESC LIMIT 50 OFFSET 100;

-- Cursor-based (faster for deep pages)
SELECT * FROM files WHERE owner_id = $1 AND created_at < $2 ORDER BY created_at DESC LIMIT 50;
```

---

**Document Version**: 1.0  
**Last Updated**: 2026-02-14  
**Related**: [../../docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md), [http.md](http.md)
