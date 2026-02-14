````markdown
# WebAuthn Adapter (Passwordless Authentication)

**Purpose**: Implement WebAuthn/FIDO2 passwordless authentication using hardware keys and passkeys.

**Technology**: webauthn-rs crate + FIDO2 protocol

**Layer**: Adapters (Primary/Driving Adapter)

---

## Responsibilities

- Generate WebAuthn registration/authentication challenges
- Verify authenticator attestations (registration)
- Verify authenticator assertions (authentication)
- Store/retrieve public keys
- Manage credential lifecycle
- Support multiple authenticators per user
- Handle WebAuthn protocol edge cases

---

## Dependencies

### Required Crates
```toml
[dependencies]
webauthn-rs = "0.5"
webauthn-rs-proto = "0.5"
base64 = "0.21"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.6", features = ["v4", "serde"] }
sha2 = "0.10"
```

---

## WebAuthn Flow

### Registration Flow

1. **Initiate Registration** (`InitiateWebAuthnRegistrationCommand`)
   - Generate random challenge (32 bytes)
   - Store challenge temporarily (Redis, 5 min TTL)
   - Return PublicKeyCredentialCreationOptions to client
   
2. **Complete Registration** (`CompleteWebAuthnRegistrationCommand`)
   - Verify challenge matches stored challenge
   - Verify attestation signature
   - Verify origin matches expected domain
   - Extract public key from attestation object
   - Store credential (credential_id + public_key)

### Authentication Flow

1. **Initiate Authentication** (`InitiateWebAuthnAuthenticationCommand`)
   - Generate random challenge (32 bytes)
   - Retrieve user's registered credentials
   - Store challenge temporarily
   - Return PublicKeyCredentialRequestOptions with allowed credentials
   
2. **Complete Authentication** (`CompleteWebAuthnAuthenticationCommand`)
   - Verify challenge matches stored challenge
   - Verify assertion signature using stored public key
   - Verify sign count is incremented (clone detection)
   - Update last_used_at and sign_count
   - Issue JWT token

---

## Registration API

### Initiate
```http
POST /api/auth/webauthn/register/initiate
Authorization: Bearer {jwt_token}
Content-Type: application/json

Request:
{
  "credential_name": "YubiKey 5C"
}

Response:
{
  "challenge_id": "uuid",
  "publicKey": {
    "challenge": "base64url_encoded",
    "rp": { "id": "domain.com", "name": "Secure Sandbox" },
    "user": { "id": "base64_user_id", "name": "email", "displayName": "email" },
    "pubKeyCredParams": [
      { "type": "public-key", "alg": -7 },   // ES256
      { "type": "public-key", "alg": -257 }  // RS256
    ],
    "authenticatorSelection": {
      "requireResidentKey": true,
      "userVerification": "required"
    },
    "timeout": 300000
  }
}
```

### Complete
```http
POST /api/auth/webauthn/register/complete
Authorization: Bearer {jwt_token}
Content-Type: application/json

Request:
{
  "challenge_id": "uuid",
  "credential_name": "YubiKey 5C",
  "credential": {
    "id": "base64_credential_id",
    "rawId": "base64_raw_id",
    "response": {
      "attestationObject": "base64_cbor",
      "clientDataJSON": "base64_json"
    },
    "type": "public-key"
  }
}

Response:
{
  "credential_id": "cred_123",
  "credential_name": "YubiKey 5C",
  "registered_at": "2026-02-14T10:30:00Z"
}
```

---

## Authentication API

### Initiate
```http
POST /api/auth/webauthn/login/initiate
Content-Type: application/json

Request:
{
  "email": "user@example.com"
}

Response:
{
  "challenge_id": "uuid",
  "publicKey": {
    "challenge": "base64url_encoded",
    "rpId": "domain.com",
    "allowCredentials": [
      {
        "type": "public-key",
        "id": "base64_credential_id"
      }
    ],
    "userVerification": "required",
    "timeout": 300000
  }
}
```

### Complete
```http
POST /api/auth/webauthn/login/complete
Content-Type: application/json

Request:
{
  "challenge_id": "uuid",
  "credential": {
    "id": "base64_credential_id",
    "rawId": "base64_raw_id",
    "response": {
      "authenticatorData": "base64",
      "clientDataJSON": "base64_json",
      "signature": "base64_signature",
      "userHandle": "base64_user_id"
    },
    "type": "public-key"
  }
}

Response:
{
  "success": true,
  "token": "jwt_token",
  "user": {
    "user_id": "usr_123",
    "email": "user@example.com",
    "role": "Owner"
  }
}
```

---

## Data Model

### WebAuthnCredential Entity
```rust
pub struct WebAuthnCredential {
    pub id: CredentialId,
    pub user_id: UserId,
    pub credential_id: Vec<u8>,        // Raw credential ID
    pub public_key: Vec<u8>,           // COSE public key
    pub sign_count: u32,               // Usage counter (clone detection)
    pub aaguid: Uuid,                  // Authenticator GUID
    pub credential_name: String,       // User-friendly name
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_revoked: bool,
}
```

### Database Schema
```sql
CREATE TABLE webauthn_credentials (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    credential_id BYTEA NOT NULL UNIQUE,
    public_key BYTEA NOT NULL,
    sign_count INTEGER NOT NULL DEFAULT 0,
    aaguid UUID NOT NULL,
    credential_name VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,
    is_revoked BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_webauthn_credentials_user_id ON webauthn_credentials(user_id);
CREATE INDEX idx_webauthn_credentials_credential_id ON webauthn_credentials(credential_id);
```

---

## Challenge Storage (Redis)

Challenges are stored temporarily in Redis:

```rust
pub struct Challenge {
    pub challenge: Vec<u8>,
    pub user_id: Option<UserId>,  // None for login initiation
    pub created_at: DateTime<Utc>,
}

// Redis key: webauthn:challenge:{challenge_id}
// TTL: 5 minutes
```

---

## Verification Steps

### Registration Verification
1. ✅ Challenge exists and matches
2. ✅ Challenge not expired (< 5 minutes)
3. ✅ Client data type == "webauthn.create"
4. ✅ Origin matches expected origin
5. ✅ RP ID matches expected RP ID
6. ✅ Attestation signature is valid
7. ✅ Credential ID is unique (not already registered)
8. ✅ User presence verified (UP flag)
9. ✅ User verification performed (UV flag)

### Authentication Verification
1. ✅ Challenge exists and matches
2. ✅ Challenge not expired
3. ✅ Client data type == "webauthn.get"
4. ✅ Origin matches expected origin
5. ✅ RP ID matches expected RP ID
6. ✅ Credential exists and belongs to user
7. ✅ Assertion signature is valid (using stored public key)
8. ✅ Sign count incremented (clone detection)
9. ✅ User presence verified (UP flag)
10. ✅ User verification performed (UV flag)

---

## Clone Detection

Sign count must always increment:
- If sign_count decreases → credential cloned → reject authentication
- If sign_count == 0 → authenticator doesn't support counter → allow
- If sign_count increases → valid authentication → update stored value

```rust
if stored_sign_count > 0 && assertion_sign_count <= stored_sign_count {
    return Err(DomainError::CredentialCloned);
}
```

---

## Supported Authenticators

### Platform Authenticators
- ✅ Windows Hello
- ✅ macOS Touch ID
- ✅ iOS Face ID / Touch ID
- ✅ Android Biometrics

### Cross-Platform Authenticators
- ✅ YubiKey (all models)
- ✅ Google Titan Security Key
- ✅ Feitian ePass FIDO
- ✅ Solo Keys

---

## Testing Strategy

### Unit Tests
- Challenge generation and validation
- Attestation parsing and verification
- Assertion signature verification
- Sign count validation

### Integration Tests
- Full registration flow (happy path)
- Full authentication flow (happy path)
- Invalid attestation rejection
- Expired challenge rejection
- Clone detection

### Hardware Testing
- Test with real YubiKey
- Test with platform authenticators (Touch ID, Face ID)
- Test with multiple credentials per user

---

## Configuration

```toml
[webauthn]
rp_id = "domain.com"
rp_name = "Secure Sandbox"
origin = "https://domain.com"
challenge_ttl_seconds = 300
max_credentials_per_user = 10

[redis]
host = "localhost"
port = 6379
challenge_key_prefix = "webauthn:challenge:"
```

---

## Security Considerations

1. **Challenge Randomness** - Use cryptographically secure RNG (32 bytes)
2. **Challenge Single-Use** - Delete challenge after use
3. **Origin Validation** - Strictly validate origin (prevent phishing)
4. **Clone Detection** - Always check sign count
5. **Revocation** - Support credential revocation
6. **Attestation Validation** - Verify attestation statements
7. **User Verification** - Require UV=true (PIN, biometric, etc.)

---

## Error Handling

| Error | Description | Action |
|-------|-------------|--------|
| `ChallengeExpired` | Challenge older than 5 minutes | User must restart flow |
| `InvalidAttestation` | Attestation signature invalid | Registration rejected |
| `InvalidAssertion` | Assertion signature invalid | Authentication rejected |
| `CredentialCloned` | Sign count decreased | Block credential, alert user |
| `DuplicateCredential` | Credential ID already exists | Registration rejected |
| `MaxCredentialsExceeded` | User has 10+ credentials | Delete old credential first |

---

## FIDO2 Certification

To achieve FIDO2 certification:
1. Implement all mandatory WebAuthn features
2. Pass FIDO conformance tests
3. Submit to FIDO Alliance for review
4. Display FIDO Certified logo

**Status**: Implementation-ready, certification pending production deployment.

---

**Document Version**: 1.0  
**Last Updated**: 2026-02-14  
**Related**: [http.md](http.md), [PASSWORDLESS.md](../PASSWORDLESS.md)

````