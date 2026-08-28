# KFintech — current IPO allotment service

**Observed:** 2026-08-28 17:48–18:09 UTC
**Validation level:** Level 1 public service only
**Live PAN submitted:** no
**Result lookup submitted:** no

## Official service

| Item | Current observation |
|---|---|
| Official status URL | <https://ipostatus.kfintech.com> |
| Availability | HTTP 200; React application rendered in headless Chromium |
| Legacy route | <https://kosmic.kfintech.com/ipostatus/> still returns an ASP.NET page that points users to the current URL |
| Provider health | `AVAILABLE` for public page and issue discovery |
| Public issue list | 64 entries embedded in the current public React bundle |
| Example provider issue IDs | `61328680581` — TEMPSENS INSTRUMENTS (INDIA) LIMITED; `54923077460` — SHANKESH JEWELLERS LIMITED |

The issue list is bundled client-side as `{clientId, name}` records. `clientId` is the registrar-specific issue identifier.

## Current lookup mechanics

The rendered page offers:

- PAN;
- Application Number plus PAN;
- Demat Account (NSDL or CDSL).

The public JavaScript calls:

```text
GET https://0uz601ms56.execute-api.ap-south-1.amazonaws.com/prod/api/query?type={pan|appno|dpclid}
headers:
  client_id: <provider issue id>
  reqparam: <lookup value>
```

The application-number mode builds `reqparam` as `<application-number>|<PAN>`. The Demat mode sends the normalized NSDL/CDSL identifier. No request was made to this result endpoint during Gate 2.

## Response contract visible in the public client

Recognized success data is an array under `response.data.data` with fields including:

- `Appln_No`
- `Name`
- `DP_CLID`
- `Pan_No`
- `App_Shares`
- `All_Shares`

The public client displays `Not Allotted` only when a recognized record has numeric `All_Shares == 0`; positive shares display `Allotted`. Sanket must preserve that structured precondition and must not scan arbitrary page text for the phrase.

Observed client error handling:

- HTTP 404 → record not found;
- HTTP 429 → rate limited;
- HTTP 500/502/504 → retry/backoff message;
- network/other failure → generic request problem.

The client declares five attempts with exponential backoff starting at two seconds. Gate 2 did not induce rate limiting.

## CAPTCHA, JavaScript, and session behavior

| Question | Observation |
|---|---|
| CAPTCHA | None present in the current rendered lookup flow |
| OTP | None present in the current rendered lookup flow |
| JavaScript required for official UI | Yes |
| Deterministic HTTP mechanics visible | Yes — API Gateway GET with explicit headers |
| Cookie/session required by current React root | No cookie set on the observed root response |
| Result-API session requirement | No cookie/credential use visible in the current client; not exercised with an identifier |

## Existing adapter assumption review

| Assumption | Classification | Evidence / consequence |
|---|---|---|
| `https://ipostatus.kfintech.com` is the official entry | `VALID` | HTTP 200 and rendered current application |
| Official UI is JavaScript-rendered | `VALID` | React shell plus rendered form |
| Public issue identifiers cannot be discovered | `CHANGED` | 64 `{clientId,name}` entries are embedded in the public bundle |
| No deterministic HTTP contract is visible | `CHANGED` | Current client exposes the API Gateway request contract |
| CAPTCHA may be required on the current path | `CHANGED` | No CAPTCHA/OTP observed in the rendered current flow |
| Arbitrary HTML lines containing `IPO` can discover issues | `CHANGED` | Current shell does not expose issue options that way |
| Any reachable non-challenge page is healthy | `CHANGED` | Health must also verify the bundle, issue list, and expected API contract |
| Unknown content must fail closed | `VALID` | Required and preserved by the domain error mapping |

## Adapter verdict

**MAJOR UPDATE REQUIRED** inside the isolated KFintech adapter.

The current `LiveKfintechProvider` only probes the root page, its issue parser cannot read the current bundle, and its lookup path always returns `UNKNOWN`. The provider abstraction can represent the safe outcomes, but the adapter needs explicit current issue discovery, contract health checks, structured response parsing, and status-code mapping.

## Safe health signal

A conservative public health check can verify, at low frequency:

1. root page returns 200;
2. expected application bundle is available;
3. issue list parses to non-empty validated `{clientId,name}` records;
4. expected API endpoint/type/header contract remains present.

Any contract mismatch becomes `DEGRADED` or `BROKEN`; it never produces `NOT_ALLOTTED`.
