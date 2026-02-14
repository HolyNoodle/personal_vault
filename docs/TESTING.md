# Testing Strategy

## Overview

This document defines the comprehensive testing strategy for the Secure Sandbox Server. Testing is **mandatory** at multiple levels to ensure security, correctness, and reliability.

**Testing Philosophy:**
- **Security-first:** All security-critical paths must have tests
- **Acceptance-driven:** Each command/query has explicit acceptance criteria
- **Fast feedback:** Tests run in CI/CD, block merges on failure
- **Test pyramid:** Many unit tests, fewer integration tests, minimal E2E

---

## Testing Levels

### 1. Domain Level - Unit Tests

**What:** Pure business logic without external dependencies.

**Scope:**
- Aggregates (User, File, Permission, Session, Share, AccessRequest)
- Value Objects (Email, UserId, FileName, etc.)
- Domain Services
- Domain Events

**Tools:**
- `cargo test` - Standard Rust testing
- `proptest` - Property-based testing for value objects
- `mockall` - Mocking dependencies (if needed, though domain should be pure)

**Characteristics:**
- ✅ Fast (milliseconds)
- ✅ No I/O (no database, filesystem, network)
- ✅ Deterministic (no random data except with fixed seeds)
- ✅ Isolated (one aggregate/value object per test)

**Example:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn user_can_be_registered_with_valid_data() {
        // Given
        let email = Email::new("john@example.com").unwrap();
        let role = UserRole::Owner;
        let quota = 10_000_000_000; // 10GB
        
        // When
        let user = User::register(
            UserId::generate(),
            email.clone(),
            role,
            quota,
        );
        
        // Then
        assert!(user.is_ok());
        let user = user.unwrap();
        assert_eq!(user.email, email);
        assert_eq!(user.role, role);
        assert_eq!(user.storage_quota_bytes, quota);
        assert!(!user.is_deleted);
        assert_eq!(user.events().len(), 1);
        assert!(matches!(user.events()[0], DomainEvent::UserRegistered { .. }));
    }
    
    #[test]
    fn user_cannot_be_registered_with_zero_quota() {
        let result = User::register(
            UserId::generate(),
            Email::new("john@example.com").unwrap(),
            UserRole::Owner,
            0, // Invalid: zero quota
        );
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DomainError::InvalidStorageQuota);
    }
    
    #[test]
    fn permission_expires_after_expiration_date() {
        // Given
        let expires_at = Utc::now() - Duration::hours(1); // 1 hour ago
        let permission = Permission::grant(
            PermissionId::generate(),
            UserId::generate(),
            UserId::generate(),
            FileId::generate(),
            Some(expires_at),
            7200,
        ).unwrap();
        
        // When
        let is_expired = permission.is_expired();
        
        // Then
        assert!(is_expired);
    }
}
```

**Property-Based Testing Example:**
```rust
#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn email_value_object_accepts_valid_formats(
            local in "[a-z]{3,10}",
            domain in "[a-z]{3,10}",
            tld in "[a-z]{2,4}"
        ) {
            let email_str = format!("{}@{}.{}", local, domain, tld);
            let result = Email::new(&email_str);
            prop_assert!(result.is_ok());
        }
        
        #[test]
        fn filename_rejects_path_traversal(s in ".*\\.\\..*") {
            let result = FileName::new(&s);
            prop_assert!(result.is_err());
        }
    }
}
```

---

### 2. Application Level - Integration Tests

**What:** Commands and queries with real dependencies (database, filesystem).

**Scope:**
- Command handlers
- Query handlers
- Application services
- Event handlers

**Tools:**
- `cargo test` with `#[tokio::test]`
- `testcontainers` - Docker containers for PostgreSQL
- `tempfile` - Temporary directories for filesystem tests
- Test fixtures and builders

**Characteristics:**
- ✅ Medium speed (seconds per test)
- ✅ Real database (PostgreSQL in Docker)
- ✅ Real filesystem (temporary directories)
- ✅ Transactional (rollback after each test)
- ✅ Isolated (each test gets fresh database)

**Test Structure:**
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use testcontainers::*;
    
    struct TestContext {
        db: PgConnection,
        file_system: TempDir,
        event_bus: InMemoryEventBus,
        handler: RegisterUserCommandHandler,
    }
    
    impl TestContext {
        async fn new() -> Self {
            // Setup test database
            let docker = clients::Cli::default();
            let postgres = docker.run(images::postgres::Postgres::default());
            let db = PgConnection::connect(&postgres.connection_string()).await.unwrap();
            sqlx::migrate!().run(&db).await.unwrap();
            
            // Setup test filesystem
            let file_system = TempDir::new().unwrap();
            
            // Setup dependencies
            let user_repository = PostgresUserRepository::new(db.clone());
            let event_bus = InMemoryEventBus::new();
            let filesystem = LocalFileSystem::new(file_system.path());
            
            let handler = RegisterUserCommandHandler {
                user_repository: Arc::new(user_repository),
                event_publisher: Arc::new(event_bus.clone()),
                filesystem: Arc::new(filesystem),
            };
            
            Self {
                db,
                file_system,
                event_bus,
                handler,
            }
        }
    }
    
    #[tokio::test]
    async fn register_user_command_creates_user_and_folder() {
        // Given
        let ctx = TestContext::new().await;
        let command = RegisterUserCommand {
            email: Email::new("john@example.com").unwrap(),
            role: UserRole::Owner,
            storage_quota_bytes: 10_000_000_000,
            local_root_folder: ctx.file_system.path().join("john"),
            created_by_admin_id: UserId::generate(),
        };
        
        // When
        let result = ctx.handler.handle(command).await;
        
        // Then - Success
        assert!(result.is_ok());
        let user_id = result.unwrap();
        
        // Then - User persisted in database
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&ctx.db)
            .await
            .unwrap();
        assert_eq!(user.email.as_str(), "john@example.com");
        assert_eq!(user.role, UserRole::Owner);
        
        // Then - Folder created on filesystem
        assert!(ctx.file_system.path().join("john").exists());
        
        // Then - Domain event published
        let events = ctx.event_bus.published_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DomainEvent::UserRegistered { .. }));
    }
}
```

---

### 3. Acceptance Criteria Tests

**What:** High-level scenarios that validate business requirements.

**Format:** Given-When-Then (Gherkin-style, but in Rust)

**Example:**
```rust
#[tokio::test]
async fn acceptance_owner_can_revoke_permission_and_client_loses_access_immediately() {
    // GIVEN an owner with a file
    let ctx = TestContext::new().await;
    let owner = ctx.create_owner("owner@example.com").await;
    let file = ctx.upload_file(&owner.id, "contract.pdf", b"content").await;
    
    // AND a client with permission to view the file
    let client = ctx.create_client("client@example.com").await;
    let permission = ctx.grant_permission(&owner.id, &client.id, &file.id).await;
    
    // AND the client has an active session viewing the file
    let session = ctx.start_session(&client.id).await;
    assert!(ctx.can_access_file(&session.id, &file.id).await);
    
    // WHEN the owner revokes the permission
    let revoke_cmd = RevokePermissionCommand {
        permission_id: permission.id,
        revoked_by: owner.id,
    };
    ctx.execute_command(revoke_cmd).await.unwrap();
    
    // THEN the client immediately loses access to the file
    // (Landlock policy updated, file blocked at kernel level)
    assert!(!ctx.can_access_file(&session.id, &file.id).await);
    
    // AND the permission is marked as revoked in database
    let updated_permission = ctx.get_permission(&permission.id).await;
    assert!(updated_permission.is_revoked());
    
    // AND an audit log entry is created
    let audit_log = ctx.get_audit_logs_for_permission(&permission.id).await;
    assert!(audit_log.iter().any(|e| e.action == "PermissionRevoked"));
    
    // AND the client's UI is notified via WebSocket
    let ws_messages = ctx.get_websocket_messages_for_session(&session.id).await;
    assert!(ws_messages.iter().any(|m| matches!(m, WsMessage::PermissionRevoked { .. })));
}
```

---

## Acceptance Criteria Format

Each command and query MUST define acceptance criteria in this format:

```rust
/// # Acceptance Criteria
///
/// ## AC1: Happy Path - Valid Input
/// **GIVEN** [preconditions]
/// **WHEN** [action]
/// **THEN** [expected outcomes]
///
/// ## AC2: Error Case - Invalid Input
/// **GIVEN** [preconditions]
/// **WHEN** [action with invalid data]
/// **THEN** [error returned, state unchanged]
///
/// ## AC3: Security - Unauthorized Access
/// **GIVEN** [preconditions]
/// **WHEN** [unauthorized user attempts action]
/// **THEN** [error returned, audit log created]
///
/// ## AC4: Side Effects - Events Published
/// **GIVEN** [preconditions]
/// **WHEN** [action]
/// **THEN** [domain events published, event handlers triggered]
```

---

## Testing Requirements by Command/Query

### Commands (Write Operations)

Each command MUST test:

1. **Happy Path** - Valid input, successful execution
2. **Validation Failures** - Each validation rule must have a failing test
3. **Authorization Failures** - Unauthorized users blocked
4. **Domain Events** - Correct events emitted
5. **Side Effects** - Database persisted, filesystem updated, etc.
6. **Idempotency** - Can retry safely (if applicable)
7. **Concurrency** - Handle concurrent execution (if applicable)
8. **Audit Logging** - Every command creates audit entry

**Minimum Test Coverage:** 90% for command handlers

### Queries (Read Operations)

Each query MUST test:

1. **Happy Path** - Valid input, correct data returned
2. **Empty Results** - No data found (not an error)
3. **Filtering** - Query parameters correctly filter results
4. **Pagination** - Correct page size, offset, total count
5. **Sorting** - Results in correct order
6. **Authorization** - Users only see their own data
7. **Performance** - Query completes within SLA (e.g., < 100ms)

**Minimum Test Coverage:** 80% for query handlers

---

## Test Organization

```
src/
├── domain/
│   ├── aggregates/
│   │   ├── user.rs
│   │   └── user_tests.rs        # Unit tests
│   ├── value_objects/
│   │   ├── email.rs
│   │   └── email_tests.rs       # Unit tests + property tests
│
├── application/
│   ├── commands/
│   │   ├── register_user.rs
│   │   └── register_user_tests.rs  # Integration tests + acceptance criteria
│   ├── queries/
│   │   ├── get_user_files.rs
│   │   └── get_user_files_tests.rs
│
tests/
├── integration/                 # Full integration tests
│   ├── auth_flow.rs
│   ├── file_management.rs
│   ├── permission_management.rs
│   └── session_lifecycle.rs
│
├── acceptance/                  # Acceptance criteria tests
│   ├── owner_workflows.rs
│   ├── client_workflows.rs
│   └── admin_workflows.rs
│
└── fixtures/                    # Test data builders
    ├── user_builder.rs
    ├── file_builder.rs
    └── permission_builder.rs
```

---

## Test Fixtures & Builders

**Builder Pattern for Test Data:**

```rust
pub struct UserBuilder {
    email: Option<Email>,
    role: Option<UserRole>,
    storage_quota_bytes: u64,
}

impl UserBuilder {
    pub fn new() -> Self {
        Self {
            email: None,
            role: None,
            storage_quota_bytes: 10_000_000_000,
        }
    }
    
    pub fn with_email(mut self, email: &str) -> Self {
        self.email = Some(Email::new(email).unwrap());
        self
    }
    
    pub fn as_owner(mut self) -> Self {
        self.role = Some(UserRole::Owner);
        self
    }
    
    pub fn as_client(mut self) -> Self {
        self.role = Some(UserRole::Client);
        self
    }
    
    pub fn with_quota_gb(mut self, gb: u64) -> Self {
        self.storage_quota_bytes = gb * 1_000_000_000;
        self
    }
    
    pub fn build(self) -> User {
        User::register(
            UserId::generate(),
            self.email.expect("Email required"),
            self.role.expect("Role required"),
            self.storage_quota_bytes,
        ).unwrap()
    }
}

// Usage in tests:
let owner = UserBuilder::new()
    .with_email("owner@example.com")
    .as_owner()
    .with_quota_gb(100)
    .build();
```

---

## CI/CD Integration

### GitHub Actions Workflow

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Run unit tests
        run: cargo test --lib
      
      - name: Run integration tests
        run: cargo test --test '*'
        env:
          DATABASE_URL: postgresql://postgres:test@localhost/test
      
      - name: Check test coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml --exclude-files 'tests/*'
      
      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml
```

---

## Performance Testing

### Load Testing for Queries

```rust
#[tokio::test]
async fn query_user_files_completes_within_100ms() {
    let ctx = TestContext::new().await;
    let owner = ctx.create_owner("owner@example.com").await;
    
    // Create 1000 files
    for i in 0..1000 {
        ctx.upload_file(&owner.id, &format!("file_{}.txt", i), b"content").await;
    }
    
    // Measure query time
    let start = Instant::now();
    let query = GetUserFilesQuery {
        user_id: owner.id,
        path: "/".into(),
        limit: 50,
        offset: 0,
    };
    let result = ctx.execute_query(query).await.unwrap();
    let duration = start.elapsed();
    
    // Assert performance
    assert!(duration < Duration::from_millis(100), 
        "Query took {:?}, expected < 100ms", duration);
    assert_eq!(result.files.len(), 50);
}
```

---

## Security Testing

### Authorization Tests

```rust
#[tokio::test]
async fn client_cannot_access_file_without_permission() {
    let ctx = TestContext::new().await;
    let owner = ctx.create_owner("owner@example.com").await;
    let file = ctx.upload_file(&owner.id, "secret.pdf", b"confidential").await;
    
    let attacker = ctx.create_client("attacker@example.com").await;
    
    // Attacker tries to access file without permission
    let result = ctx.download_file(&attacker.id, &file.id).await;
    
    // Should fail with Unauthorized
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DomainError::Unauthorized);
    
    // Audit log should record attempted access
    let audit_logs = ctx.get_audit_logs_for_file(&file.id).await;
    assert!(audit_logs.iter().any(|e| {
        e.action == "UnauthorizedFileAccess" && e.actor_id == attacker.id
    }));
}
```

### Injection Tests

```rust
#[test]
fn filename_value_object_prevents_sql_injection() {
    let malicious_names = vec![
        "'; DROP TABLE users; --",
        "../../../etc/passwd",
        "<script>alert('xss')</script>",
        "../../secrets.txt",
    ];
    
    for name in malicious_names {
        let result = FileName::new(name);
        assert!(result.is_err(), "Should reject malicious filename: {}", name);
    }
}
```

---

## Test Coverage Requirements

| Layer | Minimum Coverage | Critical Paths Coverage |
|-------|------------------|------------------------|
| Domain (aggregates, value objects) | 95% | 100% |
| Application (commands, queries) | 90% | 100% |
| Infrastructure (repositories, adapters) | 80% | 90% |
| Overall | 85% | 95% |

**Critical Paths:**
- Authentication (WebAuthn)
- Permission enforcement (Landlock)
- File access control
- Audit logging
- Session management

---

## Example: Complete Command Test Suite

### RegisterUserCommand - Full Test Coverage

```rust
#[cfg(test)]
mod register_user_tests {
    use super::*;
    
    /// AC1: Happy Path - Valid User Registration
    #[tokio::test]
    async fn valid_user_can_be_registered() {
        // GIVEN a Super Admin
        let ctx = TestContext::new().await;
        let admin = ctx.create_super_admin().await;
        
        // WHEN registering a new user with valid data
        let command = RegisterUserCommand {
            email: Email::new("new@example.com").unwrap(),
            role: UserRole::Owner,
            storage_quota_bytes: 10_000_000_000,
            local_root_folder: ctx.temp_dir().join("new_user"),
            created_by_admin_id: admin.id,
        };
        let result = ctx.execute_command(command).await;
        
        // THEN the user is created
        assert!(result.is_ok());
        let user_id = result.unwrap();
        
        // AND persisted in database
        let user = ctx.get_user(&user_id).await.unwrap();
        assert_eq!(user.email.as_str(), "new@example.com");
        
        // AND folder created on filesystem
        assert!(ctx.temp_dir().join("new_user").exists());
        
        // AND domain event emitted
        ctx.assert_event_published(DomainEvent::UserRegistered { .. });
        
        // AND audit log created
        ctx.assert_audit_log_exists("UserRegistered", admin.id);
    }
    
    /// AC2: Validation - Duplicate Email Rejected
    #[tokio::test]
    async fn cannot_register_user_with_duplicate_email() {
        let ctx = TestContext::new().await;
        let admin = ctx.create_super_admin().await;
        
        // GIVEN an existing user
        ctx.create_owner("existing@example.com").await;
        
        // WHEN attempting to register with same email
        let command = RegisterUserCommand {
            email: Email::new("existing@example.com").unwrap(),
            role: UserRole::Client,
            storage_quota_bytes: 1_000_000_000,
            local_root_folder: ctx.temp_dir().join("duplicate"),
            created_by_admin_id: admin.id,
        };
        let result = ctx.execute_command(command).await;
        
        // THEN registration fails
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DomainError::EmailAlreadyExists);
        
        // AND no folder created
        assert!(!ctx.temp_dir().join("duplicate").exists());
        
        // AND no event emitted
        ctx.assert_no_events_published();
    }
    
    /// AC3: Authorization - Only Super Admin Can Register Users
    #[tokio::test]
    async fn only_super_admin_can_register_users() {
        let ctx = TestContext::new().await;
        let regular_owner = ctx.create_owner("owner@example.com").await;
        
        // WHEN a non-admin tries to register a user
        let command = RegisterUserCommand {
            email: Email::new("new@example.com").unwrap(),
            role: UserRole::Client,
            storage_quota_bytes: 1_000_000_000,
            local_root_folder: ctx.temp_dir().join("unauthorized"),
            created_by_admin_id: regular_owner.id,
        };
        let result = ctx.execute_command(command).await;
        
        // THEN the command is rejected
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DomainError::Unauthorized);
        
        // AND audit log records the attempt
        ctx.assert_audit_log_exists("UnauthorizedUserRegistration", regular_owner.id);
    }
    
    /// AC4: Validation - Zero Quota Rejected
    #[tokio::test]
    async fn cannot_register_user_with_zero_quota() {
        let ctx = TestContext::new().await;
        let admin = ctx.create_super_admin().await;
        
        let command = RegisterUserCommand {
            email: Email::new("new@example.com").unwrap(),
            role: UserRole::Owner,
            storage_quota_bytes: 0,  // Invalid
            local_root_folder: ctx.temp_dir().join("user"),
            created_by_admin_id: admin.id,
        };
        let result = ctx.execute_command(command).await;
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DomainError::InvalidStorageQuota);
    }
}
```

---

## Running Tests

```bash
# Run all tests
cargo test

# Run only unit tests (domain layer)
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run tests for specific module
cargo test commands::register_user

# Run with output
cargo test -- --nocapture

# Run with coverage
cargo tarpaulin --out Html

# Run performance tests
cargo test --release perf_

# Run in parallel (default)
cargo test -- --test-threads=4

# Run serially (for debugging)
cargo test -- --test-threads=1
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related Documents:** [COMMANDS.md](COMMANDS.md), [QUERIES.md](QUERIES.md), [ARCHITECTURE.md](ARCHITECTURE.md)
