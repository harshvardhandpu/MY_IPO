# Controlled Real-PAN Pilot — Preparation Record

- **Prepared:** 2026-08-30T10:44:55Z
- **Scope:** public issue/provider validation and local security preflight only
- **Investor request submitted:** **NO**
- **PAN accessed or recorded:** **NO**

## Decision

| Item | Result |
|---|---|
| PRE-PILOT HARDENING | **PASS — REMEDIATION REVIEWED** |
| KNOWN APPLICATION | **CONFIRMED BY OWNER** |
| EXPECTED RESULT | **ALLOTTED** |
| PROVIDER | **MUFG Intime India Private Limited** (formerly Link Intime India Private Limited) |
| ISSUE AVAILABLE | **YES** |
| SECURE KEYRING | **PASS** |
| REAL-PAN EXECUTION | **NOT AUTHORIZED — explicit owner YES still required** |

The public-contract and transport blockers are closed. The current CAPTCHA
wrapper is classified `Dormant` only from positive hidden-ancestor evidence;
ambiguous structures still fail closed as `Unknown`. Live transport is
implemented, but real investor lookup remains independently unauthorized.

## Exact issue mapping

- **IPO:** Symbiotec Pharmalab Limited
- **Official registrar:** MUFG Intime India Private Limited (formerly Link Intime India Private Limited)
- **Provider issue id:** `11926`
- **Provider issue label:** `Symbiotec Pharmalab Limited - IPO`
- **Official allotment service:** <https://in.mpms.mufg.com/Initial_Offer/public-issues.html>
- **Expected known result:** `ALLOTTED`

### Public-source evidence

1. SEBI's official Red Herring Prospectus filing identifies MUFG Intime India
   Private Limited as registrar:
   <https://www.sebi.gov.in/filings/public-issues/aug-2026/symbiotec-pharmalab-limited-rhp_103750.html>
2. Identifier-free `POST {}` to MUFG's official `IPO.aspx/GetDetails` endpoint
   returned HTTP 200 and exactly one case-insensitive issue match: id `11926`,
   label `Symbiotec Pharmalab Limited - IPO`.
3. No PAN, account id, application number, DP id, or investor identifier was
   sent during this discovery.

## Provider precheck — no PAN

Repeated 2026-08-30T17:04:52Z against the official MUFG service.

| Check | Result |
|---|---|
| Bootstrap page | HTTP 200 |
| Issue discovery contract | JSON-wrapped XML markers present and parsed |
| Exact issue exposed | YES — one match, id `11926` |
| Session/token bootstrap | HTTP 200; non-empty token and session cookie observed in memory only, then discarded |
| Search contract marker | `SearchOnPan` present |
| OTP marker | Not observed |
| CAPTCHA markup | Present |
| Sanket CAPTCHA classification | **`Dormant` — positive hidden-wrapper evidence** |
| Provider runtime capability | **Transport implemented; real lookup authorization remains false** |
| Provider health | **ACCEPTABLE FOR A SEPARATELY AUTHORIZED ONE-shot pilot** |

An HTTP 200 alone is not treated as provider health. The precheck verified the
current structure, exact issue, isolated session cookie, JSON token contract,
and dormant CAPTCHA evidence. No investor identifier or result request was
used. A visible or ambiguous challenge still stops the flow.

## Security preflight

| Control | Result |
|---|---|
| Linux Secret Service generated synthetic key write/read/delete | PASS |
| Generated key read-back equality | PASS |
| Generated key deletion and absence | PASS |
| `PRODUCTION_SECURE` application status | PASS |
| Production key provider | `os-keyring` |
| Production mode blocker | None |
| Production mode + in-memory/dev provider | Rejected fail-closed |
| Device-derived/development fallback in production | Disallowed |
| Real PAN used in preflight | NO |

Required invariant remains enforced:

`REAL_SENSITIVE_MODE + NO_SECURE_OS_KEY_PROVIDER = FAIL CLOSED`

## Gate 4F pre-pilot conditions

| Condition | Status |
|---|---|
| A — no curl/shell registrar transport | PASS after follow-up `6b8ff6a`; public ureq `rustls` feature + webpki roots; local HTTPS no-panic regression |
| B — provider-specific rate limiting | PASS |
| C — no KFintech URL fallback | PASS |
| D — execution-boundary `JobLease` proof | PASS |
| E — unified registry aliases | PASS |

A public live precheck found that the original internal ureq `_tls` feature did
not activate HTTPS and panicked before transport. Commit `6b8ff6a` replaced it
with the public `rustls` feature and added a local regression. Full cargo/npm
verification passed, and the owner-locked independent reviewer
`gpt-oss-120b` via generalcompute returned PASS with no blockers. See
`docs/reviews/GATE_4F_TLS_AMENDMENT_INDEPENDENT_REVIEW.md`.

## Exact execution checklist — remediation complete; execution not authorized

### Blocker-remediation gate

- [x] Capture the current public MUFG structure without investor identifiers.
- [x] Explain why the tested parser returned CAPTCHA `Unknown`.
- [x] Update the provider contract only from observed, sanitized evidence; do not guess visibility.
- [x] Preserve legitimate CAPTCHA as `NEEDS_HUMAN_VERIFICATION`; no bypass.
- [x] Implement and test the MUFG live investor transport behind the existing authorization gate.
- [x] Repeat identifier-free issue/session/CAPTCHA precheck.
- [x] Require acceptable provider health and exact issue id `11926`.
- [x] Run full security, cargo, npm, secrets, and independent-review gates.

### Owner/runtime gate after all blockers are closed

- [ ] Confirm the owner account has a SensitiveIdentityService identity reference.
- [ ] If absent, use only the native secure onboarding/edit flow; never chat, logs, shell history, source, fixtures, env files, or docs.
- [ ] Confirm `PRODUCTION_SECURE`, `os-keyring`, and no security blocker.
- [ ] Ask separately: **"Run the controlled live lookup now?"**
- [ ] Require an explicit owner **YES** for exactly one lookup.
- [ ] Resolve PAN only through `SensitiveIdentityService.with_pan(account_id, AllotmentCheck, ...)`.
- [ ] Submit exactly one request to MUFG issue id `11926`; no other registrar request.
- [ ] Treat `ALLOTTED` as expected; stop on every other financial or operational outcome.
- [ ] Immediately perform the post-lookup PAN/session/provenance audit required by the pilot policy.

## Stop conditions

Do not access PAN or call the result endpoint without a separate explicit owner
YES. Do not reinterpret `Unknown`, `NotFound`, `ProviderUnavailable`, or
`NeedsHumanVerification` as a financial result. Do not bypass CAPTCHA or OTP.
