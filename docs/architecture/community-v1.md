# Community v1 — Architecture

Status: **B-1 complete / production baseline**

Community v1 provides the platform-level community profile subsystem.
It is intentionally separated from the AUTH user model and from future
Forum/Social functionality.

---

## 1. Purpose and Scope

Community v1 owns:

- public community identity;
- username;
- display name;
- bio;
- community role;
- profile storage;
- public/private profile APIs.

Community v1 does **not** own:

- authentication credentials;
- sessions;
- passwords;
- email verification;
- 2FA;
- avatar blob storage;
- forum topics/posts/comments;
- search indexing;
- notifications.

Those concerns remain separate subsystems.

---

## 2. Domain Boundaries

### User vs CommunityProfile

AUTH owns the platform `User`.

Community owns:

```text
CommunityProfile

A CommunityProfile is keyed by user_id and does not duplicate
authentication or security metadata.

Private community responses must not expose:

password data;
session data;
email;
email verification state;
security metadata.
AuthContext

Authentication middleware establishes AuthContext.

Community HTTP handlers consume the authenticated user identity from
that context and delegate business logic to CommunityApi.

A dedicated CommunityContext is deferred until community authorization
requires additional request-scoped state.

3. Storage Layout

Community storage uses the namespace:

platform:community:*

Canonical keys:

platform:community:profile:{user_id}
platform:community:profile:username:{normalized_username}
platform:community:role:{user_id}

The username index stores the owning user_id.

The role value is one of:

user
moderator
admin

An absent role defaults to user.

4. Username Contract

Username normalization:

trim surrounding whitespace;
convert to ASCII lowercase;
validate the normalized value.

Allowed characters:

a-z
0-9
_
-

Length:

minimum: 3
maximum: 32

Reserved usernames:

admin
administrator
root
system
support
help
staff
moderator
mod
api
auth
login
logout
register
settings
profile
forum
community
search
notifications
me
u
www
mail
email
security
aevum
aevumchain
foundation
official
Canonical vs normalized

The stored username preserves the trimmed user-supplied spelling.

The stored normalized_username is the lowercase uniqueness key.

Therefore:

Alice
alice
ALICE

refer to the same username identity.

Immutability

Username is immutable after profile creation.

An update may omit username, or may supply the same normalized username
idempotently.

A different normalized username returns:

USERNAME_IMMUTABLE
5. Profile Fields
display_name
Unicode supported.
Maximum 64 Unicode scalar values.
Trimmed.
Empty value becomes None.
Control characters are rejected.
Bidirectional override/isolate characters are rejected.

Rejected bidi ranges:

U+202A..U+202E
U+2066..U+2069
bio
Unicode supported.
Maximum 500 Unicode scalar values.
Trimmed.
Empty value becomes None.
Control characters are rejected except newline.
The same bidi override/isolate characters are rejected.
6. Storage Layer

The storage abstraction is:

CommunityStorage

Implementations:

InMemoryCommunityStorage
AevumDbCommunityStorage

The storage layer owns persistence and storage-level invariants.

The service layer owns domain validation and API semantics.

AevumDB currently provides atomic WriteBatch commits but does not provide
a conditional PutIfAbsent / CAS primitive.

7. Service Layer

The domain service is:

CommunityService<S>

where S: CommunityStorage.

The service owns:

validation;
profile creation;
profile update semantics;
username immutability;
duplicate username error mapping;
public/private DTO construction;
role lookup.

HTTP handlers must not duplicate these rules.

Create

Initial profile creation requires a username.

Missing username returns:

USERNAME_REQUIRED
Update

For an existing profile:

omitted username keeps the current username;
identical normalized username is accepted;
different normalized username is rejected;
display name and bio use full replacement semantics.
8. HTTP API

Current endpoints:

GET /api/v1/community/u/{username}
GET /api/v1/community/me/profile
PUT /api/v1/community/me/profile
Public profile

The public DTO whitelist is exactly:

username
display_name
bio
avatar_url
joined_at

No user ID, email, authentication, or security metadata is exposed.

avatar_url is currently None.

The field is reserved for a future public avatar contract and does not
reuse the private AUTH avatar endpoint implicitly.

Private profile

The private DTO adds:

role

to the public fields.

CSRF

State-changing Community requests require both:

__Host-aevum_csrf cookie
X-CSRF-Token header

The values must match.

The existing CSRF middleware is the outer middleware for mutating requests.
Therefore an unauthenticated unsafe request may be rejected with 403
before authentication reaches the inner middleware.

This ordering is intentional and must not be changed merely to alter test
status codes.

9. Security Invariants
Username uniqueness

The following invariants must hold:

one CommunityProfile per user_id;
one user_id per normalized username.

Phase B v1 guarantees username uniqueness under the platform's
single authoritative writer / single-instance invariant.

In-memory and AevumDB implementations use a per-username async mutex to
serialize the application-level uniqueness check.

This mutex is an application-level concurrency mechanism, not an AevumDB
transaction primitive.

Multi-instance limitation

The current:

get -> check -> put

pattern cannot provide cross-process uniqueness without a conditional
write primitive.

Multi-instance scale-out therefore requires:

AevumDB-TX-1

before Community uniqueness can be considered cross-instance atomic.

Visibility

Public and private responses use explicit DTO whitelists.

Internal model fields must not be serialized directly as API responses.

10. Known Limitations and Backlog
AevumDB-TX-1

Add a conditional write primitive to AevumDB.

See:

docs/backlog/aevumdb-tx-1.md

This is required for safe multi-instance unique-index claims.

Multi-instance scale-out

Community v1 assumes one authoritative application writer.

Distributed application instances are outside the current contract.

Avatar URL

Community currently returns:

avatar_url: None

until a public avatar access contract is explicitly designed.

CommunityContext

A dedicated CommunityContext is deferred until forum/community
authorization requires it.

Search

Search is intentionally not implemented through direct AevumDB scans.

Future search must follow:

canonical write
    ->
community event
    ->
indexing pipeline
    ->
explicit search index
11. Test Coverage

B-1 covers:

username validation;
reserved usernames;
Unicode display names;
bio validation;
control and bidi rejection;
profile creation;
profile update;
username immutability;
duplicate username handling;
role storage;
InMemory storage;
AevumDB storage;
concurrent username creation;
public/private response contracts;
authentication;
CSRF enforcement;
HTTP endpoint behavior;
case-insensitive username lookup.

The B-1 baseline is considered complete only with the full repository test
suite passing without regressions.

12. Architectural Rules for Future Phases

Future Community phases must preserve these boundaries:

Forum must not move authentication data into CommunityProfile.
Forum must not duplicate username uniqueness logic.
Public profile DTOs must remain explicit whitelists.
HTTP handlers must remain thin.
Community business rules belong in CommunityService.
Storage invariants belong in CommunityStorage implementations.
AevumDB conditional-write work must remain infrastructure-level.
Multi-instance claims must not be introduced while the single-instance
invariant remains undocumented or unenforced.
