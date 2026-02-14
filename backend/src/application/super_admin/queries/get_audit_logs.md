# GetAuditLogsQuery

**Purpose:** Super Admin retrieves audit logs with filtering and search.

**Persona:** Super Admin

**Module:** `application::super_admin::queries::get_audit_logs`

---

## Query Structure

```rust
pub struct GetAuditLogsQuery {
    pub actor_id: Option<UserId>,           // Filter by who performed action
    pub target_user_id: Option<UserId>,     // Filter by target user
    pub action_filter: Option<Vec<String>>, // e.g., ["UserRegistered", "PermissionRevoked"]
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub severity: Option<AuditSeverity>,    // Info, Warning, Critical
    pub search: Option<String>,              // Search in action or metadata
    pub page: u32,
    pub page_size: u32,
}

pub enum AuditSeverity {
    Info,
    Warning,
    Critical,
}
```

---

## Response Structure

```rust
pub struct GetAuditLogsQueryResult {
    pub logs: Vec<AuditLogEntry>,
    pub total_count: u64,
    pub page: u32,
    pub page_size: u32,
}

pub struct AuditLogEntry {
    pub id: AuditLogId,
    pub timestamp: DateTime<Utc>,
    pub actor_id: Option<UserId>,
    pub actor_email: Option<String>,
    pub action: String,
    pub target_user_id: Option<UserId>,
    pub target_email: Option<String>,
    pub resource_type: Option<String>,     // File, Permission, Session
    pub resource_id: Option<String>,
    pub result: AuditResult,               // Success, Failure
    pub severity: AuditSeverity,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,       // Additional context
}
```

---

## Acceptance Criteria

### AC1: Happy Path - Get Recent Audit Logs
**GIVEN** a Super Admin is authenticated  
**AND** database has 10,000 audit log entries  
**WHEN** GetAuditLogsQuery is executed with page=1, page_size=100  
**THEN** query returns 100 most recent logs ordered by timestamp desc  
**AND** each log includes: timestamp, actor, action, result, IP address

### AC2: Filter By Actor
**GIVEN** audit logs exist for multiple users  
**WHEN** GetAuditLogsQuery is executed with actor_id=usr_123  
**THEN** query returns only logs where actor_id = usr_123

### AC3: Filter By Date Range
**GIVEN** audit logs from past 30 days  
**WHEN** GetAuditLogsQuery is executed with:
- start_date: 7 days ago
- end_date: now
**THEN** query returns only logs from last 7 days

### AC4: Filter By Action Type
**GIVEN** various audit log actions  
**WHEN** GetAuditLogsQuery is executed with action_filter=["PermissionRevoked", "UserDeleted"]  
**THEN** query returns only logs with those actions

### AC5: Filter By Severity
**GIVEN** audit logs with different severities  
**WHEN** GetAuditLogsQuery is executed with severity=Critical  
**THEN** query returns only critical security events

### AC6: Search in Metadata
**GIVEN** audit logs with various metadata  
**WHEN** GetAuditLogsQuery is executed with search="contract.pdf"  
**THEN** query returns logs where metadata contains "contract.pdf"

### AC7: Performance - Query Within 300ms
**GIVEN** 100,000 audit log entries  
**WHEN** GetAuditLogsQuery is executed  
**THEN** query completes within 300ms using database indexes

---

## API Endpoint

```http
GET /api/admin/audit-logs?actor_id=usr_123&action=PermissionRevoked&start_date=2026-02-07T00:00:00Z&severity=Critical&page=1&page_size=100
Authorization: Bearer {super_admin_jwt_token}

Response 200 OK:
{
  "logs": [
    {
      "id": "aud_789",
      "timestamp": "2026-02-14T10:25:00Z",
      "actor_id": "usr_123",
      "actor_email": "owner@example.com",
      "action": "PermissionRevoked",
      "target_user_id": "usr_456",
      "target_email": "client@example.com",
      "resource_type": "Permission",
      "resource_id": "prm_789",
      "result": "Success",
      "severity": "Warning",
      "ip_address": "192.168.1.100",
      "user_agent": "Mozilla/5.0...",
      "metadata": {
        "file_id": "fil_123",
        "file_path": "/Documents/contract.pdf",
        "reason": "Access no longer needed"
      }
    }
  ],
  "total_count": 523,
  "page": 1,
  "page_size": 100
}
```

---

## Database Indexes

```sql
CREATE INDEX idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX idx_audit_logs_actor ON audit_logs(actor_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_severity ON audit_logs(severity);
CREATE INDEX idx_audit_logs_metadata_gin ON audit_logs USING gin(metadata jsonb_path_ops);
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [TRACEABILITY.md](../../../docs/TRACEABILITY.md)
