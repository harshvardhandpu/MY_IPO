# Current State

- **Branch:** `feature/multi-registrar` (from Phase 3B closeout / `b9f8c0f`)
- **HEAD:** Gate 4A implementation lock (parent `d2bf075`)
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
- **Date:** 2026-08-29

## Model routing (binding)

**GEMINI = DISABLED FOR SANKET IPO BY OWNER POLICY (2026-08-29, PERMANENT).** Gemini is barred from every Sanket IPO role — implementation, debugging, architecture, research, independent review, and fallback execution. Prior Gemini review artifacts (2C, 3B, 4A) remain historical fact; no future Gemini use in any capacity.

Independent-reviewer selection rules for this repo: model must be (1) free — zero paid usage, any 402/insufficient-balance route excluded; (2) healthy and coherent on a probe; (3) independent of implementers (GLM 5.3 = active chat model, DeepSeek V4 Pro = Gate 4B implementer). Route inventory 2026-08-29: groq `openai/gpt-oss-120b` healthy but 8K TPM cap rejects the evidence pack; generalcompute `gpt-oss-120b` healthy, coherent, **selected**; generalcompute MiniMax M2.7 degenerate — verdicts rejected; TokenRouter Qwen 403; Kimi K3 402 paid; omni proxy down; inferex deepseek-v4-flash rate-limited (low-priority fallback only).

## Security posture

| Item | Status |
|---|---|
| Default runtime mode | `DEVELOPMENT_SYNTHETIC` |
| Production runtime mode | Explicit `SANKET_SECURITY_MODE=PRODUCTION_SECURE` |
| OS-backed key provider | Linux Secret Service / Windows Credential Manager / macOS Keychain via `keyring` |
| Production mode + in-memory/mock provider | Rejected fail-closed |
| Fixture provider in production | Rejected |
| Live provider in development | Rejected |
| Real investor lookup | **Blocked pending separately authorized controlled pilot** |
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
- `LIVE_ADAPTER_IMPLEMENTED` and `REAL_INVESTOR_LOOKUP_AUTHORIZED` remain separate; the latter is fail-closed false.
- Manual negative reports remain `MANUAL_RESULT`, never provider-confirmed `NOT_ALLOTTED`.
- Review: `docs/reviews/GATE_4A_INDEPENDENT_REVIEW.md` — PASS.

## Verification

- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS
- `cargo test --workspace` — PASS
- `npm run check` on host Node 26 — PASS (129-file secret scan, formatting, TypeScript, 4 Vitest, 4 Python tests)
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

Implement Gate 4D MUFG session/token adapter (next approved slice; see `docs/plans/multi-registrar-allotment/04-slices.md`). Deferred debt carried into later slices: Gate 4B `check_allotment` live transport request construction (task item 10), `provider_reference` population pending verified `Appln_No` semantics; Gate 4C isolated verification-surface wiring and challenge presentation flow (adapters complete, runtime wiring is Gate 4E scope). Real PAN remains blocked.

## Canonical commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run check
npm run build
```
