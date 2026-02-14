# Super Admin Commands

All commands executed by **Super Admin** persona.

---

## User Management

- [RegisterUserCommand](register_user.md) - Create new Owner or Client user account
- [DisableUserCommand](disable_user.md) - Disable a user account
- [EnableUserCommand](enable_user.md) - Re-enable a disabled user account
- [UpdateUserQuotaCommand](update_user_quota.md) - Modify user's storage quota
- [DeleteUserCommand](delete_user.md) - Permanently delete a user account

## System Management

- [ConfigureSystemSettingsCommand](configure_system_settings.md) - Update system-wide settings
- [TriggerCleanupCommand](trigger_cleanup.md) - Manually trigger trash cleanup
- [ForceTerminateSessionCommand](force_terminate_session.md) - Terminate any user's session

## Monitoring & Auditing

- [GenerateAuditReportCommand](generate_audit_report.md) - Generate compliance audit report
- [ExportUserDataCommand](export_user_data.md) - Export all data for a user (GDPR compliance)

---

**Persona:** Super Admin  
**Capabilities:** Full system access, user provisioning, system configuration  
**Authorization:** Highest privilege level (limited to 2-3 users)  
**Security:** Requires WebAuthn hardware key authentication
