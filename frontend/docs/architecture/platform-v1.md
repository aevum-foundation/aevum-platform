# Aevum Web Platform Architecture v1

## Status

ARCHITECTURE DRAFT — NOT PRODUCTION

## Principles

1. Website Account != Aevum Wallet.
2. Mnemonics and private keys never leave the user's device.
3. Platform state and protocol state are logically isolated.
4. Platform failures must never affect L1 consensus.
5. Frontend must not know whether the protocol provider is Mock or Mainnet.
6. AevumDB is the storage layer.
7. Protocol state is never duplicated into platform storage without an explicit reason.
8. Authentication and authorization are server-side responsibilities.
9. Forum, comments and chat must have moderation and rate limiting.
10. Mainnet integration must happen through AevumProvider.

## Storage Namespaces

### Platform

platform:user:
platform:profile:
platform:session:
platform:role:
platform:content:
platform:comment:
platform:forum:
platform:chat:
platform:moderation:
platform:analytics:

### Protocol

protocol:epoch:
protocol:snapshot:
protocol:account:
protocol:transaction:
protocol:presence:

Platform code must not directly manipulate protocol consensus state.

## Provider Architecture

Frontend
    |
    +-- PlatformAPI
    |
    +-- AevumAPI
              |
              +-- MockAevumProvider
              |
              +-- NetworkAevumProvider

## Mainnet Switch

Development:
MockAevumProvider

Production:
NetworkAevumProvider

The frontend contract remains unchanged.

## Platform Modules

- Auth
- Users
- Profiles
- Content
- Comments
- Moderation
- Forum
- Chat
- Analytics

## Security

- No mnemonic storage.
- No private-key storage.
- Passwords are never stored plaintext.
- Authorization is enforced server-side.
- Rate limiting is mandatory for authentication, comments and chat.
- Platform failures must not affect protocol consensus.

## Development Order

1. Architecture freeze
2. Platform API foundation
3. AevumDB platform storage layer
4. Authentication
5. Users and profiles
6. Content
7. Comments
8. Moderation
9. Forum
10. Chat
11. Analytics
12. NetworkProvider
13. Wallet integration
14. Security audit
15. Load testing
16. Production hardening
17. Mainnet integration
