# InitiateWebAuthnRegistrationCommand

**Purpose:** Client initiates WebAuthn/FIDO2 credential registration (hardware key, passkey, etc.).

**Persona:** Client

**Module:** `application::client::commands::initiate_webauthn_registration`

---

## Command Structure

```rust
pub struct InitiateWebAuthnRegistrationCommand {
    pub user_id: UserId,
    pub credential_name: String,  // User-friendly name like "YubiKey 5C" or "iPhone Passkey"
}
```

---

## Validations

- ✅ user_id is authenticated
- ✅ credential_name is not empty and <= 100 chars
- ✅ user doesn't exceed max credentials (10)

---

## Acceptance Criteria

### AC1: Happy Path - Initiate Registration
**GIVEN** a Client is authenticated  
**WHEN** InitiateWebAuthnRegistrationCommand is executed with credential_name="YubiKey 5C"  
**THEN** WebAuthn challenge is generated  
**AND** Challenge is stored temporarily (5 min TTL)  
**AND** PublicKeyCredentialCreationOptions returned to client  
**AND** HTTP response 200 OK with options for navigator.credentials.create()

### AC2: Challenge Format - WebAuthn Standard
**GIVEN** a registration is initiated  
**THEN** response includes:
- challenge: Random 32-byte value (base64url)
- rp: { id: "domain.com", name: "Secure Sandbox" }
- user: { id: user_id (base64url), name: email, displayName: email }
- pubKeyCredParams: [ES256, RS256]
- authenticatorSelection: { requireResidentKey: true, userVerification: "required" }
- timeout: 300000 (5 minutes)

### AC3: Max Credentials - Registration Rejected
**GIVEN** a Client already has 10 registered credentials  
**WHEN** InitiateWebAuthnRegistrationCommand is executed  
**THEN** command fails with `DomainError::MaxCredentialsExceeded`

---

## Handler Implementation

```rust
impl CommandHandler<InitiateWebAuthnRegistrationCommand> for InitiateWebAuthnRegistrationCommandHandler {
    async fn handle(&self, cmd: InitiateWebAuthnRegistrationCommand) -> Result<PublicKeyCredentialCreationOptions, DomainError> {
        // 1. Get user
        let user = self.user_repository
            .find_by_id(&cmd.user_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;
        
        // 2. Check credential limit
        let credential_count = self.webauthn_credential_repository
            .count_by_user(&cmd.user_id)
            .await?;
        
        if credential_count >= 10 {
            return Err(DomainError::MaxCredentialsExceeded);
        }
        
        // 3. Generate challenge
        let challenge = self.webauthn_service.generate_challenge();
        
        // 4. Store challenge temporarily (Redis, 5 min TTL)
        let challenge_id = Uuid::new_v4();
        self.challenge_store.store(
            &challenge_id,
            &challenge,
            &cmd.user_id,
            Duration::from_secs(300),
        ).await?;
        
        // 5. Create PublicKeyCredentialCreationOptions
        let options = PublicKeyCredentialCreationOptions {
            challenge: challenge.to_base64url(),
            rp: RelyingParty {
                id: self.config.domain.clone(),
                name: "Secure Sandbox".to_string(),
            },
            user: UserEntity {
                id: cmd.user_id.to_base64url(),
                name: user.email.to_string(),
                display_name: user.email.to_string(),
            },
            pub_key_cred_params: vec![
                PubKeyCredParam { type_: "public-key", alg: -7 },  // ES256
                PubKeyCredParam { type_: "public-key", alg: -257 }, // RS256
            ],
            authenticator_selection: AuthenticatorSelection {
                authenticator_attachment: None,  // Allow both platform and cross-platform
                require_resident_key: true,
                resident_key: "required",
                user_verification: "required",
            },
            timeout: 300000,  // 5 minutes
            attestation: "direct",
            extensions: None,
        };
        
        Ok(options)
    }
}
```

---

## API Endpoint

```http
POST /api/client/auth/webauthn/register/initiate
Authorization: Bearer {client_jwt_token}
Content-Type: application/json

Request Body:
{
  "credential_name": "YubiKey 5C"
}

Response 200 OK:
{
  "challenge_id": "550e8400-e29b-41d4-a716-446655440000",
  "publicKey": {
    "challenge": "abc123...",
    "rp": {
      "id": "domain.com",
      "name": "Secure Sandbox"
    },
    "user": {
      "id": "usr_123_base64",
      "name": "client@example.com",
      "displayName": "client@example.com"
    },
    "pubKeyCredParams": [
      { "type": "public-key", "alg": -7 },
      { "type": "public-key", "alg": -257 }
    ],
    "authenticatorSelection": {
      "requireResidentKey": true,
      "residentKey": "required",
      "userVerification": "required"
    },
    "timeout": 300000,
    "attestation": "direct"
  }
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [CompleteWebAuthnRegistrationCommand](complete_webauthn_registration.md), [InitiateWebAuthnAuthenticationCommand](initiate_webauthn_authentication.md)
