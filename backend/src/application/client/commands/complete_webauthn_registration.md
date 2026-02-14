# CompleteWebAuthnRegistrationCommand

**Purpose:** Client completes WebAuthn credential registration by submitting the signed attestation.

**Persona:** Client

**Module:** `application::client::commands::complete_webauthn_registration`

---

## Command Structure

```rust
pub struct CompleteWebAuthnRegistrationCommand {
    pub user_id: UserId,
    pub challenge_id: Uuid,
    pub credential_name: String,
    pub credential_response: AuthenticatorAttestationResponse,
}

pub struct AuthenticatorAttestationResponse {
    pub id: String,                    // Credential ID (base64url)
    pub raw_id: Vec<u8>,               // Credential ID (raw bytes)
    pub response: AttestationResponse,
    pub type_: String,                 // "public-key"
}

pub struct AttestationResponse {
    pub attestation_object: Vec<u8>,   // CBOR-encoded
    pub client_data_json: Vec<u8>,     // JSON
}
```

---

## Validations

- ✅ challenge_id exists and is not expired
- ✅ challenge belongs to user_id
- ✅ attestation signature is valid
- ✅ client_data_json.origin matches expected origin
- ✅ client_data_json.type == "webauthn.create"
- ✅ credential_id is unique (not already registered)

---

## Acceptance Criteria

### AC1: Happy Path - Complete Registration
**GIVEN** a Client initiated WebAuthn registration  
**AND** Client completed authenticator ceremony  
**WHEN** CompleteWebAuthnRegistrationCommand is executed with valid attestation  
**THEN** Credential is verified and stored  
**AND** Credential is marked as active  
**AND** Challenge is deleted from temporary store  
**AND** WebAuthnCredentialRegistered event emitted  
**AND** HTTP response 201 Created with credential_id

### AC2: Challenge Validation - Expired Challenge Rejected
**GIVEN** a registration challenge that expired 10 minutes ago  
**WHEN** CompleteWebAuthnRegistrationCommand is executed  
**THEN** command fails with `DomainError::ChallengeExpired`

### AC3: Attestation Verification - Invalid Signature Rejected
**GIVEN** a Client submits invalid attestation  
**WHEN** CompleteWebAuthnRegistrationCommand is executed  
**THEN** command fails with `DomainError::InvalidAttestation`  
**AND** credential is NOT stored

### AC4: Origin Validation - Cross-Origin Rejected
**GIVEN** client_data_json.origin = "https://evil.com"  
**WHEN** CompleteWebAuthnRegistrationCommand is executed  
**THEN** command fails with `DomainError::InvalidOrigin`

### AC5: Duplicate Credential - Rejected
**GIVEN** a credential_id is already registered  
**WHEN** CompleteWebAuthnRegistrationCommand is executed  
**THEN** command fails with `DomainError::CredentialAlreadyRegistered`

### AC6: Credential Storage - Secure Persistence
**GIVEN** a valid attestation  
**WHEN** CompleteWebAuthnRegistrationCommand is executed  
**THEN** credential is stored with:
- credential_id
- public_key (extracted from attestation)
- sign_count: 0
- aaguid (authenticator type)
- created_at
- last_used_at: null

---

## Handler Implementation

```rust
impl CommandHandler<CompleteWebAuthnRegistrationCommand> for CompleteWebAuthnRegistrationCommandHandler {
    async fn handle(&self, cmd: CompleteWebAuthnRegistrationCommand) -> Result<CredentialId, DomainError> {
        // 1. Get and verify challenge
        let challenge = self.challenge_store
            .get(&cmd.challenge_id)
            .await?
            .ok_or(DomainError::ChallengeExpired)?;
        
        if challenge.user_id != cmd.user_id {
            return Err(DomainError::Unauthorized);
        }
        
        // 2. Parse client_data_json
        let client_data: ClientData = serde_json::from_slice(&cmd.credential_response.response.client_data_json)?;
        
        // 3. Verify client_data
        if client_data.type_ != "webauthn.create" {
            return Err(DomainError::InvalidClientDataType);
        }
        
        if client_data.origin != format!("https://{}", self.config.domain) {
            return Err(DomainError::InvalidOrigin);
        }
        
        if client_data.challenge != challenge.value {
            return Err(DomainError::ChallengeMismatch);
        }
        
        // 4. Parse and verify attestation_object
        let attestation = self.webauthn_service
            .verify_attestation(
                &cmd.credential_response.response.attestation_object,
                &client_data,
            )
            .await?;
        
        // 5. Check for duplicate credential
        let existing = self.webauthn_credential_repository
            .find_by_credential_id(&cmd.credential_response.id)
            .await?;
        
        if existing.is_some() {
            return Err(DomainError::CredentialAlreadyRegistered);
        }
        
        // 6. Create credential entity
        let credential = WebAuthnCredential::new(
            CredentialId::new(),
            cmd.user_id.clone(),
            cmd.credential_response.raw_id,
            attestation.public_key,
            attestation.aaguid,
            cmd.credential_name,
        )?;
        
        // 7. Persist credential
        self.webauthn_credential_repository.save(&credential).await?;
        
        // 8. Delete challenge
        self.challenge_store.delete(&cmd.challenge_id).await?;
        
        // 9. Emit event
        self.event_publisher.publish(DomainEvent::WebAuthnCredentialRegistered {
            credential_id: credential.id.clone(),
            user_id: cmd.user_id,
            credential_name: cmd.credential_name,
            aaguid: attestation.aaguid,
            timestamp: Utc::now(),
        }).await?;
        
        Ok(credential.id)
    }
}
```

---

## API Endpoint

```http
POST /api/client/auth/webauthn/register/complete
Authorization: Bearer {client_jwt_token}
Content-Type: application/json

Request Body:
{
  "challenge_id": "550e8400-e29b-41d4-a716-446655440000",
  "credential_name": "YubiKey 5C",
  "credential": {
    "id": "abc123...",
    "rawId": "abc123..." (base64),
    "response": {
      "attestationObject": "o2NmbXRkbm9uZWdhdHRTdG10oGhhdXRoRGF0YVjE..." (base64),
      "clientDataJSON": "eyJ0eXBlIjoid2ViYXV0aG4uY3JlYXRlIiwi..." (base64)
    },
    "type": "public-key"
  }
}

Response 201 Created:
{
  "credential_id": "cred_789",
  "credential_name": "YubiKey 5C",
  "registered_at": "2026-02-14T10:30:00Z"
}
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related:** [InitiateWebAuthnRegistrationCommand](initiate_webauthn_registration.md)
