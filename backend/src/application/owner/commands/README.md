# Owner Commands

All commands executed by **Owner** persona.

---

## File Management

- [UploadFileCommand](upload_file.md) - Upload a file to secure storage
- [DownloadFileCommand](download_file.md) - Download a file from storage
- [DeleteFileCommand](delete_file.md) - Delete a file (soft delete with 30-day retention)
- [MoveFileCommand](move_file.md) - Move file to different folder
- [RenameFileCommand](rename_file.md) - Rename a file
- [CreateFolderCommand](create_folder.md) - Create a new folder

## Permission Management

- [GrantPermissionCommand](grant_permission.md) - Grant file access to a client user
- [RevokePermissionCommand](revoke_permission.md) - Revoke file access from a client user
- [UpdatePermissionExpirationCommand](update_permission_expiration.md) - Change permission expiration date

## Session Management

- [TerminateSessionCommand](terminate_session.md) - Terminate a client's active session
- [UpdateSessionSettingsCommand](update_session_settings.md) - Change session settings (watermark, timeout)

## Access Request Management

- [ApproveAccessRequestCommand](approve_access_request.md) - Approve client's access request
- [DenyAccessRequestCommand](deny_access_request.md) - Deny client's access request

## Invitation Management

- [CreateInvitationCommand](create_invitation.md) - Create invitation link for client
- [RevokeInvitationCommand](revoke_invitation.md) - Revoke an invitation

---

**Persona:** Owner (Data Owner)  
**Capabilities:** Manage files, grant/revoke permissions, monitor client activity  
**Authorization:** Must own the resources being managed
