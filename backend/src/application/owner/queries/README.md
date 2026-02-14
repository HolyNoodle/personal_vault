# Owner Queries

All queries executed by **Owner** persona.

---

## File Management

- [GetMyFilesQuery](get_my_files.md) - List all files owned by the user with filtering and search
- [GetFileDetailsQuery](get_file_details.md) - Get detailed information about a specific file
- [GetFolderContentsQuery](get_folder_contents.md) - List files and subfolders in a folder
- [GetStorageUsageQuery](get_storage_usage.md) - Get storage usage breakdown by folder/file type

## Permission Management

- [GetFilePermissionsQuery](get_file_permissions.md) - List all permissions granted for a specific file
- [GetAllPermissionsQuery](get_all_permissions.md) - List all permissions granted by the owner

## Session Monitoring

- [GetActiveSessionsQuery](get_active_sessions.md) - List all active sessions accessing owner's files
- [GetSessionHistoryQuery](get_session_history.md) - Get historical session data with filters

## Invitations

- [GetInvitationsQuery](get_invitations.md) - List all invitations created by the owner

## Access Requests

- [GetPendingAccessRequestsQuery](get_pending_access_requests.md) - List pending access requests requiring approval
- [GetAccessRequestHistoryQuery](get_access_request_history.md) - Get all access requests (pending, approved, denied)

## Audit & Activity

- [GetFileActivityQuery](get_file_activity.md) - Get activity log for a specific file (who accessed, when)
- [GetMyAuditLogsQuery](get_my_audit_logs.md) - Get audit logs for actions performed by/on owner's resources

---

**Persona:** Owner  
**Authorization:** Requires Owner role
