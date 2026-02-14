# Client Queries

All queries executed by **Client** persona.

---

## File Access

- [GetMyAccessibleFilesQuery](get_my_accessible_files.md) - List all files the client has permission to access
- [GetFileDetailsQuery](get_file_details.md) - Get details about a file the client can access

## Session Monitoring

- [GetMyActiveSessionQuery](get_my_active_session.md) - Get current active session (if any)
- [GetMySessionHistoryQuery](get_my_session_history.md) - Get historical session data

## Access Requests

- [GetMyAccessRequestsQuery](get_my_access_requests.md) - List all access requests (pending, approved, denied)
- [GetPendingAccessRequestsQuery](get_pending_access_requests.md) - List only pending access requests

## Invitations

- [GetMyInvitationsQuery](get_my_invitations.md) - List all invitations received

## Authentication

- [GetMyCredentialsQuery](get_my_credentials.md) - List registered WebAuthn credentials (hardware keys, passkeys)

---

**Persona:** Client  
**Authorization:** Requires Client role
