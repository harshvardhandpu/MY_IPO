# Current State

- **Branch:** `feature/sensitive-identity`
- **Milestone base:** annotated tag `phase-2b-green`
- **Phase:** Phase 2C/3A complete — first usable local investment workflow
- **Date verified:** 2026-08-28

## Implemented workflow

### Onboarding and private identity

- First-run React onboarding captures owner name, email, primary account, broker, UPI, PAN, and explicit local-storage consent.
- Rust validates PAN/UPI at a narrow Tauri command boundary and immediately encrypts `IdentitySecret` with XChaCha20-Poly1305.
- Member/friend profile JSON and UI responses contain masked PAN only (`ABCDE****F`).
- Friend accounts support 10% default profit-share eligibility and archive-not-delete behavior.
- Captured `email` and `broker` are currently acknowledged by the command but not persisted because the domain profile does not yet contain those fields.

### Investment workflow

- `InvestmentSession`, `IpoApplication`, and `InvestmentAllocation` use UUIDv7 IDs.
- Money is integer paise; percentages are integer basis points. No financial float is stored in Rust.
- The React investment studio supports daily capital, dynamic multi-IPO rows, account selection, CHECK, EDIT, and SUBMIT.
- CHECK constructs the approved multi-IPO DTO: session ID, declared capital, account count, typed IPO names/amounts, algorithm version, and public references only.
- `RankingAlgorithm` isolates ranking policy behind a versioned interface.
- `DevRankingAlgorithm` is deterministic (`50% / 30% / 10%`, fourth-and-later skipped) and always labeled **DEVELOPMENT ALGORITHM — NOT INVESTMENT ADVICE**.
- SUBMIT seals immutable events, appends them to MemberVault, and applies SQLite projections.

### Projection-driven dashboard

SQLite schema version 2 projects:

- members and active/archive friend accounts;
- IPOs, investment sessions, applications, and per-account allocations;
- recommendations and algorithm versions;
- dashboard counts and total planned capital.

SQLite remains derived state. `LocalIndex::rebuild_from_events` recreates projections from verified event envelopes.

## Command surface

The webview receives no arbitrary filesystem or shell capability. Registered commands are:

- `get_app_status`
- `onboard_member`
- `add_friend`
- `archive_friend`
- `list_members`
- `list_friends`
- `check_recommendation`
- `submit_investment`
- `get_dashboard`

Application-service instances reopen the same vault and SQLite paths on each command call.

## Canonical events

- `MEMBER_CREATED`
- `FRIEND_ADDED`
- `FRIEND_ARCHIVED`
- `INVESTMENT_SESSION_CREATED`
- `IPO_APPLICATION_CREATED`
- `ALLOCATION_ADDED`
- `INVESTMENT_SESSION_SUBMITTED`
- `INVESTMENT_RECOMMENDATION_GENERATED`
- `INVESTMENT_RECOMMENDATION_APPLIED`

Generated event IDs are UUIDv7 when a caller does not supply one. SHA-256 envelope verification remains mandatory during replay.

## Security invariants proven

1. Full PAN and UPI are encrypted before persistence.
2. Plaintext identity does not appear in ordinary profiles, event JSON, encrypted-envelope bytes, SQLite bytes, AI request JSON, service errors, or generated log files.
3. PAN has no serializable production type; `Display`/`Debug` remain masked or redacted.
4. AI-bound request validation rejects PAN-like values, UPI IDs, proof refs, private paths, and private field names.
5. SQLite permits `masked_pan` only; no full-PAN-capable column exists.
6. Friend removal archives rather than deletes.
7. Validation errors do not echo rejected private values.
8. The repository scanner fails closed on PAN-like or credential-like values outside synthetic test fixtures.

The end-to-end service regression is `apps/desktop/src-tauri/tests/security_boundaries.rs`.

## Verification (all passing)

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`: **116 Rust tests**
- `npm run check`:
  - secret scan: **99 files**
  - Biome format check
  - TypeScript project build
  - **4 Vitest UI tests**
  - **4 Python scanner tests**
- `npm run build`: Vite production bundle generated successfully
- Browser QA: dashboard and investment studio rendered without console/JS errors; action rail and disabled-state defects corrected
- Linux Tauri smoke:
  - transient user service became active;
  - real `target/debug/sanket-ipo` launched;
  - native AX tree exposed the complete first-run onboarding form;
  - journal contained zero runtime error/panic lines;
  - isolated smoke data was removed after verification.

## Phase 2C commits

```text
b834d89 feat(domain): investment sessions, applications, allocations and events
4e0ce96 feat(ranking): versioned algorithm interface, deterministic dev algo, multi-IPO request
704ae2e feat(index): project investment events into SQLite (schema v2)
fdc6795 feat(desktop): secure onboarding, recommendation, submit, and dashboard commands
7f9f7e6 feat(ui): add private onboarding and investment workflows
66f322a test(security): prove private identity never crosses storage or AI boundaries
```

## Status labels and deferred work

| Item | Status |
|---|---|
| XChaCha20-Poly1305 identity encryption | IMPLEMENTED + TESTED |
| `KeyProvider` abstraction | IMPLEMENTED + TESTED |
| Device-ID-derived development key | IMPLEMENTED for development only; **not protected key storage** |
| Windows Credential Manager provider | DEFERRED |
| Linux Secret Service/keyring provider | DEFERRED |
| Windows vault ACL hardening | DEFERRED |
| Production owner ranking algorithm | DEFERRED; current algorithm is development-only |
| Live registrar/KFintech automation | DEFERRED |
| Email/broker persistence in member profiles | DEFERRED |
| Key rotation tooling | DEFERRED |

## Known limitations

- A device UUID is not a secret. SHA-256 derivation from device ID supplies stable development behavior only and must not be represented as production-grade key protection.
- Native OS key-store integration and Windows ACL verification are required before production identity enrollment.
- Multi-file vault writes plus SQLite projection are not one atomic transaction. Immutable events are the durable replay source; a projection failure is recoverable by rebuild.
- The live IPO ranking algorithm and public-data inputs have not been supplied. Current output is deterministic scaffolding, not investment advice.
- Allotment checking and registrar integration remain unavailable; the UI marks that action disabled.

## Next exact phase

1. Implement native Windows Credential Manager and Linux Secret Service `KeyProvider`s.
2. Add Windows packaging/ACL verification and key-rotation migration.
3. Extend member profiles deliberately if email/broker persistence is approved.
4. Add public IPO metadata ingestion with provenance and stale-data status.
5. Replace the development algorithm only after the owner supplies a versioned production contract and fixtures.
6. Build registrar adapters behind purpose-scoped sensitive access; preserve `UNKNOWN` for ambiguous results.

## Commands

```bash
npm install
npm run check
npm run build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run tauri:dev
```
