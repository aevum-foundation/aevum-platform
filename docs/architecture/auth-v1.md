# Aevum Platform — Auth Security Contract v1

## Status
AUTH-01 — SECURITY CONTRACT — FROZEN

## Principles
1. Website Account != Aevum Wallet
2. Mnemonics and private keys NEVER in auth system
3. Passwords never stored in plaintext
4. Session identifiers never stored in plaintext
5. HttpOnly Secure cookies for session transport
6. SameSite=Lax or stricter
7. Argon2id for password hashing
8. Rate limiting for /register and /login
9. CSRF protection for state-changing requests
10. Same error messages for login to prevent account enumeration

## Data Model

### User
```

platform:user:{user_id}

· id: uuid
· email: string (unique)
· password_hash: string (Argon2id)
· status: active | suspended | deleted
· created_at: timestamp
· updated_at: timestamp

```

### Session
```

platform:session:{session_id_hash}

· id: uuid
· user_id: uuid
· created_at: timestamp
· expires_at: timestamp
· revoked_at: timestamp (optional)
· user_agent: string (optional)
· ip_address: string (optional)

```

## API Endpoints (v1)

- POST /api/v1/auth/register
- POST /api/v1/auth/login
- POST /api/v1/auth/logout
- GET  /api/v1/auth/me

## Cookie Settings

- Name: aevum_session
- HttpOnly: true
- Secure: true (production)
- SameSite: Lax
- Path: /
- Max-Age: 7 days (configurable)

## Rate Limiting

- /register: 10 requests per hour per IP
- /login: 20 requests per hour per IP

## CSRF Protection

- State-changing requests: POST, PUT, PATCH, DELETE
- CSRF token in header: X-CSRF-Token
- Token bound to session

## Error Responses

- Always same message for invalid credentials
- No user enumeration via error messages
- HTTP 401 for unauthenticated
- HTTP 403 for forbidden

## Security Tests

- Password hashing verified
- Session storage verified
- CSRF protection verified
- Rate limiting verified
- Brute force protection verified
