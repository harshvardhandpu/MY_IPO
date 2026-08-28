# Bigshare Services — IPO allotment status research

**Date tested:** 2026-08-28  
**Live PAN used:** none

## Official entry points

| Item | Value |
|---|---|
| Home | https://www.bigshareonline.com |
| IPO allotment status | https://www.bigshareonline.com/ipo_Allotment.html |
| Notes | Multiple server endpoints (Server 1/2/3) advertised on allotment page |

## Observed workflow (public)

1. Open allotment status page.
2. Choose server / issue as presented by the site.
3. Submit investor lookup (PAN commonly required).
4. Read allotment outcome.

## Automation characteristics

| Question | Finding |
|---|---|
| Stable public HTTP API? | **Not documented** |
| Browser/JS likely? | **Yes** |
| Multi-server failover? | Yes (explicit Server 1/2/3) |
| CAPTCHA / anti-bot? | Possible — fail to human verification |
| Direct fetch from research sandbox | **Blocked** (private/internal network classification on one probe) |

## Sanket strategy

- Stub provider id `bigshare` with health `UNKNOWN` until spike complete.
- Prefer fixture adapter before any live calls.
- Domain allowlist: `*.bigshareonline.com` if browser worker is required.
- Unknown parse → `UNKNOWN`, never `NOT_ALLOTTED`.

## Fallback

Open official page + manual result with provenance.
