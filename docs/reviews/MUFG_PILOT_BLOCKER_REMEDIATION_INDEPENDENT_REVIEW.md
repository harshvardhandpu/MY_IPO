# MUFG Pilot-Blocker Remediation — Independent Review

- **Date:** 2026-08-30
- **Scope:** Current uncommitted MUFG CAPTCHA-contract and guarded live-transport remediation
- **Reviewer:** `gpt-oss-120b` via generalcompute (owner-locked, free, independent route)
- **Mode:** READ-ONLY complete-patch review; no repository edits, PAN access, CAPTCHA submission, or investor-result request
- **Provenance:** HTTP 200; served model `gpt-oss-120b`; prompt tokens 13,945; completion tokens 298; reasoning tokens 107; stop reason `stop`

## Verdict

**PASS — APPROVED — BLOCKING FINDINGS: NONE**

The reviewer returned `approve: true`, with no blocking or non-blocking findings.

## Confirmed invariants

- Current CAPTCHA is `DORMANT` only from positive hidden-ancestor evidence; ambiguity remains `UNKNOWN`.
- Native HTTPS transport keeps an exact MUFG host allowlist and disables redirects for sensitive requests.
- `REAL_INVESTOR_LOOKUP_AUTHORIZED` remains `false`; transport implementation does not authorize PAN lookup.
- Cookies and tokens remain in memory and redacted.
- TLS verification, response bounds, timeouts, and rate limiting remain enforced.
- Issue mapping remains generic; issue `11926` is not a parser special case.
- Positive and negative financial results remain behind guarded proof constructors.
- Sanitized fixtures retain provenance and SHA-256 verification.

## Verification evidence reviewed

All commands exited zero:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `npm run check`
- `npm run build`
- `npm run secrets` — 161 files
- `git diff --check`

The explicit ignored live precheck also passed:

- MUFG contract: PASS
- Issue `11926`: AVAILABLE
- CAPTCHA: `Dormant`
- Session bootstrap: PASS
- Request token: PASS
- Real PAN lookup: NOT EXECUTED

The precheck used only the public page, issue discovery, and session/token bootstrap. It did not call `SearchOnPan`.
