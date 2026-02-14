# Project Requirements

## ⚠️ CRITICAL SECURITY DIRECTIVE

**THIS IS A SECURITY-FIRST PROJECT. ALL DECISIONS MUST PRIORITIZE SECURITY OVER CONVENIENCE.**

### Non-Negotiable Security Principles

1. **NEVER sacrifice security for ease of use or development speed**
2. **NEVER open ports, permissions, or access by default**
3. **ALWAYS use the most restrictive configuration possible**
4. **ALWAYS require explicit opt-in for any relaxed security measures**
5. **ALWAYS assume hostile actors have access to the system**
6. **ALWAYS validate, sanitize, and audit every input and action**

**This directive applies to:**
- All code implementations
- All configuration defaults
- All documentation examples
- All deployment configurations
- All testing environments
- All development workflows

**Violations of this directive are considered critical bugs and must be fixed immediately.**

---

## High-Level Requirements

### 1. User Personas & Authorization

**The system supports three distinct user roles:**

1. **Super Admin** - System administrator with root access (2-3 accounts maximum, MFA required)
2. **User (Data Owner)** - Person who owns data, creates shares, and approves access requests
3. **Client User (Data Consumer)** - Person who requests and views another user's data

**See [PERSONAS.md](PERSONAS.md) for:**
- Complete persona definitions and capabilities
- Authorization workflows (share creation, access requests, approvals)
- Role-based access control (RBAC) implementation
- Security constraints per role
- Domain model updates (Share, AccessRequest, Permission aggregates)

### 2. Deployment Architecture

**Primary Deployment Method: Docker Compose**

The entire system MUST be deployable via Docker Compose with the following characteristics:

- **Self-contained**: All services defined in `docker-compose.yml`
- **Isolated networks**: Services communicate only through defined networks
- **Volume-based storage**: Persistent data stored in Docker volumes
- **No host dependencies**: Minimal reliance on host system configuration
- **Rootless where possible**: Containers run as non-root users
- **Read-only filesystems**: Containers use read-only root filesystems where applicable

**Services Required:**
- **Application Server** (Rust sandbox server)
- **PostgreSQL Database** (user/permission/session storage)
- **Storage Volume** (encrypted user files)
- **Reverse Proxy** (HAProxy for TLS termination - GDPR compliant)
- **Logging Aggregator** (Optional: for audit logs)

### 2. Storage Requirements

**Volume Management:**

- **Encrypted at rest**: All user data MUST be encrypted
- **Access control**: Only application container has access to storage volume
- **Backup capability**: Volumes MUST be backup-friendly
- **Quota enforcement**: Per-user storage limits enforced
- **Immutable audit logs**: Separate volume for audit trail

**Volume Structure:**
```
volumes:
  user_data:          # Encrypted user files (restrictive permissions)
  postgres_data:      # Database storage (PostgreSQL only)
  audit_logs:         # Immutable audit trail (append-only)
  ssl_certs:          # TLS certificates (read-only for app)
```

### 3. Security Requirements

#### Network Security

**MANDATORY:**
- TLS 1.3 ONLY (no fallback to TLS 1.2)
- No plaintext HTTP (redirect to HTTPS)
- Internal services NOT exposed to host network
- Firewall rules default-deny
- No container-to-container communication except explicitly allowed
- No internet access from sandbox containers

**FORBIDDEN:**
- Port forwarding to development ports (8080, 5432, etc.)
- Binding services to 0.0.0.0 by default
- Docker socket mounting
- Privileged containers (except where absolutely required for namespaces)

#### Authentication & Authorization

**MANDATORY:**
- Strong password requirements (min 16 chars, complexity)
- argon2id for password hashing (no exceptions)
- JWT tokens with short expiry (15 min max for access tokens)
- Session invalidation on logout
- Multi-factor authentication (future requirement)
- Rate limiting on ALL endpoints (especially auth)
- Account lockout after failed attempts

**FORBIDDEN:**
- Default credentials
- Hardcoded secrets
- Plaintext password storage
- Token storage in localStorage
- Session tokens in URL parameters

#### Container Security

**MANDATORY:**
- Run as non-root user (UID/GID mapped)
- Read-only root filesystem where possible
- Dropped capabilities (cap-drop: ALL, selectively add back)
- Seccomp profiles applied
- AppArmor/SELinux profiles enforced
- Resource limits (CPU, memory, PIDs)
- Network policies restricting inter-container traffic
- No new privileges flag set
- User namespace remapping enabled

**FORBIDDEN:**
- Privileged mode (unless absolutely required, documented, and audited)
- Mounting Docker socket
- Host network mode
- Host PID namespace
- --cap-add=ALL
- Disabled security options

#### Data Security

**MANDATORY:**
- Encryption at rest (AES-256-GCM minimum)
- Encryption in transit (TLS 1.3)
- Secrets managed via Docker secrets or external vault
- Audit logging for all data access
- Automatic secret rotation
- Secure key derivation (PBKDF2/scrypt/argon2)
- Data retention policies enforced
- Secure deletion (overwrite, not just unlink)

**FORBIDDEN:**
- Secrets in environment variables
- Secrets in Docker images
- Secrets in version control
- Unencrypted backups
- Logging sensitive data (passwords, tokens, PII)

### 4. Operational Requirements

#### Monitoring & Logging

**CRITICAL: TOTAL TRACEABILITY REQUIREMENT**

**EVERYTHING MUST BE LOGGED.** This is a non-negotiable requirement for security, compliance, and forensics.

**MANDATORY:**
- Structured logging (JSON format)
- Centralized log aggregation
- Audit trail for ALL operations (not just security events)
- Metrics collection (resource usage, errors)
- Alerting on security violations
- Log retention (minimum 1 year, configurable per compliance requirements)
- Tamper-proof logs (write-once storage, append-only)
- Correlation IDs across all log entries for request tracing
- Performance metrics for log ingestion and query

**LOGGED EVENTS (COMPREHENSIVE):**

*Authentication & Authorization:*
- All authentication attempts (success/failure)
- All authorization checks (granted/denied)
- Token issuance, refresh, and revocation
- Role assignments and changes
- Account lockouts and unlocks

*Data Access:*
- All file access operations (read/write/execute)
- File metadata queries
- Permission grants and revocations
- Encryption/decryption operations

*System Operations:*
- All command executions (with input parameters)
- All query executions (with filters)
- Configuration changes
- Container/sandbox lifecycle events
- Resource limit changes
- Network connections and disconnections

*Application Events:*
- Session creation, activation, termination
- WebRTC connection establishment/teardown
- Video encoding start/stop
- Input forwarding events
- State transitions

*Security Events:*
- Security policy violations
- Failed sandbox isolation attempts
- Landlock policy denials
- seccomp filter violations
- cgroup limit breaches
- Rate limit violations
- Suspicious activity patterns

*Infrastructure:*
- Database queries (sanitized, no sensitive data)
- External service calls
- Background job executions
- Health check failures
- Deployment events

#### Backup & Recovery

**MANDATORY:**
- Automated daily backups
- Encrypted backup storage
- Off-site backup replication
- Regular restore testing
- Point-in-time recovery capability
- Immutable backup retention
- Backup integrity verification

#### Updates & Maintenance

**MANDATORY:**
- Automated security updates for base images
- Dependency vulnerability scanning
- Regular security audits
- Patch management process
- Zero-downtime updates (where possible)
- Rollback capability
- Change management documentation

### 5. Development Requirements

#### Development Environment

**MANDATORY:**
- Development environment MUST match production configuration
- Development uses same Docker Compose setup
- Development secrets DIFFERENT from production
- Development TLS certificates (self-signed acceptable)
- Development rate limiting enabled
- Development audit logging enabled

**FORBIDDEN:**
- Disabled security features in development
- Debug endpoints in production builds
- Development secrets committed to repository
- Production data in development environment

#### Code Quality

**MANDATORY:**
- All code reviewed before merge
- Automated testing (unit + integration)
- Security-focused linting (clippy with security rules)
- Dependency auditing (cargo audit)
- Static analysis
- Fuzzing for input validation
- Penetration testing before major releases

### 6. Compliance Requirements

#### Data Protection

**MANDATORY:**
- GDPR compliance (EU data protection)
- HIPAA compliance considerations (US healthcare)
- Data minimization principles
- User consent management
- Right to erasure support
- Data portability support
- Privacy by design

#### Audit & Compliance

**MANDATORY:**
- Immutable audit trails
- Compliance reporting capabilities
- Access control matrices
- Security policy documentation
- Incident response procedures
- Disaster recovery plan
- Business continuity plan

### 7. Performance Requirements

**Constraints (Security Takes Priority):**
- Session startup: < 2 seconds (acceptable tradeoff for security checks)
- Video latency: < 200ms (local network, security encryption overhead acceptable)
- Concurrent sessions: 20-50 per server (resource limits for isolation)
- API response time: < 500ms (includes auth validation)

**NEVER sacrifice security for performance:**
- Do NOT skip validation for speed
- Do NOT cache sensitive data unencrypted
- Do NOT bypass authentication for performance
- Do NOT reduce encryption strength for speed
- Do NOT disable audit logging for throughput

### 8. Scalability Requirements

**Horizontal Scaling:**
- Stateless application design
- Shared PostgreSQL database
- Shared storage (NFS/S3)
- Load balancer with session affinity
- No hardcoded server addresses

**Resource Limits:**
- Per-session CPU limit: 50% of 1 core (default)
- Per-session memory limit: 512MB (default)
- Per-session PID limit: 100 processes
- Per-session I/O limit: 10MB/s
- Per-user storage quota: Configurable

### 9. Documentation Requirements

**MANDATORY:**
- Security implications documented for every feature
- Threat model maintained and updated
- Architecture diagrams include security boundaries
- API documentation includes auth requirements
- Deployment guide includes security checklist
- Runbooks for security incidents
- Security best practices guide

### 10. Testing Requirements

**MANDATORY Security Tests:**
- Authentication bypass attempts
- Authorization escalation attempts
- Input validation fuzzing
- SQL injection tests
- XSS/CSRF tests
- Container escape attempts
- Network isolation verification
- Encryption verification
- Audit log integrity tests
- Rate limiting effectiveness

**Test Environments:**
- All tests run in isolated containers
- Tests MUST NOT use production secrets
- Tests MUST NOT modify production data
- Tests include security regression suite

---

## Acceptance Criteria

A feature is ONLY considered complete when:

1. ✅ **Security review passed** - No vulnerabilities introduced
2. ✅ **Tests pass** - Including security tests
3. ✅ **Documentation updated** - Including security implications
4. ✅ **Audit logging added** - For security-relevant actions
5. ✅ **Default-secure configuration** - No insecure defaults
6. ✅ **Code reviewed** - By at least one other developer
7. ✅ **Dependencies audited** - No known vulnerabilities

---

## Red Lines (Absolutely Forbidden)

The following are **NEVER** acceptable under any circumstances:

❌ Default passwords or credentials  
❌ Disabled authentication in any environment  
❌ Disabled TLS/encryption  
❌ Secrets in source code or images  
❌ Privileged containers without explicit justification  
❌ Open ports without firewall rules  
❌ Disabled audit logging  
❌ SQL injection vulnerabilities  
❌ Command injection vulnerabilities  
❌ Path traversal vulnerabilities  
❌ Unvalidated user input  
❌ Unauthenticated admin endpoints  
❌ Excessive permissions granted by default  
❌ Disabled security features for convenience  

---

## Change Management

Any change that affects security MUST:

1. Be documented in a security change request
2. Include threat modeling analysis
3. Be reviewed by security-focused reviewer
4. Include updated security tests
5. Update threat model documentation
6. Include rollback plan
7. Be announced to all stakeholders

---

## Questions & Clarifications

When in doubt about security vs. convenience:

**ALWAYS choose security.**

If a feature cannot be implemented securely, it should NOT be implemented until a secure approach is found.

"Move fast and break things" does NOT apply to security-critical systems.

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-13  
**Review Frequency:** Quarterly or after any security incident  
**Owner:** Security Team / Project Lead
