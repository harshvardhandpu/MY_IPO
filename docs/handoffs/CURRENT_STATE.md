# Current State

- **Branch:** `feature/multi-registrar` (from Phase 3B closeout / `b9f8c0f`)
- **HEAD before Gate 4D closure record:** `82971e0` (`fix(mufg): fail closed on captcha and parser ambiguity`)
- **Phase 2C:** CLOSED
- **Phase 3A:** CLOSED (fixture allotment)
- **Phase 3B:** CLOSED (secure key provider, live-adapter boundary, durable worker, manual/profit APIs)
- **Phase 3C Gate 1:** **APPROVED AND LOCKED** (multi-registrar product, fail-closed normalization, provenance, recovery, real-PAN gate)
- **Phase 3C Gate 2:** **APPROVED AND LOCKED**
- **Gate 2 verdict:** **PASS WITH CHANGES REQUIRED**
- **Phase 3C Gate 3:** **APPROVED AND LOCKED**
- **Gate 3 verdict:** **PASS — IMPLEMENTATION DESIGN READY**
- **Phase 3C Gate 4A:** **APPROVED AND LOCKED**
- **Gate 4A independent review:** PASS — Gemini 3.6 Flash, 2026-08-29
- **Phase 3C Gate 4B:** **APPROVED AND LOCKED** (KFintech bounded fixture-driven parser, guarded affirmative/pending constructors, sanitized fixtures, 13 focused tests; commit `b281379`)
- **Gate 4B independent review:** PASS WITH NON-BLOCKING FINDINGS — gpt-oss-120b via generalcompute (free), 2026-08-29; zero blocking findings
- **Phase 3C Gate 4C:** **APPROVED AND LOCKED** (Bigshare human-verification adapter: fail-closed unattended check → NEEDS_HUMAN_VERIFICATION, fixture-backed `ddlCompany` discovery + ASP.NET result parsing, guarded `not_found`/`operational()` constructors, `HumanVerificationChallenge` lifecycle state machine, 29 focused tests, sanitized fixtures with SHA-256 provenance; commit `cbbe285`)
- **Gate 4C independent review:** PASS — gpt-oss-120b via generalcompute (free), 2026-08-29; zero blocking, two accepted non-blocking findings (bounded HTML scanners; lexical RFC3339 expiry compare)
- **Phase 3C Gate 4D:** **APPROVED AND LOCKED** (MUFG session/token adapter at `db5be5d`; fail-closed correction at `82971e0`; 40 focused tests; sanitized fixtures with SHA-256 provenance)
- **Gate 4D independent re-review:** PASS — `moonshotai/kimi-k3` via NVIDIA NIM (free), 2026-08-29; original CAPTCHA visibility blocker resolved, zero blocking findings
- **Phase 3C Gate 4E:** **APPROVED AND LOCKED** (cross-provider normalization + unified allotment UI; commit `1a57379`)
- **Gate 4E independent review:** PASS — gpt-oss-120b via generalcompute (free), 2026-08-29; zero blocking findings
- **Gate 4F final system review:** **COMPLETE** — PASS WITH NON-BLOCKING FINDINGS (`moonshotai/kimi-k3` via NVIDIA NIM, free, 2026-08-29); pilot readiness YES — WITH NON-BLOCKING CONDITIONS (five pre-pilot conditions; real PAN still blocked)
- **Gate 4F Condition A TLS amendment:** **CLOSED** (`6b8ff6a`) — full gates PASS and independent focused review PASS (`gpt-oss-120b` via generalcompute)
- **Controlled pilot preparation:** **READY PENDING EXPLICIT OWNER AUTHORIZATION** — MUFG issue `11926` is available; current CAPTCHA is `Dormant`; guarded transport and all review gates pass; no real lookup executed
- **Date:** 2026-08-30

## Model routing (binding)

**GEMINI = DISABLED FOR SANKET IPO BY OWNER POLICY (2026-08-29, PERMANENT).** Gemini is barred from every Sanket IPO role — implementation, debugging, architecture, research, independent review, and fallback execution. Prior Gemini review artifacts (2C, 3B, 4A) remain historical fact; no future Gemini use in any capacity.

Independent-reviewer selection rules for this repo: model must be (1) free — zero paid usage, any 402/insufficient-balance route excluded; (2) healthy and coherent; (3) independent of implementers. Gate 4D route record, 2026-08-29: generalcompute `gpt-oss-120b` returned HTTP 404 and was unavailable; DeepSeek V4 Pro/Flash on NVIDIA NIM timed out; `moonshotai/kimi-k3` on NVIDIA NIM was healthy, free, metadata-confirmed, and selected for the initial review and correction re-review. The separate TokenRouter Kimi route returned 402 and remains excluded. Gate 4D original implementation routing was GLM 5.3 via TokenRouter; the correction runtime was `gpt-5.6-sol` via `openai-codex`; Kimi K3 was independent of both.

## Security posture

| Item | Status |
|---|---|
| Default runtime mode | `DEVELOPMENT_SYNTHETIC` |
| Production runtime mode | Explicit `SANKET_SECURITY_MODE=PRODUCTION_SECURE` |
| OS-backed key provider | Linux Secret Service / Windows Credential Manager / macOS Keychain via `keyring` |
| Production mode + in-memory/mock provider | Rejected fail-closed |
| Fixture provider in production | Rejected |
| Live provider in development | Rejected |
| Real investor lookup | **Not authorized — transport ready, explicit owner YES required** |
| Linux Secret Service synthetic-key write/read/delete smoke | PASS |
| Real PAN or investor-result request used in verification | **No** |

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
- MUFG Intime: current public discovery exposes Symbiotec Pharmalab Limited as issue `11926`; the current CAPTCHA wrapper is positively hidden and classifies `Dormant`; identifier-free page/discovery/token bootstrap passes. Guarded real-investor transport is implemented, but authorization remains independently false.
- Registrar details: `docs/research/registrars/KFINTECH.md`, `BIGSHARE.md`, and `MUFG_INTIME.md`.

### Gate 3 provider design

- One provider-independent domain contract with provider-owned HTTP/browser/hybrid transport.
- Typed capabilities, first-class human verification, safe continuation metadata, provider-specific retry, and structural drift fingerprints.
- KFintech obsolete adapter is replaced rather than patched; Bigshare pauses for legitimate human verification; MUFG runs an HTTP token/session proof before browser fallback.
- Provider cookies, request tokens, challenge content, answers, and response bodies remain ephemeral; no generic persisted provider session exists.
- Restart preserves the durable job but expires stale continuations, then enters `PREPARING_PROVIDER_SESSION` or `VERIFICATION_REQUIRED_REFRESH` and creates a fresh legitimate session.
- `NOT_ALLOTTED` requires confirmed provider, issue, structure, provider-specific negative marker, and zero parser ambiguity.
- Shared HTTP policy enforces TLS, provider-domain/redirect allowlists, hard bounds/timeouts/cancellation, isolated cookies, content-type checks, explicit user agent, and no sensitive logging/body persistence.
- Sanitized fixtures record source/retrieval/type/sanitization/SHA-256 provenance and structural fingerprints where practical; live drift degrades health.
- `LIVE_ADAPTER_IMPLEMENTED != REAL_INVESTOR_LOOKUP_AUTHORIZED`; Gate 4 may implement machinery but may not run a real-PAN investor lookup.
- Transport decision: `docs/architecture/decisions/ADR-registrar-transport.md`.
- Full design: `docs/plans/multi-registrar-allotment/03-provider-design.md`.

### Gate 4A shared runtime

- Typed provider capabilities, provider ids, registry, transport/session requirements, human-verification requirements, and provider-specific retry/rate policy.
- `ProviderAllotmentResult::confirmed_not_allotted` requires confirmed provider, issue, expected structure, recognized negative marker, and no ambiguity.
- Durable challenge projection stores safe metadata and an opaque continuation reference only; provider cookies, request tokens, challenge material, answers, and response bodies remain ephemeral.
- Startup reconciliation preserves jobs, clears stale leases/references, expires continuations, and moves work to `PREPARING_PROVIDER_SESSION` or `VERIFICATION_REQUIRED_REFRESH`.
- Sanitized fixture provenance validates provider/source/retrieval/type/SHA-256/fingerprint metadata.
- `LIVE_TRANSPORT_IMPLEMENTED` and `REAL_INVESTOR_LOOKUP_AUTHORIZED` remain separate; the latter is fail-closed false.
- Manual negative reports remain `MANUAL_RESULT`, never provider-confirmed `NOT_ALLOTTED`.
- Review: `docs/reviews/GATE_4A_INDEPENDENT_REVIEW.md` — PASS.

### Gate 4D MUFG adapter plus pilot-blocker remediation

- Identifier-free issue discovery parses only verified JSON-wrapped MUFG XML structure.
- Session cookie and JSON-delivered request token remain ephemeral; lookup-request debug output is redacted and request payloads are not persisted.
- CAPTCHA detection walks the bounded marker ancestor chain: visible is `Required`, positive hidden evidence is `Dormant`, and malformed/ambiguous markup is `Unknown`.
- Native rustls transport uses exact-host allowlisting, no redirects, bounded bodies/timeouts, isolated cookies, browser-compatible AES token encryption, and an independent authorization gate before `SearchOnPan`.
- Positive and negative financial results remain reachable only through their guarded proof constructors; unknown/drifted/provider failures remain operational.
- Implementation `db5be5d`; correction `82971e0`; independent correction re-review PASS.
- Review: `docs/reviews/GATE_4D_INDEPENDENT_REVIEW.md`.

## Verification

- `cargo fmt --all -- --check` — PASS (remediation)
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS (remediation)
- `cargo test --workspace` — PASS (remediation; identifier-free live precheck intentionally ignored in normal runs)
- `npm run check` on host Node 26 — PASS (161-file secret scan, formatting, TypeScript, 5 Vitest, 4 Python tests)
- `npm run build` — PASS
- MUFG focused suite — PASS (48 passed; identifier-free live precheck intentionally ignored)
- MUFG fixture SHA-256 provenance and PAN/allotment invariants — PASS
- Linux Secret Service generated-key write/read/delete smoke — PASS
- ProductionSecure application status (`os-keyring`, no blocker) and production rejection of the in-memory/dev provider — PASS
- Identifier-free MUFG precheck — PASS; issue `11926` available; CAPTCHA `Dormant`; session/token bootstrap PASS; real PAN lookup NOT EXECUTED
- MUFG remediation independent review — PASS, approve=true, zero findings; `gpt-oss-120b` via generalcompute (free), served model confirmed
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

Gate 4F pre-pilot conditions A–E are **CLOSED** (`75af0ce` plus Condition A runtime amendment `6b8ff6a`) and independently verified **PASS**. Full verification is green (fmt, clippy `-D warnings`, workspace tests, npm check/build, 157-file secrets scan, diff check). The amendment corrects the original internal ureq `_tls` selection to the public `rustls` feature and adds a local HTTPS no-panic regression.

Exact IPO resolution is complete: **Symbiotec Pharmalab Limited** → **MUFG Intime India Private Limited** → issue id **`11926`** → official MUFG Initial Offer service. The identifier-free live precheck passes with CAPTCHA `Dormant`, isolated session/token bootstrap, and no investor-result request. Guarded transport, full verification, secret scanning, and independent free-model review pass. Real lookup remains unauthorized until a separate explicit owner YES; no PAN was accessed. See `docs/security/CONTROLLED_REAL_PAN_PILOT.md` and `docs/reviews/MUFG_PILOT_BLOCKER_REMEDIATION_INDEPENDENT_REVIEW.md`.

## Gate 4E implemented

Cross-provider normalization and unified allotment UI, on top of the locked KFintech/Bigshare/MUFG adapters:

- **Deterministic registrar resolution** — `ProviderRegistry::resolve_registrar()` maps registrar ids (`kfintech`, `bigshare`, `mufg_intime`) to typed `ProviderDescriptor` (id + name + `ProviderId` + official status URL). Unknown registrars fail closed; no substring/name guessing.
- **Durable job resolution** — `execute_allotment_check` resolves registrar/provider from the persisted job state, not caller input twice; restart-safe prepare/verify/cancel/reconcile all use the same authoritative registry-resolved values.
- **Dev-mode enqueue gate** — `validate_allotment_provider` now permits live adapters (kfintech/bigshare/mufg) to be enqueued under `DEVELOPMENT_SYNTHETIC` so they reach their typed operational/human-verification states; the unattended run path still fails closed before any PAN access or network lookup.
- **Cross-provider normalization** — `NOT_ALLOTTED` only from provider-confirmed `NegativeResultProof`; `NOT_FOUND` / `UNKNOWN` / CAPTCHA / timeout / 503 / 429 / parser error never manufacture `NOT_ALLOTTED`. Estimated vs realized profit kept distinct with provenance.
- **Provider failure isolation** — per-account statuses; partial completion preserves finals; `run_allotment_job_once` lease-gated against duplicate execution.
- **Unified UI** — one workflow for all three registrars: candidate listing (registrar, provider, health), check/retry/manual-fallback, account-level progress (Waiting/Checking/Allotted/Not allotted/Not found/Unknown/Verification required/Retry scheduled/Provider unavailable/Cancelled/Completed), report card with provider + provenance + checked time + masked PAN + shares/lots + profit estimate + basis. UNKNOWN is visually distinct from NOT_ALLOTTED. Full PAN, cookies, tokens, and raw responses never cross the Tauri DTO boundary.

Verification (all green): `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `npm run check` (152-file secret scan + typecheck + 5 Vitest + 4 Python), `npm run build`.

## Canonical commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run check
npm run build
```
