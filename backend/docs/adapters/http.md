````markdown
# HTTP Adapter (REST API)

**Purpose**: Expose the application layer via RESTful HTTP endpoints.

**Technology**: Actix-Web or Axum (Rust web framework)

**Layer**: Adapters (Primary/Driving Adapter)

---

## Responsibilities

- Receive HTTP requests from clients (web UI, mobile apps)
- Deserialize/validate request payloads
- Map HTTP requests to application commands/queries
- Execute commands/queries via application services
- Serialize responses (JSON)
- Handle HTTP errors and status codes
- Implement CORS policies
- Rate limiting per user/IP
- Request logging

---

## Dependencies

### Required Crates
```toml
[dependencies]
# Web framework (choose one)
actix-web = "4.4"
# OR
axum = "0.7"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Validation
validator = { version = "0.16", features = ["derive"] }

# Error handling
thiserror = "1.0"

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# CORS
actix-cors = "0.7"  # for actix-web
# OR
tower-http = { version = "0.5", features = ["cors"] }  # for axum

# JWT tokens
jsonwebtoken = "9.2"

# UUID
uuid = { version = "1.6", features = ["v4", "serde"] }
```

---

## API Structure

### Persona-Based Routing

```
/api/super-admin/*   - SuperAdmin endpoints
/api/owner/*         - Owner endpoints
/api/client/*        - Client endpoints
/api/auth/*          - Authentication (all personas)
/api/health          - Health check
```

### Example Routes

#### SuperAdmin
```
POST   /api/super-admin/users/register
DELETE /api/super-admin/users/{user_id}
GET    /api/super-admin/stats
GET    /api/super-admin/audit-logs
```

#### Owner
```
POST   /api/owner/files
GET    /api/owner/files
GET    /api/owner/files/{file_id}/download
DELETE /api/owner/files/{file_id}
POST   /api/owner/permissions
GET    /api/owner/sessions/active
```

#### Client
```
POST   /api/client/sessions
DELETE /api/client/sessions/{session_id}
GET    /api/client/files/accessible
POST   /api/client/access-requests
```

#### Authentication
```
POST   /api/auth/webauthn/register/initiate
POST   /api/auth/webauthn/register/complete
POST   /api/auth/webauthn/login/initiate
POST   /api/auth/webauthn/login/complete
```

---

## Request/Response Format

### Request Body (JSON)
```json
{
  "file_name": "report.pdf",
  "parent_folder_id": "fld_123"
}
```

### Success Response
```json
{
  "success": true,
  "data": {
    "file_id": "fil_789",
    "uploaded_at": "2026-02-14T10:30:00Z"
  }
}
```

### Error Response
```json
{
  "success": false,
  "error": {
    "code": "STORAGE_QUOTA_EXCEEDED",
    "message": "Storage quota exceeded. You have used 9.5GB of 10GB.",
    "details": {
      "current_usage_bytes": 10200547328,
      "quota_bytes": 10737418240,
      "requested_bytes": 1073741824
    }
  }
}
```

---

## Authentication & Authorization

### JWT Token-Based
1. Client authenticates via WebAuthn → receives JWT token
2. Client includes token in Authorization header: `Bearer {jwt_token}`
3. Middleware validates JWT signature and expiration
4. Extracts user_id and role from JWT claims
5. Checks role matches endpoint requirements (SuperAdmin/Owner/Client)

### JWT Claims
```json
{
  "sub": "usr_123",           // user_id
  "email": "user@example.com",
  "role": "Owner",
  "iat": 1708000000,
  "exp": 1708003600          // 1 hour expiration
}
```

---

## Middleware Stack

1. **CORS** - Allow web UI origin
2. **Rate Limiting** - 100 req/min per user, 1000 req/min per IP
3. **Request Logging** - Log all requests (method, path, user, duration)
4. **JWT Authentication** - Validate token, extract user
5. **Error Handling** - Convert domain errors to HTTP responses

---

## Error Mapping

| Domain Error | HTTP Status | Error Code |
|--------------|-------------|------------|
| `UserNotFound` | 404 Not Found | `USER_NOT_FOUND` |
| `Unauthorized` | 403 Forbidden | `UNAUTHORIZED` |
| `PermissionDenied` | 403 Forbidden | `PERMISSION_DENIED` |
| `StorageQuotaExceeded` | 413 Payload Too Large | `STORAGE_QUOTA_EXCEEDED` |
| `DuplicateFileName` | 409 Conflict | `DUPLICATE_FILE_NAME` |
| `ValidationError` | 400 Bad Request | `VALIDATION_ERROR` |
| `SessionNotFound` | 404 Not Found | `SESSION_NOT_FOUND` |

---

## File Upload/Download

### Multipart Upload
```rust
#[post("/api/owner/files")]
async fn upload_file(
    user: AuthenticatedUser,
    mut payload: Multipart,
) -> Result<HttpResponse> {
    // Stream multipart data
    // Calculate SHA-256 checksum
    // Execute UploadFileCommand
}
```

### Streaming Download
```rust
#[get("/api/owner/files/{file_id}/download")]
async fn download_file(
    user: AuthenticatedUser,
    file_id: Path<FileId>,
) -> Result<HttpResponse> {
    // Execute DownloadFileCommand
    // Stream file content with chunked transfer encoding
    // Support HTTP Range requests for resume
}
```

---

## WebSocket Integration

Separate WebSocket endpoint for real-time notifications:
```
WS /api/ws
```

See [websocket.md](websocket.md) for details.

---

## Testing Strategy

### Unit Tests
- Request validation
- Error mapping
- JWT token validation

### Integration Tests
- Full HTTP request/response cycle
- Authentication flow
- File upload/download
- Error handling

### Example Test
```rust
#[tokio::test]
async fn test_upload_file_success() {
    let app = test::init_service(App::new().configure(configure_routes)).await;
    
    let req = test::TestRequest::post()
        .uri("/api/owner/files")
        .set_payload(multipart_payload())
        .insert_header(("Authorization", "Bearer valid_jwt"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
}
```

---

## Configuration

```toml
[http]
host = "0.0.0.0"
port = 8080
max_payload_size = 10737418240  # 10GB
request_timeout_seconds = 300
cors_allowed_origins = ["https://app.domain.com"]

[jwt]
secret = "env:JWT_SECRET"
expiration_seconds = 3600  # 1 hour
```

---

## Security Considerations

1. **HTTPS Only** - All traffic must use TLS (enforced by HAProxy)
2. **CSRF Protection** - Not needed (stateless JWT, no cookies)
3. **Content-Type Validation** - Reject unexpected content types
4. **Request Size Limits** - Max 10GB for file uploads, 1MB for JSON
5. **Rate Limiting** - Prevent brute force and DoS
6. **Input Validation** - Validate all inputs before processing
7. **SQL Injection** - Use parameterized queries (sqlx)
8. **XSS Prevention** - Sanitize outputs (JSON escaping)

---

## Performance Optimizations

1. **Connection Pooling** - Reuse database connections
2. **Async I/O** - Non-blocking request handling
3. **Streaming** - Stream large files without buffering
4. **Compression** - Gzip/Brotli for JSON responses (HAProxy)
5. **Caching** - Cache frequently accessed data (Redis)

---

**Document Version**: 1.0  
**Last Updated**: 2026-02-14  
**Related**: [webauthn.md](webauthn.md), [websocket.md](websocket.md)

````