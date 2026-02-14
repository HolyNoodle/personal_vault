# Security Documentation

## Overview

This document details the security architecture, threat model, and security best practices for the Secure Sandbox Server.

## Security Principles

### 1. Zero Trust Architecture

- **Never trust client input**: All data from browsers is validated and sanitized
- **Verify every request**: JWT tokens validated on every API call
- **Least privilege**: Sandboxes granted minimal required permissions
- **Assume breach**: Multiple layers of defense prevent lateral movement

### 2. Defense in Depth

Each security layer operates independently. Compromise of one layer does not compromise the system:

```
Browser → TLS → Auth → RBAC → Sandbox → Landlock → seccomp → cgroups
```

### 3. Fail Secure

- Default deny for filesystem access
- Sandboxes killed on any security violation
- Sessions terminated on invalid input
- Logs preserved even on crash

## Threat Model

### Assets

1. **User Data**: Documents, images, videos stored on server
2. **User Sessions**: Active WebRTC connections and sandbox environments
3. **Credentials**: Passwords, JWT tokens, encryption keys
4. **Server Infrastructure**: Host system, kernel, other users' sandboxes
5. **Audit Logs**: Immutable records of access and operations

### Threat Actors

1. **Unauthorized Users**: Attempting to access system without credentials
2. **Malicious Users**: Valid users attempting to escalate privileges or access others' data
3. **Compromised Clients**: Browsers infected with malware
4. **Network Attackers**: Man-in-the-middle, eavesdropping
5. **Insider Threats**: Server administrators with host access

### Attack Vectors

#### 1. Network Attacks

**Threats:**
- Man-in-the-middle interception
- Eavesdropping on video stream
- Session hijacking

**Mitigations:**
- TLS 1.3 for all HTTPS/WSS connections
- DTLS-SRTP for WebRTC media encryption
- Certificate pinning (optional)
- HSTS headers
- JWT tokens with short expiry (15 min)
- Secure cookie flags (HttpOnly, Secure, SameSite)

#### 2. Authentication/Authorization Bypass

**Threats:**
- Credential stuffing
- Brute force attacks
- JWT token forgery
- Privilege escalation

**Mitigations:**
- argon2id password hashing (memory-hard, GPU-resistant)
- Rate limiting on login endpoints
- Account lockout after N failed attempts
- HMAC-SHA256 JWT signing with strong secrets (256-bit)
- Token rotation and revocation
- RBAC enforced server-side before every file access
- Audit logging of all authorization decisions

#### 3. Data Exfiltration

**Threats:**
- Screen scraping via screenshots
- Copy/paste to clipboard
- Browser download functionality
- Network exfiltration from sandbox

**Mitigations:**
- Client-side clipboard API disabled
- Right-click context menu disabled
- Keyboard shortcuts (Ctrl+S, Ctrl+P) intercepted
- Watermarking (user ID + timestamp on video)
- Network namespace isolation (no internet from sandbox)
- Landlock prevents reading files outside permissions
- Audit logging of all file access

**Known Limitations:**
- Cannot prevent physical screen recording (cameras, screen capture devices)
- Cannot prevent browser extensions from capturing DOM
- Cannot prevent sophisticated screen scraping malware

#### 4. Container Escape

**Threats:**
- Kernel vulnerability exploitation
- Namespace breakout
- Privilege escalation to host

**Mitigations:**
- User namespaces (rootless operation, no CAP_SYS_ADMIN on host)
- seccomp-bpf blocks dangerous syscalls:
  - `ptrace` (no process debugging)
  - `kexec_load`, `kexec_file_load` (no kernel hijacking)
  - `module_init`, `finit_module` (no kernel module loading)
  - `reboot`, `swapon`, `swapoff` (no system control)
  - `mount` outside mount namespace
- Landlock prevents filesystem access outside policy
- Regular kernel updates and CVE monitoring
- AppArmor/SELinux profiles (optional, additional layer)

**Kernel Dependencies:**
- Requires Linux 5.13+ for Landlock
- User namespaces enabled (check `/proc/sys/kernel/unprivileged_userns_clone`)

#### 5. Resource Exhaustion (DoS)

**Threats:**
- CPU/memory consumption
- Fork bombs
- Disk space exhaustion
- Network bandwidth saturation

**Mitigations:**
- cgroups v2 limits per sandbox:
  - CPU: 50% of one core (configurable)
  - Memory: 512MB hard limit (OOM killer terminates sandbox)
  - PIDs: 100 processes max
  - I/O: 10MB/s read/write limits
- Session timeout after 30 minutes of inactivity
- Rate limiting on API endpoints
- Disk quotas per user
- Connection limits per IP address

#### 6. Input Injection Attacks

**Threats:**
- Command injection via input events
- XSS through rendered content
- Path traversal in file requests

**Mitigations:**
- Input validation:
  - Mouse coordinates clamped to screen bounds
  - Keyboard events allowlist (no function keys that trigger OS commands)
  - Rate limiting (max 100 events/second)
- File path sanitization:
  - Reject `..`, absolute paths, symlinks
  - Canonicalize paths before access
  - Landlock enforces path restrictions at kernel level
- Content Security Policy headers
- X-Frame-Options to prevent clickjacking

## Isolation Architecture

### Namespace Configuration

```rust
// Pseudocode for namespace setup
use nix::sched::{unshare, CloneFlags};

let flags = CloneFlags::NEWUSER   // User namespace (rootless)
          | CloneFlags::NEWNS     // Mount namespace (custom FS)
          | CloneFlags::NEWPID    // PID namespace (isolated proc tree)
          | CloneFlags::NEWIPC    // IPC namespace (no shared memory)
          | CloneFlags::NEWUTS;   // UTS namespace (independent hostname)
          // | CloneFlags::NEWNET  // Optional: network isolation

unshare(flags)?;
```

**User Namespace Mapping:**
```
Host UID 1000 → Container UID 0 (appears as root inside, unprivileged outside)
```

### Landlock Policies

```rust
// Pseudocode for Landlock filesystem rules
use landlock::*;

let ruleset = Ruleset::default()
    .handle_access(AccessFs::Execute)?       // Allow executing binaries
    .handle_access(AccessFs::ReadFile)?      // Allow reading files
    .handle_access(AccessFs::ReadDir)?;      // Allow listing directories

// Allow read-only access to system libraries
ruleset.add_rule(PathBeneath::new("/usr", AccessFs::Execute | AccessFs::ReadFile))?;
ruleset.add_rule(PathBeneath::new("/lib", AccessFs::ReadFile))?;

// Allow read access to user files
ruleset.add_rule(PathBeneath::new("/data/users/123", AccessFs::ReadFile | AccessFs::ReadDir))?;

// Allow write access to specific directories (if permission granted)
if user_has_write_permission {
    ruleset.add_rule(PathBeneath::new("/data/users/123/workspace", AccessFs::WriteFile))?;
}

ruleset.restrict_self()?;  // Apply policy
```

**Key Properties:**
- Default deny: Any path not explicitly allowed is blocked
- Kernel-enforced: Cannot be bypassed from userspace
- Stackable: Can layer multiple policies
- Unprivileged: No CAP_SYS_ADMIN required

### seccomp Filters

```rust
// Pseudocode for seccomp syscall filtering
use syscallz::{Syscall, Context, Action};

let mut ctx = Context::init_with_action(Action::Allow)?;

// Deny dangerous syscalls
ctx.set_action_for_syscall(Action::Errno(1), Syscall::ptrace)?;
ctx.set_action_for_syscall(Action::Errno(1), Syscall::kexec_load)?;
ctx.set_action_for_syscall(Action::Errno(1), Syscall::kexec_file_load)?;
ctx.set_action_for_syscall(Action::Errno(1), Syscall::init_module)?;
ctx.set_action_for_syscall(Action::Errno(1), Syscall::finit_module)?;
ctx.set_action_for_syscall(Action::Errno(1), Syscall::delete_module)?;
ctx.set_action_for_syscall(Action::Errno(1), Syscall::reboot)?;
ctx.set_action_for_syscall(Action::Errno(1), Syscall::swapon)?;
ctx.set_action_for_syscall(Action::Errno(1), Syscall::swapoff)?;

ctx.load()?;
```

### cgroups v2 Configuration

```
/sys/fs/cgroup/sandbox_user_123/
├── cpu.max               # 50000 100000 (50% CPU)
├── memory.max            # 536870912 (512MB)
├── memory.high           # 503316480 (480MB soft limit)
├── io.max                # 8:0 rbps=10485760 wbps=10485760 (10MB/s)
└── pids.max              # 100
```

## Authentication & Authorization

### Password Security

**Hashing Algorithm:** argon2id

**Parameters:**
- Memory: 64 MB (65536 KiB)
- Iterations: 3
- Parallelism: 4 threads
- Salt: 16 bytes (random per password)
- Output: 32 bytes

**Rationale:**
- Memory-hard: Resistant to GPU/ASIC attacks
- Time-hard: Resistant to brute force
- Side-channel resistant: Constant-time operations
- OWASP recommended for 2025+

**Storage Format:**
```
$argon2id$v=19$m=65536,t=3,p=4$<base64_salt>$<base64_hash>
```

### JWT Tokens

**Access Token:**
- Expiry: 15 minutes
- Algorithm: HS256 (HMAC-SHA256)
- Claims: `sub` (user ID), `exp` (expiry), `iat` (issued at), `roles`
- Storage: Memory only (not in localStorage to prevent XSS)

**Refresh Token:**
- Expiry: 7 days
- Stored in HttpOnly, Secure, SameSite=Strict cookie
- Rotation on use (new refresh token issued)
- Revocation list in PostgreSQL

**Token Generation:**
```rust
use jsonwebtoken::{encode, Header, EncodingKey};

let claims = Claims {
    sub: user_id,
    exp: now + 15 * 60,  // 15 minutes
    iat: now,
    roles: vec!["user"],
};

let token = encode(
    &Header::default(),
    &claims,
    &EncodingKey::from_secret(secret.as_ref()),
)?;
```

### Role-Based Access Control (RBAC)

**Roles:**
- `admin`: Full access, can manage users and permissions
- `user`: Normal user, access to own files based on permissions
- `viewer`: Read-only access to specific files

**Permissions (per file/directory):**
- `read`: Can view file in sandbox
- `write`: Can modify file in sandbox
- `execute`: Can run executable files
- `share`: Can grant access to other users (future)

**Enforcement:**
- Database query filters by user_id and permission
- Landlock rules applied based on permission level
- Server validates permission before mounting files

## Encryption

### Data at Rest

**File Encryption:**
- Algorithm: AES-256-GCM
- Key derivation: PBKDF2-HMAC-SHA256 or age encryption
- Per-user keys or per-file keys (configurable)
- Keys stored in secure key management system (future: HashiCorp Vault)

**Database Encryption:**
- PostgreSQL transparent data encryption (TDE) or
- Full disk encryption (LUKS) on storage volumes

### Data in Transit

**HTTPS/WSS:**
- TLS 1.3 only
- Strong cipher suites: TLS_AES_256_GCM_SHA384, TLS_CHACHA20_POLY1305_SHA256
- Certificate from trusted CA (Let's Encrypt)
- HSTS with max-age=31536000; includeSubDomains

**WebRTC (DTLS-SRTP):**
- DTLS 1.2 for signaling
- SRTP for media encryption
- Perfect Forward Secrecy (ephemeral key exchange)

## Audit Logging

### Log Events

**Security Events:**
- Authentication attempts (success/failure)
- Authorization denials
- Session creation/destruction
- File access (read/write)
- Permission changes
- Security policy violations
- Sandbox escape attempts

**Log Format:**
```json
{
  "timestamp": "2026-02-13T10:30:45Z",
  "event_type": "file_access",
  "user_id": 123,
  "session_id": "abc123",
  "resource": "/data/users/123/document.pdf",
  "action": "read",
  "result": "allowed",
  "ip_address": "192.168.1.100",
  "user_agent": "Mozilla/5.0..."
}
```

**Storage:**
- Append-only database table or log file
- Write-once storage (immutable)
- Regular backups
- Retention: 1 year minimum (compliance dependent)

**Alerting:**
- Failed login threshold: Alert after 5 failures in 5 minutes
- Sandbox escape attempt: Immediate alert + kill sandbox
- Unauthorized file access: Alert + log
- Resource limit exceeded: Log warning

## Security Checklist

### Deployment

- [ ] TLS certificate from trusted CA installed
- [ ] Strong JWT secret (256-bit random) configured
- [ ] Database credentials use strong passwords
- [ ] Host kernel is 5.13+ with Landlock support
- [ ] User namespaces enabled
- [ ] Host firewall allows only necessary ports (443, 3478 STUN)
- [ ] Regular security updates enabled
- [ ] Audit logging to centralized system
- [ ] Backup encryption keys securely
- [ ] Rate limiting configured on HAProxy (100 req/10s per IP)
- [ ] HSTS headers enabled
- [ ] CSP headers configured
- [ ] Fail2ban or similar IDS enabled

### Runtime

- [ ] Monitor sandbox resource usage
- [ ] Review audit logs daily
- [ ] Check for failed authentication patterns
- [ ] Verify cgroups limits are enforced
- [ ] Test Landlock policies monthly
- [ ] Rotate JWT secrets quarterly
- [ ] Update dependencies for CVEs
- [ ] Test disaster recovery procedures

### Development

- [ ] Code review for all changes
- [ ] Dependency scanning (cargo audit)
- [ ] Static analysis (clippy, rustfmt)
- [ ] No secrets in source code
- [ ] Input validation on all endpoints
- [ ] Error messages don't leak sensitive info
- [ ] Security testing before releases

## Incident Response

### Suspected Breach

1. **Isolate**: Disconnect affected server from network
2. **Preserve**: Take disk snapshot for forensics
3. **Analyze**: Review audit logs, check for anomalies
4. **Contain**: Kill all active sandboxes, revoke all JWT tokens
5. **Remediate**: Patch vulnerability, restore from clean backup
6. **Document**: Post-mortem report with timeline
7. **Notify**: Inform affected users (breach notification laws)

### Sandbox Escape

1. **Kill**: Immediately terminate sandbox and all processes
2. **Alert**: Page on-call engineer
3. **Log**: Capture kernel logs, audit trail, memory dump
4. **Analyze**: Determine escape vector (kernel CVE, config error)
5. **Patch**: Apply kernel update or fix configuration
6. **Review**: Audit all sandbox configurations
7. **Report**: File bug report with Landlock/kernel teams if applicable

## Compliance Considerations

### GDPR (EU)

- Right to access: Provide user's stored files and audit logs
- Right to erasure: Delete user data and anonymize logs
- Data minimization: Only collect necessary data
- Encryption: Data at rest and in transit
- Breach notification: Within 72 hours

### HIPAA (US Healthcare)

- Access controls: RBAC with audit logging
- Encryption: AES-256 for data at rest, TLS 1.3 for transit
- Audit logs: Immutable, 7-year retention
- Session timeout: 15 minutes inactivity
- Unique user IDs: No shared accounts

### SOC 2

- Availability: 99.9% uptime SLA
- Confidentiality: Encryption + access controls
- Processing integrity: Audit logs, change management
- Privacy: Consent, data minimization

## Security Contact

For security issues, contact: security@example.com

**PGP Key:** [To be added]

**Coordinated Disclosure:** 90-day disclosure period after patch availability
