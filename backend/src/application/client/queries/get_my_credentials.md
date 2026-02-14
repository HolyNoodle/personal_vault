# GetMyCredentialsQuery

**Purpose:** Client retrieves all registered WebAuthn credentials (hardware keys, passkeys).

**Persona:** Client

**Module:** `application::client::queries::get_my_credentials`

---

## Query Structure

```rust
pub struct GetMyCredentialsQuery {
    pub user_id: UserId,
}
```

---

## Response Structure

```rust
pub struct GetMyCredentialsQueryResult {
    pub credentials: Vec<WebAuthnCredentialSummary>,
    pub total_count: u64,
}

pub struct WebAuthnCredentialSummary {
    pub credential_id: CredentialId,
    pub credential_name: String,
    pub aaguid: String,                      // Authenticator type identifier
    pub authenticator_type: String,          // "YubiKey", "iPhone", "Android", etc.
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub sign_count: u32,                     // Usage counter for security
}
```

---

## Acceptance Criteria

### AC1: Happy Path - List All Credentials
**GIVEN** a Client has 3 registered credentials  
**WHEN** GetMyCredentialsQuery is executed  
**THEN** query returns all 3 credentials  
**AND** sorted by created_at desc (newest first)

### AC2: Authenticator Type Detection
**GIVEN** credentials include YubiKey and iPhone passkey  
**WHEN** GetMyCredentialsQuery is executed  
**THEN** YubiKey has authenticator_type="YubiKey"  
**AND** iPhone has authenticator_type="iPhone"  
**AND** based on AAGUID mapping

### AC3: Usage Tracking
**GIVEN** a credential used 5 times  
**WHEN** GetMyCredentialsQuery is executed  
**THEN** credential shows sign_count=5  
**AND** last_used_at shows most recent authentication

### AC4: No Credentials - Empty List
**GIVEN** a Client has no registered credentials  
**WHEN** GetMyCredentialsQuery is executed  
**THEN** query returns empty array  
**AND** total_count=0

---

## API Endpoint

```http
GET /api/client/auth/credentials
Authorization: Bearer {client_jwt_token}

Response 200 OK:
{
  "credentials": [
    {
      "credential_id": "cred_123",
      "credential_name": "YubiKey 5C",
      "aaguid": "f8a011f3-8c0a-4d15-8006-17111f9edc7d",
      "authenticator_type": "YubiKey",
      "created_at": "2026-01-15T10:00:00Z",
      "last_used_at": "2026-02-14T09:30:00Z",
      "sign_count": 42
    },
    {
      "credential_id": "cred_456",
      "credential_name": "iPhone Passkey",
      "aaguid": "00000000-0000-0000-0000-000000000000",
      "authenticator_type": "Platform Authenticator",
      "created_at": "2026-02-01T14:00:00Z",
      "last_used_at": null,
      "sign_count": 0
    }
  ],
  "total_count": 2
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [InitiateWebAuthnRegistrationCommand](../commands/initiate_webauthn_registration.md)
