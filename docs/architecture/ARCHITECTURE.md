# Architecture

## System shape

Sanket IPO is one local-first Tauri desktop product with modular Rust services and a React UI. It is not a networked microservice system.

```text
React UI
  │ narrow typed Tauri commands (no arbitrary fs/shell)
  ▼
Desktop Application service
  ├── identity-security ── encrypted identity ── MemberVault
  ├── domain events ── immutable event files ── local-index projection
  ├── ranking interface ── sanitized CHECK DTO ── development algorithm
  └── sync-engine policy (asynchronous; never blocks local writes)

Private identity ─X─ AI / public intelligence boundary
```

## Durable and derived state

- **Durable truth:** immutable, schema-versioned event envelopes in MemberVault.
- **Private identity:** versioned XChaCha20-Poly1305 envelopes under `_secure_identity`; ordinary profile JSON contains masked PAN only.
- **Derived state:** restricted local SQLite projection (schema v2), rebuildable from verified events.
- **Device settings:** stable local device identity and projection schema metadata.
- **Synchronization:** asynchronous policy contracts; local operation is the availability boundary.

## Workspace boundaries

- `apps/desktop`: React UI, Tauri command adapter, and cohesive application service.
- `crates/domain`: event envelopes, member/friend, investment, money, and basis-point types.
- `crates/identity-security`: PAN/UPI validation, redaction, AEAD envelopes, and purpose-scoped access.
- `crates/member-vault`: encrypted identity, safe profiles, and immutable event persistence.
- `crates/local-index`: SQLite schema v2, event projection, dashboard queries, and rebuild.
- `crates/intelligence-vault`: sanitized AI/public-intelligence DTOs and adversarial boundary validation.
- `crates/ranking`: versioned ranking interface and explicitly development-only algorithm.
- `crates/audit`: immutable sensitive-access event bridge.
- `crates/device-settings`: atomic device settings and stable device ID.
- `crates/sync-engine`: local-first Git synchronization policy contracts.

## Command and authority boundary

The webview can invoke only registered business commands. It never receives filesystem, shell, key, cipher, SQLite connection, or vault-path authority. Sensitive values enter Rust only in onboarding/friend request DTOs, are validated, converted to transient `IdentitySecret`, encrypted immediately, and dropped.

```text
PAN + UPI request
  -> Rust validation
  -> transient IdentitySecret
  -> XChaCha20-Poly1305 envelope
  -> encrypted MemberVault file

UI/event/SQLite/AI response
  <- masked PAN or non-identity metadata only
```

CHECK has a separate public/sanitized shape. Account IDs are reduced to `account_count` before the `InvestmentDecisionRequest` crosses the intelligence boundary.

## Investment write path

```text
CHECK
  -> validate approved multi-IPO request
  -> RankingAlgorithm::rank
  -> labeled Recommendation
  -> recommendation event + projection

SUBMIT
  -> InvestmentSessionCreated
  -> IpoApplicationCreated (per selected IPO)
  -> AllocationAdded (per selected account)
  -> InvestmentSessionSubmitted
  -> append each sealed event
  -> apply each SQLite projection
```

Event append precedes projection. If projection fails, immutable events remain available for deterministic rebuild. Multi-event submission is not currently a single atomic transaction.

## Security implementation status

- XChaCha20-Poly1305 identity encryption: **IMPLEMENTED, TESTED**.
- Redacted PAN/UPI types and error behavior: **IMPLEMENTED, TESTED**.
- `KeyProvider` abstraction: **IMPLEMENTED, TESTED**.
- Stable SHA-256 device-ID-derived development key: **IMPLEMENTED for development only**.
- Windows Credential Manager and Linux Secret Service providers: **DEFERRED**.
- Windows ACL hardening: **DEFERRED**.

A device ID is not protected key material. The development derivation must be replaced before production enrollment.

## Failure model

- Local immutable event persistence is the success boundary for investment writes.
- SQLite is derived and rebuildable.
- Git sync, public intelligence, and registrar work must be asynchronous and expose explicit status.
- Unknown registrar output remains `UNKNOWN`, never `NOT_ALLOTTED`.
- Validators fail closed and return static errors that do not echo private input.

## Significant decisions

1. Tauri + React keeps one desktop product while Rust owns trust boundaries and durable writes.
2. Integer paise and basis points prevent floating-point financial drift.
3. UUIDv7 provides sortable entity and generated event IDs.
4. Event envelopes are authoritative; SQLite is only a local projection.
5. Private member identity and sanitized public intelligence have separate DTOs and persistence capabilities.
6. Ranking is versioned behind an interface; current deterministic output is prominently not investment advice.
7. Friend accounts archive instead of delete so history and notifications remain auditable.
