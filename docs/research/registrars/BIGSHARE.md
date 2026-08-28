# Bigshare Services — current IPO allotment service

**Observed:** 2026-08-28 17:48–18:09 UTC
**Validation level:** Level 1 public service only
**Live PAN submitted:** no
**CAPTCHA solved or bypassed:** no

## Official service

| Item | Current observation |
|---|---|
| Official landing page | <https://www.bigshareonline.com/ipo_Allotment.html> |
| Server 1 | <https://ipo.bigshareonline.com/ipo_status.html> |
| Server 2 | <https://ipo1.bigshareonline.com/ipo_status.html> |
| Server 3 | <https://ipo2.bigshareonline.com/ipo_status.html> |
| Final availability check | All three status servers returned HTTP 200 |
| Transient failure observed | Server 2 returned HTTP 503 during an earlier probe, then recovered |
| Provider health | `HUMAN_VERIFICATION_REQUIRED`; public service available with live drift observed |

## Public issue discovery

The final current server pages expose four registrar issue IDs directly in `ddlCompany`.

| Provider issue ID | Issue |
|---|---|
| `9046` | HY-TECH ENGINEERS LIMITED |
| `9045` | SKYWAYS AIR SERVICES LIMITED |
| `590` | ABH HEALTHCARE LIMITED |
| `9044` | SUNSHINE PICTURES LIMITED |

The issue list expanded during the observation window alongside the CAPTCHA deployment. The final host-rendered page is the source of truth above.

## Current lookup modes

- Application Number / CAF Number (`AP`)
- Beneficiary ID (`BN`), with NSDL/CDSL detail
- PAN Number (`PN`)

The status request is a same-origin JSON POST to:

```text
Data.aspx/FetchIpodetails
```

The current payload includes issue, lookup mode/value, language, a CAPTCHA token, the human-entered CAPTCHA answer, and an optional signed result token used only to re-render a previously verified result.

## Human verification

CAPTCHA is mandatory for a new search.

The final observed page obtains a challenge from:

```text
GET Captcha.ashx
```

The response contained only a challenge token and an image data URL. No cookie was set by that request. Sanket did not inspect the image contents, solve it, submit an answer, or call the result endpoint.

Correct application capability:

```text
NEEDS_HUMAN_VERIFICATION
```

A legitimate continuation must open the official Bigshare page in an isolated, ephemeral registrar-only browser context or let the member open the official page and enter a manual result. No CAPTCHA solving or bypass belongs in the adapter.

## Live drift observed during Gate 2

The first direct response during the observation window contained an older client-side canvas CAPTCHA implementation. A later response from the same host served a new server-verified token/image CAPTCHA flow with explicit rate-limit states. The final recheck from both host and container returned the new page.

This is direct evidence that the adapter contract can drift during operation. Bigshare health must validate the expected challenge and response contract rather than trust only HTTP 200.

## Current response and failure states visible in the public client

The current `data.d` object includes an explicit `Status` field and result fields such as:

- `APPLICATION_NO`
- `DPID`
- `Name`
- `APPLIED`
- `ALLOTED`
- `MatchCount`
- `Records`

Recognized operational statuses include:

- `OK`
- `NOTFOUND`
- `CAPTCHA`
- `RATELIMIT`
- `WARMING`

HTTP 429 and 503 are handled with `Retry-After` where supplied. The page distinguishes no record, invalid input, CAPTCHA failure, rate limiting, warming, and successful result data. Gate 2 did not submit an investor identifier or induce throttling.

`NOTFOUND` must normalize to `NOT_FOUND`, not `NOT_ALLOTTED`. `ALLOTED` may be interpreted only inside an explicit successful structured response.

## JavaScript and session behavior

| Question | Observation |
|---|---|
| JavaScript required | Yes for challenge acquisition, form validation, and result rendering |
| Browser required for legitimate automated continuation | Yes, because a human must solve the displayed challenge |
| Deterministic public issue discovery | Yes, static select options |
| Result HTTP mechanics visible | Yes, but gated by server-verified human CAPTCHA |
| Cookie/session required | No cookie observed for page or `Captcha.ashx`; signed challenge token carries state |
| Rate-limit behavior | Explicit 429/503 handling; one transient 503 observed on Server 2 |

## Adapter verdict

**NOT IMPLEMENTED / BLOCKED BY HUMAN VERIFICATION for unattended lookup.**

The existing provider abstraction can return `NeedsHumanVerification`, `RateLimited`, `Unavailable`, and `Unknown` safely. A Bigshare adapter still needs public issue discovery, capabilities, contract health, and an explicit human-continuation handoff. It must not call the status endpoint without the legitimate human challenge step.
