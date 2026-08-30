# Gate 4F Independent Review — Final Multi-Registrar System Review

- **Reviewer:** `moonshotai/kimi-k3`
- **Provider:** NVIDIA NIM (free route; zero paid usage)
- **Review date:** 2026-08-29
- **Scope:** Complete integrated registrar subsystem, Gates 4A–4E, git range `d2bf075..1a57379` (49 files, +5301/−339), culminating in commit `1a57379`
- **Mode:** independent, read-only, fail-closed final system review
- **Route evidence:** runner configured `moonshotai/kimi-k3` via NVIDIA NIM (`https://integrate.api.nvidia.com/v1`); response metadata reported served model `moonshotai/kimi-k3`; usage 26951 prompt / 2065 completion tokens
- **Implementer models:** DeepSeek V4 Pro (4E + majority integration), GLM 5.3 (4C), plus earlier routes — reviewer is independent of all implementation routes
- **Policy:** **GEMINI = DISABLED FOR SANKET IPO BY OWNER POLICY**; no real PAN, no live investor lookup, no CAPTCHA bypass

## Verdict

**PASS WITH NON-BLOCKING FINDINGS**

## Pilot readiness

**YES — WITH NON-BLOCKING CONDITIONS**

The integrated subsystem is coherent and ready to ENTER the controlled real-PAN pilot gate. This approval does **not** itself authorize any real PAN lookup — real PAN remains blocked.

## Blocking Findings

None.

## High Severity (non-blocking, pre-pilot-execution condition)

1. `LiveKfintechProvider::http_get_text` shells out to `curl` via `std::process::Command` (`crates/allotment/src/kfintech_live.rs:190`). Not wired into the service execution path (service uses `KfintechProvider`, whose `check_allotment` performs no network I/O and fails closed); never carries PAN; GET of a public page only. Latent least-privilege violation — must be removed/replaced with a sanctioned HTTP client before any production transport slice. Already flagged in-code as a `ponytail:` simplification.

## Medium Severity

1. **Rate-limiter wiring** — `service.rs` initializes one global `ProviderRateLimiter::new(Default::default())` instead of selecting per-provider `ProviderRatePolicy::for_provider(...)`. Per-provider policies exist and are tested, but are not selected at the orchestration layer. Correct before real traffic; harmless in dev.
2. **Registry alias asymmetry** — `resolve` and `resolve_registrar` accept different alias sets (`"link intime"`/`"linkintime"` in `resolve` but not `resolve_registrar`; `"kfin technologies"` vs `"kfin_technologies"`). Both fail closed, so not an escape hatch, but the two surfaces should be unified to prevent drift.
3. **Cross-registrar URL default** — `execute_allotment_check` falls back to a hardcoded KFintech URL when `official_status_url` is absent on the persisted job, regardless of actual registrar. Should be registrar-derived or absent, never cross-registrar.
4. **Lease enforcement evidence** — `JobLease` is defined and tested, but the orchestration excerpt does not demonstrate lease acquisition/checking around `execute_allotment_check`. Confirmed in tests but confirm at execution boundary before pilot.

## Low / Non-Blocking

- `supports()` uses substring matching (`contains("kfin")` etc.); routing authority is the registry's exact-match `resolve_registrar`, so not a routing guess, but unnecessary surface area.
- `NormalizedAllotmentStatus::from_provider_text` is heuristic substring classification; all three live adapters use structured parsers with fingerprints, so it's fallback-only. Ordering (NOT ALLOT before ALLOT) correct.
- Fixture provider keys synthetic outcomes off last PAN character — explicitly synthetic-only, gated to DEVELOPMENT_SYNTHETIC, rejected in production.
- Schema excerpt duplicated `registrar_id` lines — excerpt artifact (migrations v1–v6 apply cleanly, tests pass).

## Security Assessment

Sound: `check_allotment` receives `&Pan` as temporary arg, no adapter stores it; `reject_embedded_pan` guards all job-persisted string fields; MUFG session/token memory-only with redacted Debug; lookup-request DTO non-serde; challenge/continuation metadata constrained to safe identifiers. Mode gating correct. The curl shell-out is the one latent item (High #1).

## Financial / Normalization Assessment

Strong: `NOT_ALLOTTED` only via five-fact `NegativeResultProof`; `ALLOTTED` only via `PositiveResultProof` + positive share count; `NOT_FOUND`/`UNKNOWN`/`PENDING`/`RATE_LIMITED`/`NEEDS_HUMAN_VERIFICATION` structurally distinct and refuse financial statuses by construction. Money integer paise with saturating arithmetic; estimated profit carries explicit basis/provenance + unavailable state. Manual results source-tagged, cannot masquerade. Friend 10% share untouched.

## Restart / Recovery Assessment

Legal-transition enforcement; final statuses clear retry; partial completion preserves finals; cancellation from all non-terminal states; ephemeral session state in-memory only, not reused after restart. Lease-based duplicate-execution prevention designed; enforcement-at-boundary not demonstrated in excerpt (Medium #4).

## UI / Tauri Assessment

Unified report card consumes only normalized status enum + safe fields; UNKNOWN distinct from NOT_ALLOTTED end-to-end; no PAN/raw HTML/challenge content in DTOs; no generic fs/shell/network Tauri escape hatch (curl concern is Rust-side).

## Test Coverage Assessment

~290 workspace tests + JS/Python checks; fmt/clippy/clean-diff pass. Gaps: integration test for per-provider rate-policy selection at service layer, and lease-contention test at execution boundary.

## Cross-Gate Coherence

4A–4E compose coherently; shared runtime types consumed unchanged by all three adapters; per-gate commitments survive integration; 4E registry authority + dev-enqueue matches service code.

## Final Recommendation

**APPROVE** — the integrated subsystem is coherent, fail-closed, and ready to ENTER the controlled real-PAN pilot gate. Pilot readiness: YES — WITH NON-BLOCKING CONDITIONS. Conditions to close before pilot *execution* (not gate entry): (1) remove/replace curl shell-out, (2) wire per-provider rate policies into service limiter, (3) demonstrate lease enforcement at execution, (4) unify registry alias sets, (5) fix cross-registrar URL default.

## Disposition

Gate 4F (final multi-registrar system review) is COMPLETE: PASS WITH NON-BLOCKING FINDINGS; pilot readiness YES — WITH NON-BLOCKING CONDITIONS. Real PAN remains BLOCKED; no investor-result request or CAPTCHA bypass was performed. The five pre-pilot conditions are carried into the controlled-pilot-gate checklist.