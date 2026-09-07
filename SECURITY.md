# Aevum Platform — Security

Security baseline for the Aevum Platform authentication and backend
subsystem.

This document describes implemented security controls and their intended
production posture. A checked item means the control is implemented in the
current codebase; it does not constitute a claim that the entire system is
free of vulnerabilities.

---

## 1. Authentication

- [x] Argon2id password hashing
- [x] Explicit Argon2id parameters
- [x] Password length limits enforced
- [x] Password hashes are never returned by API responses
- [x] Passwords are not intentionally written to logs
- [x] Session tokens use 32 cryptographically secure random bytes
- [x] Session tokens are opaque to clients
- [x] Raw session tokens are not persisted
- [x] Session token hashes are persisted instead of raw tokens
- [x] Session expiration enforced
- [x] Session rotation supported
- [x] Session revocation supported
- [x] Logout is idempotent
- [x] All-session revocation supported
- [x] Suspended/deleted users cannot authenticate

### Authentication behavior

Unknown users, invalid passwords, expired sessions and revoked sessions
must not expose sensitive internal information through API responses.

---

## 2. Session Cookies

Production session cookies use the `__Host-` prefix.

- [x] `__Host-` session cookie
- [x] `Secure`
- [x] `HttpOnly`
- [x] `SameSite=Lax`
- [x] `Path=/`
- [x] No `Domain` attribute
- [x] Explicit session lifetime
- [x] Session cookie contains only the opaque session token

The production deployment MUST use HTTPS.

---

## 3. CSRF Protection

State-changing browser requests are protected using a CSRF mechanism.

- [x] Double-submit CSRF cookie pattern
- [x] CSRF token comparison uses constant-time comparison
- [x] CSRF token is not stored in the session cookie
- [x] CSRF token is not `HttpOnly`
- [x] `X-CSRF-Token` request header supported
- [x] Authentication bootstrap endpoints have explicit CSRF policy
- [x] Protected state-changing requests require CSRF validation

CSRF exemptions MUST remain explicit and limited to endpoints that do not
require an authenticated browser session or where the security model
specifically permits the exemption.

---

## 4. Rate Limiting

Authentication endpoints are protected against automated abuse.

- [x] Login rate limiting by IP
- [x] Login rate limiting by normalized email
- [x] Registration rate limiting by IP
- [x] Rate-limit responses use HTTP 429
- [x] Rate-limit events are auditable
- [x] Rate-limit metrics are exposed

Current configured limits:

- Login: 20 attempts / 15 minutes / IP
- Login: 5 attempts / 15 minutes / normalized email
- Register: 5 attempts / 15 minutes / IP

Rate limiting is an abuse-control mechanism and MUST NOT be interpreted as
permanent account lockout unless an explicit account-lockout mechanism is
implemented.

---

## 5. Security Audit Log

Security-sensitive authentication operations generate structured events.

- [x] Structured `SecurityEvent` model
- [x] Stable event names
- [x] Event timestamps
- [x] Event severity
- [x] Security metadata support
- [x] Login success events
- [x] Login failure events
- [x] Registration events
- [x] Rate-limit events
- [x] Session creation events
- [x] Session revocation events
- [x] Session rotation events
- [x] All-session revocation events
- [x] InMemory audit storage for tests/development
- [x] AevumDB persistent audit storage
- [x] User index
- [x] Event-type index
- [x] Timeline index
- [x] Audit storage errors do not intentionally fail authentication flows
- [x] Audit storage flush on graceful shutdown

Audit logging is best-effort. Authentication correctness MUST NOT depend
on successful audit persistence.

### Audit data minimization

Audit events MUST NOT contain:

- plaintext passwords
- raw session tokens
- private keys
- wallet mnemonics
- CSRF secrets
- authentication credentials

IP addresses and email addresses are security-sensitive metadata and should
be retained only according to the platform's applicable privacy and
retention policy.

---

## 6. Transport Security

- [x] Production session cookies require Secure
- [x] HTTPS required for production deployment
- [x] Authentication credentials are accepted only over the protected
      application transport
- [x] Session tokens are never intentionally logged
- [x] CSRF tokens are never intentionally logged
- [x] Passwords are never intentionally logged

TLS certificate management and TLS termination are deployment
responsibilities when TLS is terminated by a reverse proxy.

---

## 7. Error Handling

Public API errors must not expose internal implementation details.

- [x] Canonical API error model
- [x] Stable public error codes
- [x] Internal errors mapped to generic API responses
- [x] Authentication failures use generic unauthorized responses
- [x] Database errors are not returned directly to clients
- [x] Serialization failures are not returned directly to clients

Logs may contain additional diagnostic information, but secrets and
credentials MUST remain redacted.

---

## 8. Observability

Authentication and storage behavior is observable without exposing
credentials.

### Metrics

- [x] Login success counter
- [x] Login failure counter
- [x] Registration counter
- [x] Active-session gauge
- [x] Session creation counter
- [x] Session revocation counter
- [x] Session rotation counter
- [x] Login latency histogram
- [x] Registration latency histogram
- [x] Session rotation latency histogram
- [x] Audit write counter
- [x] Audit failure counter
- [x] Audit write latency
- [x] Storage read counter
- [x] Storage write counter
- [x] Storage error counter
- [x] Storage latency metrics
- [x] Rate-limit counters
- [x] Failure-reason labels where appropriate

Metrics labels MUST NOT contain passwords, session tokens, CSRF tokens or
other high-cardinality secrets.

---

## 9. Logging

- [x] Structured logging
- [x] Authentication events are observable
- [x] Security-sensitive credentials are excluded from logs
- [x] Session tokens are excluded from logs
- [x] Passwords are excluded from logs
- [x] CSRF tokens are excluded from logs

Production log aggregation SHOULD enforce retention and access controls
appropriate for security and privacy data.

---

## 10. Storage

Authentication persistence uses the platform storage abstraction.

- [x] `AuthStorage` abstraction
- [x] InMemory implementation for tests/development
- [x] AevumDB implementation
- [x] Atomic persistence where required by authentication state changes
- [x] Session records persisted without raw session tokens
- [x] Persistent audit event storage
- [x] AevumDB WAL-backed persistence
- [x] Graceful shutdown flush

Production MUST NOT use the InMemory authentication storage.

---

## 11. Graceful Shutdown

The backend handles controlled shutdown signals.

- [x] SIGINT handling
- [x] SIGTERM handling
- [x] Actix signal handling explicitly controlled
- [x] Graceful request shutdown
- [x] Shutdown timeout
- [x] Audit storage flush
- [x] AevumDB flush before process termination

A forced process termination may prevent asynchronous best-effort
operations from completing. Production deployments should therefore use
the graceful termination path and allow sufficient termination time.

---

## 12. Health and Readiness

- [x] Health endpoint
- [x] Readiness endpoint
- [x] Version endpoint
- [x] Storage health represented separately from application health

Readiness SHOULD be used by the process supervisor/load balancer to decide
whether the instance should receive traffic.

---

## 13. HTTP/API Hardening

Production deployment SHOULD additionally enforce:

- [ ] Request body size limits
- [ ] Request header size limits
- [ ] Strict CORS allowlist
- [ ] Security response headers
- [ ] Content-Type validation
- [ ] Appropriate request timeouts
- [ ] Appropriate keep-alive configuration
- [ ] Trusted proxy configuration
- [ ] Forwarded-header validation
- [ ] Production debug mode disabled

These controls are deployment/API hardening requirements and should not be
marked complete until verified in the running production configuration.

---

## 14. Secrets and Configuration

Production configuration MUST follow these rules:

- [ ] No credentials committed to source control
- [ ] No session secrets committed to source control
- [ ] No database credentials committed to source control
- [ ] Secrets supplied through the deployment secret mechanism
- [ ] Production secrets are not printed during startup
- [ ] Sensitive configuration is redacted from diagnostics

---

## 15. Dependency and Supply-Chain Security

Before production release:

- [ ] `cargo audit`
- [ ] Dependency vulnerability review
- [ ] Dependency lockfile committed
- [ ] Release build verified
- [ ] No unexpected dependency changes
- [ ] Unsafe code review where applicable

---

## 16. Security Testing

Authentication must be covered by both unit and integration tests.

- [x] Registration success
- [x] Registration validation failures
- [x] Duplicate registration
- [x] Login success
- [x] Wrong-password rejection
- [x] Unknown-user rejection
- [x] Session authentication
- [x] Session expiration
- [x] Logout
- [x] Logout idempotency
- [x] Session rotation
- [x] CSRF protection
- [x] Rate limiting
- [x] Audit event generation
- [x] AevumDB audit persistence
- [x] Graceful shutdown path

Additional production security testing SHOULD include concurrency,
replay, malformed-cookie, malformed-request, storage-failure and
load-testing scenarios.

---

## 17. Deployment Requirements

A production deployment MUST:

1. Use HTTPS.
2. Use AevumDB authentication storage.
3. Keep the backend outside the public static web root.
4. Keep production credentials outside source control.
5. Disable debug behavior.
6. Configure appropriate CORS policy.
7. Configure request limits and timeouts.
8. Monitor authentication failure and rate-limit metrics.
9. Monitor audit/storage failures.
10. Verify graceful shutdown behavior.
11. Maintain regular database backups according to the operational policy.
12. Restrict access to production logs and audit data.

---

## 18. Residual Risks

This document is not a security certification.

Remaining areas requiring explicit production verification include:

- reverse-proxy configuration
- TLS configuration
- CORS policy
- HTTP security headers
- request limits
- trusted proxy configuration
- secret management
- dependency vulnerability scanning
- operational backup/restore procedures
- audit retention policy
- monitoring and alert thresholds
- load/concurrency testing

Security controls should be revalidated whenever authentication,
session handling, storage, deployment topology or cryptographic parameters
change.

---

## 19. Release Gate

The authentication subsystem is considered production-ready only when:

- all mandatory `[x]` controls remain passing,
- all deployment `[ ]` controls applicable to the environment are verified,
- the full test suite passes,
- dependency security checks pass,
- production configuration has been reviewed,
- graceful shutdown has been tested,
- AevumDB backup/restore has been verified,
- no credentials or secrets are present in the repository.

---

**Aevum Platform Security Baseline**

This file documents the current security posture and must be updated
whenever security-sensitive behavior changes.
