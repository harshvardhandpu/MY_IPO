# Current State

- **Branch:** `feature/multi-registrar` (from Phase 3B closeout / `b9f8c0f`)
- **Phase 2C:** CLOSED
- **Phase 3A:** CLOSED (fixture allotment)
- **Phase 3B:** CLOSED (secure key provider, live-adapter boundary, durable worker, manual/profit APIs)
- **Phase 3C Gate 1:** **APPROVED AND LOCKED** (multi-registrar product, fail-closed normalization, provenance, recovery, real-PAN gate)
- **Phase 3C Gate 2:** NEXT — public live registrar behavior and capability validation; no real PAN
- **Independent review:** PASS — Gemini 3.6 Flash, 2026-08-28
- **Date:** 2026-08-28

## Security posture

| Item | Status |
|---|---|
| Default runtime mode | `DEVELOPMENT_SYNTHETIC` |
| Production runtime mode | Explicit `SANKET_SECURITY_MODE=PRODUCTION_SECURE` |
| OS-backed key provider | Linux Secret Service / Windows Credential Manager / macOS Keychain via `keyring` |
| Production mode + in-memory/mock provider | Rejected fail-closed |
| Fixture provider in production | Rejected |
| Live provider in development | Rejected |
| Linux Secret Service synthetic-key write/read/delete smoke | PASS |
| Real PAN or live registrar request used in verification | **No** |

Plaintext PAN remains confined to audited `with_pan(..., AllotmentCheck, ...)` closures. It is rejected from free-form allotment/profit provenance and never represented in events, SQLite projections, report DTOs, logs, AI payloads, or filenames.

## Phase 3B delivered

### Identity keys
- `RuntimeSecurityMode::{DevelopmentSynthetic, ProductionSecure}`
- `OsKeyringKeyProvider` and development-only `InMemoryKeyProvider`
- Runtime mock-backend detection and `assert_mode_allows_provider` fail-closed gate
- Stable envelope key ids for future rotation/migration

### Allotment runtime
- `LiveKfintechProvider` with offline/live/human-gate modes
- Ambiguous, unavailable, CAPTCHA, and challenge outcomes remain `UNKNOWN` / `NEEDS_HUMAN_VERIFICATION`; never false `NOT_ALLOTTED`
- Persist-before-call jobs, durable SQLite leases, restart reconciliation, cancellation, capped retry/backoff, and provider rate limiting
- Fixture provider remains the deterministic CI path and is development-only

### Desktop and service integration
- Background allotment worker plus Tauri command wiring
- Manual result ownership/account validation and provenance events
- Estimated-profit API with integer paise, explicit basis/source/as-of, and unavailable state
- UI report polling, cancellation, manual result controls, and profit estimation controls

### Research
- `docs/research/registrars/KFINTECH.md` records the JS portal/CAPTCHA reality and fail-closed integration boundary

## Verification

- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS
- `cargo test --workspace` — PASS
- `npm run check` on host Node 26 — PASS (119-file secret scan, formatting, TypeScript, 4 Vitest, 4 Python tests)
- `npm run build` — PASS
- Linux Secret Service generated-key write/read/delete smoke — PASS
- Tauri debug executable, `.deb`, and `.rpm` artifacts produced (bridge response timed out after 300 seconds, artifacts verified afterward)
- Independent review — PASS; see `docs/reviews/PHASE_3B_INDEPENDENT_REVIEW.md`

Container Node 20 cannot start the current jsdom/undici Vitest workers; host Node 26 is the canonical frontend gate and passed.

## Explicitly not performed

1. No real PAN was entered or persisted.
2. No live KFintech allotment lookup was attempted.
3. Windows Credential Manager runtime smoke remains pending a Windows host.

## Next phase candidates

1. Validate current public KFintech, Bigshare, and MUFG Intime behavior without submitting PAN.
2. Classify each provider's capabilities, health, and adapter drift.
3. Keep real-PAN use blocked pending the controlled pilot checklist and explicit owner initiation.

## Canonical commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run check
npm run build
```
