# KFintech (KFin Technologies) — IPO allotment status research

**Date tested:** 2026-08-28  
**Researcher:** Sanket Phase 3 agent  
**Live PAN used:** none (public pages / third-party indexes only)

## Official entry points

| Item | Value |
|---|---|
| Corporate registry hub | https://ris.kfintech.com |
| IPO allotment status | https://ipostatus.kfintech.com |
| Corporate site | https://www.kfintech.com |

## Observed workflow (public)

1. Investor opens IPO allotment status portal.
2. Selects the IPO/issue from a list (issue-selection mechanism).
3. Submits a lookup identifier — **PAN is the common investor key** for IPO allotment checks on Indian RTAs.
4. Portal returns allotted / not allotted / pending-style messaging for that application.

## Lookup identifiers

| Key | Supported (public expectation) |
|---|---|
| PAN | Yes (primary for IPO status) |
| Application number | Sometimes (issue-dependent; not assumed) |
| DP ID / Client ID | Possible on some flows; not confirmed without live issue |

## Automation characteristics (current)

| Question | Finding |
|---|---|
| Stable pure-HTTP API documented publicly? | **Not found** on public pages |
| JS / browser UI required? | **Likely yes** — SPA-style status portals historically |
| CAPTCHA? | **Possible** (site protection varies; treat as NEEDS_HUMAN_VERIFICATION if present) |
| OTP? | Not the default for allotment status; treat as human path if seen |
| Rate limits? | Unknown; assume conservative sequential checks |
| Official HTML structure stable? | **Unknown** — fail closed to `UNKNOWN` on parse mismatch |

## Adapter strategy for Sanket

1. **Fixture-first provider** (`kfintech-fixture`) for CI and local synthetic jobs.
2. Live HTTP/form spike only when a current issue is available; prefer deterministic request/response if discovered.
3. Browser DOM worker only if required, isolated profile, domain allowlist `*.kfintech.com`, no focus steal.
4. Never bypass CAPTCHA/OTP; pause job as `NEEDS_HUMAN_VERIFICATION`.
5. Unknown/malformed response → `UNKNOWN` or operational error — **never** `NOT_ALLOTTED`.

## Fallback

- Open official status URL in system browser (no embedded PAN).
- Manual result entry with `source = MANUAL` provenance.

## Notes

- `ipostatus.kfintech.com` did not return extractable content to the research agent on 2026-08-28 (fetch failed / empty). Re-verify before live adapter lock-in.
- Third-party IPO calendars (e.g. IPO Watch) list many current issues under KFintech — useful for registrar discovery metadata, not for authoritative allotment.
