# KFintech (KFin Technologies) — IPO allotment status research

**Date tested:** 2026-08-28 (Phase 3B re-verify)
**Live PAN used:** none

## Official entry points

| Item | Value |
|---|---|
| IPO allotment status | https://ipostatus.kfintech.com |
| How-to article | https://www.kfintech.com/how-to-check-your-ipo-allotment-status-key-factors-you-need-to-know |
| Corporate registry hub | https://ris.kfintech.com |

## Observed workflow (public)

1. Open IPO allotment status portal.
2. Select IPO/issue.
3. Submit PAN (primary investor lookup).
4. Read allotted / not allotted / pending messaging.

## Automation characteristics (Phase 3B)

| Question | Finding |
|---|---|
| Stable pure-HTTP API documented? | **No** |
| JS / SPA portal? | **Yes** — machine-readable allotment without browser session is not reliable |
| CAPTCHA possible? | **Yes** — treat as `NEEDS_HUMAN_VERIFICATION` |
| Selected Sanket path | **A→C**: try reachability/discovery GET; if no deterministic form POST, fail closed to UNKNOWN or human verification — never invent NOT_ALLOTTED |
| Browser worker | Deferred until a stable DOM contract is proven; fixture path remains CI default |

## Adapter strategy

- `kfintech-fixture` — deterministic CI/local synthetic
- `kfintech-live` — live adapter with offline/human-gate test modes; production network path probes portal and fails closed
- Manual fallback + open official URL always available

## Rate policy (default)

- min interval 1.5s between provider calls
- max 5 attempts with capped exponential backoff

## Notes

Third-party IPO calendars still list many issues under KFintech for registrar discovery metadata only.
