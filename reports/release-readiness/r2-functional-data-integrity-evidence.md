# SANKET-R2 — Functional + Data Integrity Closure Evidence

- **Recorded:** 2026-09-07T14:06:00+05:30
- **Card:** `t_1705d13c`
- **Scope:** existing functional and data-integrity behavior only; no feature expansion, provider integration, financial mutation, commit, or push.

## Verified local functional/data behaviors

- Local-index projection replay is idempotent; rebuilds sessions/allocations from events; VOIDED sessions are excluded from submitted applications; projection tables contain no full PAN column.
- Durable allotment leases prevent duplicate ownership, survive reopen, expire ephemeral continuation on restart, preserve jobs, replay retry metadata, and never re-execute finalized/cancelled jobs.
- Domain investment events seal correctly; submitted/voided state transitions and duplicate/invalid/affirmation constraints are enforced; member/friend archive and share rules remain intact; money arithmetic is integer/deterministic and fails closed on overflow/negative results.
- Desktop service flows preserve plaintext identity exclusion from projections, historical duplicate rejection, replacement after VOIDED, owner-chain mapping, and local projection status.
- Provider contracts preserve typed registry routing, provider-specific retry, explicit negative-proof requirements, provenance, safe URL/status handling, and fail-closed unknown/ambiguous responses. Live transport was not invoked.
- Device settings and member-vault persistence/reload/path-traversal/security-invariant tests pass.
- Frontend existing tests pass; no UI scope was added.

## Targeted execution gates

| Gate | Result |
|---|---|
| `cargo test -p sanket-local-index --test projection` | PASS, 7 tests |
| `cargo test -p sanket-local-index --test allotment_runtime` | PASS, 4 tests |
| `cargo test -p sanket-local-index --test initialization` | PASS, 1 test |
| `cargo test -p sanket-domain --test investment` | PASS, 11 tests |
| `cargo test -p sanket-domain --test members` | PASS, 9 tests |
| `cargo test -p sanket-domain --test event_envelope` | PASS, 4 tests |
| `cargo test -p sanket-domain --test foundation_types` | PASS, 2 tests |
| `cargo test -p sanket-domain --test money` | PASS, 11 tests |
| `cargo test -p sanket-allotment --test fixture_provider` | PASS, 2 tests |
| `cargo test -p sanket-allotment --test provider_contract` | PASS, 13 tests |
| desktop `service_flow` | PASS, 4 tests |
| desktop `status` | PASS, 1 test |
| desktop `allotment_security` | PASS, 2 tests |
| desktop `allotment_lease_and_url` | PASS, 6 tests |
| desktop `provider_credentials` | PASS, 4 tests |
| desktop `key_provider_gate` | PASS, 2 tests |
| desktop `security_boundaries` | PASS, 1 test |
| desktop `upstox` | PASS, 7 tests |
| `npm run test` | PASS, 10 frontend tests and 4 Python tests |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS |
| `npm run check` | PASS: 216 files secret scan, format, typecheck, frontend/Python tests |
| `git diff --check` | PASS |

The workspace run included offline provider fixture/contract suites. The single ignored test is the explicitly ignored identifier-free live MUFG precheck; no live provider/MUFG request was made.

## Authority and release separation

- `SANKET-R1` is native `done` after direct `ASTRA_HIGH` phase-closure approval; decision artifact SHA-256 `14947dea7c447b7f45220d67cb3fc5d0659128f628cf4817bd6281327e62ccdb`.
- This R2 evidence is for functional/data closure. R2 phase closure still requires `ASTRA_HIGH` review.
- R3 Linux package/install/smoke, R4 Windows package/install/smoke, R5 cross-platform acceptance, and SANKET-FINAL release approval remain open.

## Protected boundaries

- Protected HEAD: `d3864557ee56282214733d44149cb2ed1fbc0ee6`.
- Worktree remains intentionally dirty; this card produced evidence only.
- No PAN, credential value, keyring value, live business provider/MUFG request, financial mutation, commit, or push occurred.
