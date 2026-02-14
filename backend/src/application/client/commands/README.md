# Client Commands

All commands executed by **Client** persona.

---

## Session Management

- [StartSessionCommand](start_session.md) - Start a sandbox session to view files
- [RequestSessionTerminationCommand](request_session_termination.md) - Client requests to end their own session

## Access Request Management

- [RequestAccessCommand](request_access.md) - Request access to owner's files
- [CancelAccessRequestCommand](cancel_access_request.md) - Cancel pending access request

## Authentication

- [InitiateWebAuthnRegistrationCommand](initiate_webauthn_registration.md) - Begin passkey registration
- [CompleteWebAuthnRegistrationCommand](complete_webauthn_registration.md) - Complete passkey registration
- [InitiateWebAuthnAuthenticationCommand](initiate_webauthn_authentication.md) - Begin WebAuthn login
- [CompleteWebAuthnAuthenticationCommand](complete_webauthn_authentication.md) - Complete WebAuthn login
- [InitiateMagicLinkAuthCommand](initiate_magic_link_auth.md) - Request magic link for login
- [CompleteMagicLinkAuthCommand](complete_magic_link_auth.md) - Verify magic link token

---

**Persona:** Client (Data Consumer)  
**Capabilities:** View files (via video feed), request access, manage own sessions  
**Authorization:** Can only access files explicitly granted by owner  
**Security:** All file access is kernel-level enforced via Landlock LSM
