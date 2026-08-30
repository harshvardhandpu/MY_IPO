# Gate 4E Independent Review

- **Reviewer:** `gpt-oss-120b`
- **Provider:** generalcompute (free tier; zero paid usage)
- **Review date:** 2026-08-29
- **Implementation scope:** Gate 4E cross-provider normalization + unified allotment UI, commit `1a57379` on `feature/multi-registrar`, git range `744a602..1a57379` (19 files, +1224/−281)
- **Mode:** independent, read-only, fail-closed review
- **Route evidence:** runner configured `gpt-oss-120b` via generalcompute (`https://api.generalcompute.com/v1`); response metadata reported served model `gpt-oss-120b`; usage 20008 prompt / 882 completion tokens
- **Implementer model:** DeepSeek V4 Pro (NVIDIA NIM) — reviewer is a different model/provider
- **Policy:** **GEMINI = DISABLED FOR SANKET IPO BY OWNER POLICY**; no real PAN, no live investor lookup, no CAPTCHA bypass

## Verdict

**PASS**

**Recommendation:** APPROVE — zero blocking findings.

## Blocking Findings

None.

## High Severity

None.

## Medium Severity

None.

## Low / Non-Blocking

The reviewer reported two non-blocking notes, both of which are evidence-pack artifacts rather than code defects (verified post-review by the coordinator):

1. "Duplicate definition of `ProviderDescriptor` in `registry.rs`" — **not a defect.** `ProviderDescriptor` is defined exactly once (in `crates/allotment/src/registry.rs`). The reviewer saw the same struct twice because the evidence pack included `registry.rs` both verbatim and inside the diff.
2. "UI references a removed provider selector" — **not a defect.** The provider-mode selector was intentionally removed; `App.flows.test.tsx` explicitly asserts `queryByLabelText("Provider mode").not.toBeInTheDocument()` and the UI/test handle its absence correctly.

## Security Assessment

- PAN: no plaintext PAN persisted, logged, or exposed; access strictly purpose-scoped via `SensitiveIdentityRecord` / `with_pan(..., AllotmentCheck, ...)`.
- Human verification: unified `NEEDS_HUMAN_VERIFICATION` isolates CAPTCHA/OTP; no bypass; challenge state is safe metadata only.
- Provider failure isolation: one registrar failing does not abort the whole job; partial results retained.
- Manual provenance: source/provenance fields distinguish manual from automated.
- Tauri surface: least-privilege; no generic fs/shell/network escape hatch.

## Financial / Normalization Assessment

- One application-level result shape across KFintech/Bigshare/MUFG; provider details stripped before frontend DTOs.
- `NOT_ALLOTTED` gated on provider-confirmed `NegativeResultProof` only; UNKNOWN/NOT_FOUND/CAPTCHA/timeout/429/parser drift never manufacture a financial negative.
- Estimated profit (integer paise) kept distinct from realized; explicit basis/provenance.

## Restart / Recovery Assessment

- Job/provider/registrar state persisted and authoritative on restart; `execute_allotment_check` resolves from the persisted job, not repeated caller fields.
- Cancelled jobs and finalized attempts survive restart; duplicate execution prevented via lease + status checks.

## UI / Tauri Assessment

- UI reflects provider health, verification requirements, and mixed-account progress; UNKNOWN distinct from NOT_ALLOTTED; no PAN/sensitive data in UI.

## Test Coverage Assessment

- Covers PAN non-persistence, unknown-registrar fail-closed, cancellation, recovery, human-verification, cross-provider normalization, and dev-mode enqueue of live adapters.

## Verification evidence supplied (not rerun)

- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS
- `cargo test --workspace` — PASS
- `npm run check` — PASS (152-file secret scan, typecheck, 5 Vitest + 4 Python)
- `npm run build` — PASS
- `git diff --check` — PASS

## Disposition

Gate 4E is CLOSED. Implementation locked at commit `1a57379`. No real PAN, investor lookup, CAPTCHA solution, or CAPTCHA bypass was used. Next approved slice: Gate 4F (not started — blocked pending Gate 4E formal closure, which is now complete).