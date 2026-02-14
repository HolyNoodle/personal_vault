# Super Admin Queries

All queries executed by **Super Admin** persona.

---

## User Management

- [GetAllUsersQuery](get_all_users.md) - List all users with filters and pagination
- [GetUserDetailsQuery](get_user_details.md) - Get detailed information about a specific user
- [GetUserStorageUsageQuery](get_user_storage_usage.md) - Get storage usage breakdown for a user

## System Monitoring

- [GetSystemStatsQuery](get_system_stats.md) - Get system-wide statistics (users, files, sessions)
- [GetActiveSessionsQuery](get_active_sessions.md) - List all active sessions across all users
- [GetSystemHealthQuery](get_system_health.md) - System health metrics (CPU, memory, disk)

## Auditing & Compliance

- [GetAuditLogsQuery](get_audit_logs.md) - Retrieve audit logs with filters
- [GetSecurityEventsQuery](get_security_events.md) - Get security-related events (failed logins, unauthorized access)
- [GenerateComplianceReportQuery](generate_compliance_report.md) - Generate GDPR/compliance report

---

**Persona:** Super Admin  
**Authorization:** Requires SuperAdmin role
