# Gate 4F Pre-Pilot Corrections — Independent Review Record

- **Date**: 2026-08-30
- **Scope**: Commit `75af0ce` "fix(allotment): close gate 4f pre-pilot conditions A-E" (parent `8353af4`)
- **Reviewer**: gpt-oss-120b via generalcompute (owner-locked route; free, independent of the implementation model)
- **Mode**: READ-ONLY evidence-pack review (diffs + full new-file contents + host test evidence); no repository access, no edits, no PAN in pack (by policy)
- **Evidence**: diff_stat (11 files, +844/−47), diff_A (http.rs + kfintech_live.rs), diff_B_C (runtime.rs + service.rs), diff_E (registry.rs), diff_D_tests (lease/URL proofs + provider_contract additions); host-run test output 2026-08-30
- **Provenance** (from response metadata): served model `gpt-oss-120b`; HTTP 200; prompt_tokens 12676, completion_tokens 325; stop_reason stop

## Verdict

**PASS**

## Conditions

| Cond | Requirement | Result |
|------|-------------|--------|
| A | Remove KFintech curl shell-out → bounded native HTTP client | **PASS** — ureq+rustls; TLS, host allowlist, redirect limits, size cap, timeouts, UA, content-type check, sanitized errors, no body logging, no fallback |
| B | Per-provider rate limiting wired into execution | **PASS** — per-provider policy map; set_policy per registrar; policy_for consulted in wait_turn |
| C | No KFintech URL default; fail closed | **PASS** — missing/empty official_status_url → ServiceError::Invalid; no default applied |
| D | JobLease enforcement proven at boundary | **PASS** — single-owner, stale-lease recovery, COMPLETE/CANCELLED never re-executed; exercised via run_allotment_job_once |
| E | Unified registry alias resolution | **PASS** — one canonical normalization (trim, lowercase, spaces→underscores) shared by both resolvers; unknown → None |

**Regressions**: none.

## Reviewer notes (verbatim summary)

- No curl or std::process::Command usage remains.
- Per-provider rate policies are correctly wired and distinct.
- URL handling now fails closed with explicit error messages.
- Lease enforcement is proven at the execution boundary with comprehensive tests.
- Registry alias resolution is unified and rejects unknown aliases.

## Verification evidence (host, 2026-08-30)

- `cargo fmt --all -- --check` clean
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `cargo test --workspace` all green (~186 tests) incl. new: allotment_security 2/2, allotment_lease_and_url 6/6, provider_contract 13/13, http policy 3/3
- `npm run check` green (secrets scan 156 files, biome, tsc, vitest 4/4)
- `npm run build` green
- `git diff --check` clean

## Disposition

The five Gate 4F pre-pilot conditions are closed and independently verified.
Real-PAN pilot remains BLOCKED pending: exact-IPO identification with the owner,
registrar/issue availability verification, secure identity mode verification
(Linux Secret Service ACTIVE, production sensitive mode, dev/device-derived key
fallback disallowed), and separate explicit owner authorization for the real
PAN lookup.
