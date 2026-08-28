# Current State

- **Branch:** `feature/multi-registrar` (from Phase 3B closeout / `b9f8c0f`)
- **HEAD:** `b87bdfb` — `docs(allotment): lock gate 1 product requirements`
- **Phase 2C:** CLOSED
- **Phase 3A:** CLOSED (fixture allotment)
- **Phase 3B:** CLOSED (secure key provider, live-adapter boundary, durable worker, manual/profit APIs)
- **Phase 3C Gate 1:** **APPROVED AND LOCKED** (multi-registrar product, fail-closed normalization, provenance, recovery, real-PAN gate)
- **Phase 3C Gate 2:** **APPROVED AND LOCKED**
- **Gate 2 verdict:** **PASS WITH CHANGES REQUIRED**
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
- `docs/plans/multi-registrar-allotment/02-live-validation.md` records the current cross-provider capability matrix and Gate 2 verdict.
- KFintech: public page/API contract and 64 issue records observed; no CAPTCHA; existing adapter needs a major isolated update.
- Bigshare: three public servers available at final check; server-verified CAPTCHA requires human continuation; one transient Server 2 HTTP 503 and live page drift observed.
- MUFG Intime: public discovery returned four issue ids; result path uses JavaScript/session/token; CAPTCHA markup is currently hidden/dormant; adapter not implemented.
- Registrar details: `docs/research/registrars/KFINTECH.md`, `BIGSHARE.md`, and `MUFG_INTIME.md`.

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
3. No Bigshare or MUFG investor-result lookup was attempted.
4. No CAPTCHA was solved, submitted, or bypassed.
5. Windows Credential Manager runtime smoke remains pending a Windows host.

## Exact next task

Design Gate 3's additive capability/discovery contract, deterministic registrar registry, provider-local health checks, human-verification continuation, session/token lifecycle, and sanitized parser fixture plan. Keep real-PAN use blocked.

## Canonical commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run check
npm run build
```
