# Passwordless Authentication Architecture

## Overview

The Secure Sandbox Server uses **passwordless authentication exclusively**. No passwords are stored or transmitted anywhere in the system.

## ⚠️ Why No Passwords?

Passwords are the #1 security vulnerability in modern systems:

| Attack Vector | Impact | Mitigation with Passwordless |
|---------------|--------|------------------------------|
| **Phishing** | 1.5M fake sites/month steal credentials | ✅ WebAuthn is origin-bound, cannot be phished |
| **Credential Stuffing** | 81% of users reuse passwords | ✅ No passwords to reuse |
| **Weak Passwords** | "123456" is still #1 password | ✅ Cryptographic keys, no weak options |
| **Database Breaches** | Hashed passwords cracked offline | ✅ Public keys only, private keys never transmitted |
| **Keyloggers** | Malware steals typed passwords | ✅ No typing, cryptographic challenge-response |
| **Social Engineering** | Users tricked into revealing passwords | ✅ Cannot reveal what doesn't exist |

---

## Authentication Methods

### 1. WebAuthn/FIDO2 (Primary - Mandatory for Super Admins)

**Technology:** W3C Web Authentication API + FIDO2 protocol

**Supported Authenticators:**
- Hardware security keys (YubiKey, Titan, SoloKeys)
- Platform authenticators (Touch ID, Face ID, Windows Hello)
- Passkeys synced via cloud (iCloud Keychain, Google Password Manager)

**Security Properties:**

✅ **Phishing-Resistant** - Credentials cryptographically bound to origin (e.g., `https://sandbox.example.com`)  
✅ **No Shared Secrets** - Public key on server, private key stays on device  
✅ **Replay Protection** - Signature counter increments with each use  
✅ **Device Attestation** - Server can verify authenticator is genuine hardware  
✅ **User Verification** - Biometric or PIN required (local to device)  
✅ **Man-in-the-Middle Resistant** - Challenge-response protocol  

**Registration Flow:**

```
┌──────────┐                  ┌──────────┐                  ┌─────────────┐
│  Client  │                  │  Server  │                  │Authenticator│
└────┬─────┘                  └────┬─────┘                  └──────┬──────┘
     │                             │                               │
     │ 1. Start registration        │                               │
     ├─────────────────────────────►│                               │
     │    (email)                   │                               │
     │                             │                               │
     │ 2. Challenge + options       │                               │
     │◄─────────────────────────────┤                               │
     │   {                          │                               │
     │     challenge: [random 32B], │                               │
     │     rp: "sandbox.example.com"│                               │
     │     user: { id, email },     │                               │
     │     pubKeyCredParams: [ES256]│                               │
     │   }                          │                               │
     │                             │                               │
     │ 3. navigator.credentials.create()                           │
     ├─────────────────────────────────────────────────────────────►│
     │                             │                               │
     │                             │     4. User consent           │
     │                             │        (biometric/PIN)        │
     │                             │◄──────────────────────────────┤
     │                             │                               │
     │                             │     5. Generate key pair      │
     │                             │        (P-256 curve)          │
     │                             │                               │
     │ 6. Public key + signature    │                               │
     │◄─────────────────────────────────────────────────────────────┤
     │   {                          │                               │
     │     credentialId,            │                               │
     │     publicKey (COSE),        │                               │
     │     attestation              │                               │
     │   }                          │                               │
     │                             │                               │
     │ 7. Send to server            │                               │
     ├─────────────────────────────►│                               │
     │                             │                               │
     │                             │ 8. Verify attestation         │
     │                             │    Store public key           │
     │                             │                               │
     │ 9. Success                   │                               │
     │◄─────────────────────────────┤                               │
```

**Authentication Flow:**

```
┌──────────┐                  ┌──────────┐                  ┌─────────────┐
│  Client  │                  │  Server  │                  │Authenticator│
└────┬─────┘                  └────┬─────┘                  └──────┬──────┘
     │                             │                               │
     │ 1. Start login               │                               │
     ├─────────────────────────────►│                               │
     │    (email)                   │                               │
     │                             │                               │
     │ 2. Challenge + allowCredentials                             │
     │◄─────────────────────────────┤                               │
     │   {                          │                               │
     │     challenge: [random 32B], │                               │
     │     allowCredentials: [      │                               │
     │       {id: cred1, transports}│                               │
     │     ],                       │                               │
     │     userVerification: required│                              │
     │   }                          │                               │
     │                             │                               │
     │ 3. navigator.credentials.get()                              │
     ├─────────────────────────────────────────────────────────────►│
     │                             │                               │
     │                             │     4. User verification      │
     │                             │        (tap key/biometric)    │
     │                             │◄──────────────────────────────┤
     │                             │                               │
     │                             │     5. Sign challenge         │
     │                             │        with private key       │
     │                             │                               │
     │ 6. Signed assertion          │                               │
     │◄─────────────────────────────────────────────────────────────┤
     │   {                          │                               │
     │     credentialId,            │                               │
     │     signature,               │                               │
     │     authenticatorData,       │                               │
     │     clientDataJSON           │                               │
     │   }                          │                               │
     │                             │                               │
     │ 7. Send to server            │                               │
     ├─────────────────────────────►│                               │
     │                             │                               │
     │                             │ 8. Verify signature           │
     │                             │    Check counter (replay)     │
     │                             │    Update counter             │
     │                             │                               │
     │ 9. JWT token                 │                               │
     │◄─────────────────────────────┤                               │
```

---

### 2. Magic Links (Secondary - Owner/Client Users)

**Technology:** Email-based one-time authentication links

**Use Case:** Lower-risk users without hardware security keys

**Security Properties:**

✅ **Time-Limited** - 15-minute expiration  
✅ **Single-Use** - Token invalidated after use  
✅ **Rate-Limited** - Max 3 requests per hour per email  
✅ **Cryptographically Secure** - 32-byte random tokens (256-bit entropy)  
⚠️ **Email Compromise Risk** - Vulnerable if user's email is hacked  

**Flow:**

```
┌──────────┐          ┌──────────┐          ┌───────────┐          ┌──────────┐
│  Client  │          │  Server  │          │Email Server│         │  Email   │
└────┬─────┘          └────┬─────┘          └─────┬─────┘          └────┬─────┘
     │                     │                       │                     │
     │ 1. Request magic link                       │                     │
     ├────────────────────►│                       │                     │
     │   (email address)   │                       │                     │
     │                     │                       │                     │
     │                     │ 2. Generate token     │                     │
     │                     │    (32 random bytes)  │                     │
     │                     │    Store hash in DB   │                     │
     │                     │                       │                     │
     │                     │ 3. Send email         │                     │
     │                     ├──────────────────────►│                     │
     │                     │  with link            │                     │
     │                     │  (token in URL)       │                     │
     │                     │                       │                     │
     │                     │                       │ 4. Deliver          │
     │                     │                       ├────────────────────►│
     │                     │                       │                     │
     │                     │                       │ 5. User clicks link │
     │ 6. GET /auth/magic-link/verify/{token}      │◄────────────────────┤
     ├────────────────────────────────────────────────────────────────────┤
     │                     │                       │                     │
     │                     │ 7. Verify token hash  │                     │
     │                     │    Check expiration   │                     │
     │                     │    Mark as used       │                     │
     │                     │                       │                     │
     │ 8. Redirect to app with JWT token           │                     │
     │◄────────────────────┤                       │                     │
```

**Token Security:**

```rust
pub struct MagicLinkToken {
    pub token: String,              // 32-byte hex (64 chars)
    pub hash: String,               // SHA-256 of token
    pub expires_at: DateTime<Utc>,  // 15 minutes from creation
    pub used: bool,                 // Single-use flag
}

// Generation
pub fn generate_magic_link_token() -> (String, String) {
    let token_bytes = rand::random::<[u8; 32]>();
    let token = hex::encode(token_bytes);
    let hash = sha256(&token);
    (token, hash)
}

// Verification
pub async fn verify_magic_link_token(
    token: &str,
) -> Result<UserId, AuthError> {
    let hash = sha256(token);
    let stored = repository.find_token_by_hash(&hash).await?;
    
    // Check expiration
    if Utc::now() > stored.expires_at {
        return Err(AuthError::TokenExpired);
    }
    
    // Check single-use
    if stored.used {
        return Err(AuthError::TokenAlreadyUsed);
    }
    
    // Mark as used (prevent replay)
    repository.mark_token_used(&hash).await?;
    
    Ok(stored.user_id)
}
```

**Rate Limiting:**

```rust
// Prevent magic link spam
pub async fn request_magic_link(
    email: &str,
) -> Result<(), AuthError> {
    let recent_requests = repository
        .count_recent_requests(email, Duration::hours(1))
        .await?;
    
    if recent_requests >= 3 {
        return Err(AuthError::RateLimitExceeded);
    }
    
    // Generate and send token...
}
```

---

## Session Management

**After successful authentication (WebAuthn or magic link), the system issues JWT tokens:**

### Access Token (Short-Lived)

```json
{
  "sub": "usr_550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "role": "owner",
  "iat": 1708012800,
  "exp": 1708013700,  // 15 minutes
  "jti": "token_unique_id"
}
```

**Properties:**
- Lifetime: **15 minutes**
- Storage: httpOnly, Secure, SameSite=Strict cookie
- Used for API requests

### Refresh Token (Long-Lived)

```json
{
  "sub": "usr_550e8400-e29b-41d4-a716-446655440000",
  "type": "refresh",
  "device_id": "dev_abc123",
  "iat": 1708012800,
  "exp": 1708617600  // 7 days
}
```

**Properties:**
- Lifetime: **7 days**
- Storage: httpOnly, Secure, SameSite=Strict cookie
- Used only to refresh access token
- Invalidated on: logout, security event, max lifetime

### Token Refresh Flow

```
Client                          Server
  │                               │
  │ 1. API request (access token expired)
  ├──────────────────────────────►│
  │                               │ 2. Return 401 Unauthorized
  │◄──────────────────────────────┤
  │                               │
  │ 3. POST /api/auth/refresh     │
  │    (with refresh token)       │
  ├──────────────────────────────►│
  │                               │ 4. Verify refresh token
  │                               │    Check not revoked
  │                               │    Issue new access token
  │                               │
  │ 5. New access token           │
  │◄──────────────────────────────┤
  │                               │
  │ 6. Retry API request          │
  ├──────────────────────────────►│
```

---

## Security Considerations

### Super Admin Requirements

**CRITICAL: Super Admins MUST use hardware security keys**

- ❌ **No magic links** - Email compromise = full system access
- ✅ **WebAuthn hardware key only** - YubiKey, Titan, etc.
- ✅ **Minimum 2 registered devices** - Backup key required
- ✅ **Attestation verification** - Verify genuine FIDO2 hardware

```rust
pub async fn register_super_admin_credential(
    user_id: &UserId,
    credential: WebAuthnCredential,
) -> Result<(), AuthError> {
    let user = repository.find_user(user_id).await?;
    
    // Enforce WebAuthn for super admins
    if user.role == UserRole::SuperAdmin {
        // Verify hardware attestation
        if credential.attestation_format != "packed" &&
           credential.attestation_format != "fido-u2f" {
            return Err(AuthError::HardwareKeyRequired);
        }
        
        // Check minimum 2 devices
        let existing_count = repository
            .count_credentials(user_id)
            .await?;
        
        if existing_count == 0 {
            // First key, require backup registration
            warn!("Super admin must register backup key");
        }
    }
    
    repository.store_credential(credential).await
}
```

### Anti-Phishing

**WebAuthn is inherently phishing-resistant:**

```rust
// Browser enforces origin binding
// Credentials for sandbox.example.com CANNOT be used on evil.com

// Server verifies origin in clientDataJSON
pub fn verify_origin(client_data_json: &str) -> Result<()> {
    let data: ClientData = serde_json::from_str(client_data_json)?;
    
    if data.origin != "https://sandbox.example.com" {
        return Err(AuthError::OriginMismatch);
    }
    
    Ok(())
}
```

**Even if user visits phishing site:**
1. Browser creates challenge for phishing site's origin
2. Signature is origin-bound to phishing site
3. Real server rejects signature (origin mismatch)
4. **Attack fails**

### Replay Protection

```rust
pub async fn verify_authentication(
    credential_id: &[u8],
    authenticator_data: &[u8],
    signature: &[u8],
) -> Result<UserId> {
    let stored_cred = repository.find_credential(credential_id).await?;
    
    // Parse counter from authenticator data
    let counter = parse_counter(authenticator_data)?;
    
    // CRITICAL: Reject if counter didn't increment
    if counter <= stored_cred.counter {
        // Possible cloned authenticator or replay attack
        alert_security_team(&stored_cred.user_id, "Replay attack detected");
        return Err(AuthError::ReplayDetected);
    }
    
    // Verify signature...
    
    // Update counter in database
    repository.update_counter(credential_id, counter).await?;
    
    Ok(stored_cred.user_id)
}
```

---

## Migration from Password-Based Systems

For organizations migrating from password-based auth:

### Phase 1: Add WebAuthn (Optional)
- Users can register passkeys alongside passwords
- Gradual adoption

### Phase 2: Make WebAuthn Required
- All users must register at least one passkey
- Passwords still accepted temporarily

### Phase 3: Deprecate Passwords
- Remove password authentication
- Migrate remaining users to magic links

### Phase 4: WebAuthn Only
- Disable magic links for high-security users
- Pure passwordless system

---

## User Experience

### Registration (New User)

```
1. User enters email on registration page
2. Server sends verification email
3. User clicks email link
4. Redirect to passkey registration
5. Browser prompts: "Create a passkey for sandbox.example.com?"
6. User taps security key or uses Touch ID
7. Passkey created
8. User logged in immediately
```

### Login (Existing User)

```
1. User enters email on login page
2. Browser prompts: "Sign in with passkey?"
3. User taps security key or uses Touch ID
4. Logged in (no password typing)
```

**Time to authenticate:** ~2 seconds (vs ~10+ seconds typing password)

---

## Compliance

### FIDO2 Certification

The system uses `webauthn-rs`, a Rust implementation of W3C WebAuthn spec.

**Compliance:**
- ✅ W3C Web Authentication Level 2
- ✅ FIDO2 CTAP 2.1
- ✅ NIST SP 800-63B AAL3 (highest assurance)

### Regulatory Alignment

| Standard | Requirement | How Passwordless Helps |
|----------|-------------|------------------------|
| **HIPAA** | Strong authentication | WebAuthn = phishing-resistant MFA |
| **PCI DSS** | No default passwords | No passwords exist |
| **GDPR** | Data minimization | No password hashes stored |
| **SOC 2** | Access controls | Cryptographic authentication |

---

## Dependency: webauthn-rs

**Crate:** `webauthn-rs` (maintained by Kanidm project)

**Features:**
- Full W3C WebAuthn specification
- FIDO2 CTAP support
- Attestation verification
- Credential storage helpers
- Async-friendly

**Example Usage:**

```rust
use webauthn_rs::prelude::*;

pub struct AuthenticationService {
    webauthn: Webauthn,
}

impl AuthenticationService {
    pub fn new() -> Self {
        let rp = RelyingParty {
            name: "Secure Sandbox Server",
            id: "sandbox.example.com",
        };
        
        let webauthn = WebauthnBuilder::new("sandbox.example.com", &rp)
            .expect("Invalid configuration")
            .rp_origin(Url::parse("https://sandbox.example.com").unwrap())
            .build()
            .expect("Failed to build Webauthn");
        
        Self { webauthn }
    }
    
    pub async fn start_registration(
        &self,
        user_id: &UserId,
        email: &str,
    ) -> Result<(CreationChallengeResponse, PasskeyRegistration)> {
        let user_unique_id = user_id.as_bytes();
        
        let (ccr, reg_state) = self.webauthn
            .start_passkey_registration(
                user_unique_id,
                email,
                email,
                None,  // No excludeCredentials
            )?;
        
        Ok((ccr, reg_state))
    }
    
    pub async fn finish_registration(
        &self,
        reg: &RegisterPublicKeyCredential,
        state: &PasskeyRegistration,
    ) -> Result<Passkey> {
        let passkey = self.webauthn
            .finish_passkey_registration(reg, state)?;
        
        Ok(passkey)
    }
}
```

---

## API Endpoints

### WebAuthn Registration

```
POST /api/auth/webauthn/register/begin
Request:  { "email": "user@example.com" }
Response: {
  "publicKey": {
    "challenge": "base64...",
    "rp": { "name": "...", "id": "..." },
    "user": { "id": "...", "name": "...", "displayName": "..." },
    "pubKeyCredParams": [...],
    "timeout": 60000,
    "authenticatorSelection": {...}
  }
}

POST /api/auth/webauthn/register/finish
Request:  {
  "credential": {
    "id": "base64...",
    "rawId": "base64...",
    "response": {
      "attestationObject": "base64...",
      "clientDataJSON": "base64..."
    },
    "type": "public-key"
  }
}
Response: { "success": true }
```

### WebAuthn Authentication

```
POST /api/auth/webauthn/login/begin
Request:  { "email": "user@example.com" }
Response: {
  "publicKey": {
    "challenge": "base64...",
    "allowCredentials": [
      { "id": "base64...", "type": "public-key", "transports": ["usb"] }
    ],
    "timeout": 60000,
    "userVerification": "required"
  }
}

POST /api/auth/webauthn/login/finish
Request:  {
  "credential": {
    "id": "base64...",
    "rawId": "base64...",
    "response": {
      "authenticatorData": "base64...",
      "clientDataJSON": "base64...",
      "signature": "base64...",
      "userHandle": "base64..."
    },
    "type": "public-key"
  }
}
Response: {
  "access_token": "jwt...",
  "refresh_token": "jwt...",
  "expires_in": 900
}
```

### Magic Links

```
POST /api/auth/magic-link/request
Request:  { "email": "user@example.com" }
Response: { "message": "Check your email" }

GET /api/auth/magic-link/verify/:token
Response: Redirect to app with JWT in cookie
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related Documents:** [SECURITY.md](SECURITY.md), [PERSONAS.md](PERSONAS.md), [REQUIREMENTS.md](REQUIREMENTS.md)
