# Gate 4C Independent Review

- **Reviewer:** gpt-oss-120b
- **Provider:** generalcompute (free tier; no paid usage)
- **Review date:** 2026-08-29
- **Scope:** Gate 4C Bigshare human-verification adapter, git range `75fbf9a..cbbe285` on `feature/multi-registrar` — `BigshareProvider` (fail-closed unattended check, fixture-backed `ddlCompany` discovery and ASP.NET `{"d":{...}}` result parsing), guarded `not_found`/`operational()` result constructors, `HumanVerificationChallenge` lifecycle state machine, 29 focused tests, sanitized fixtures with pinned SHA-256 provenance
- **Mode:** independent, read-only, fail-closed review
- **Implementer model:** GLM 5.3 (TokenRouter) — reviewer is a different model/provider; same healthy free route that reviewed Gate 4B. Gemini barred by owner policy.

## Verdict

**PASS**

**Recommendation:** APPROVE (`approve: true`, zero blocking findings)

## Findings

### Non-blocking

1. **`option_value`/`option_label` are simple string scanners rather than a full HTML parser** (`crates/allotment/src/bigshare.rs`). Sufficient for the controlled fixture format.

   *Disposition: accepted, deliberate.* The Gate 3 design mandates bounded, non-expanding parsers for provider content; these helpers recognize only the static `<option value="digits">Label</option>` shape and fail closed on everything else. A general HTML parser would add an attack surface, not safety. Drift beyond this shape correctly degrades to `Unknown`.

2. **`is_expired` uses lexical RFC 3339 string comparison rather than a datetime library** (`crates/allotment/src/provider.rs`). Safe for well-formed timestamps; a dedicated datetime library would be more future-proof.

   *Disposition: accepted, deliberate.* The provenance/timestamp contract guarantees fixed-shape UTC `YYYY-MM-DDTHH:MM:SSZ` values, under which lexical order equals chronological order. The method fails safe (anything at/past expiry reads expired). Marked with a `ponytail:` ceiling comment at the definition.

## What the reviewer confirmed

Per the prompt contract: unattended `check_allotment` always fails closed to `NEEDS_HUMAN_VERIFICATION`; `NOT_ALLOTTED` only via `NegativeResultProof` on a validated OK record; `NOTFOUND` → `NOT_FOUND`; `ALLOTED > 0` only via `PositiveResultProof`, `null` → `Pending`; no PAN/member-field passthrough; guarded `operational()` cannot mint financial statuses; challenge lifecycle transitions are legal and terminal states final; discovery fails closed on placeholder/duplicate/non-digit ids; fixture SHA-256 provenance verified at test time; no CAPTCHA automation anywhere.

## Verification evidence supplied

- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS
- `cargo test --workspace` — PASS (29 Bigshare + 13 KFintech + all suites green)
- `npm run check` — PASS (biome, typecheck, vitest 4, python 4)
- `npm run build` — PASS (212ms, dist produced)
- `npm run secrets` — PASS (143 files)
- `git diff --check` — clean
- Implementation commit: `cbbe285`

## Disposition

Gate 4C is APPROVED. Both non-blocking findings are deliberate bounded-parser simplifications with documented ceilings. Real PAN remains blocked pending separately authorized controlled pilot. Next approved slice: Gate 4D — MUFG session/token adapter.
