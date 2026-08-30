# MUFG Intime India — current IPO allotment service

**Observed:** 2026-08-28 17:48–18:09 UTC; reverified 2026-08-30 10:56 UTC
**Validation level:** Level 1 public service only
**Live PAN submitted:** no
**Result lookup submitted:** no

## Current branding and official service

| Item | Current observation |
|---|---|
| Current brand | MUFG Intime India Private Limited |
| Investor hub | <https://in.mpms.mufg.com> |
| Public-issues status page | <https://in.mpms.mufg.com/Initial_Offer/public-issues.html> |
| Availability | HTTP 200; `text/html`; 37,489 bytes; rendered in Chromium |
| Provider health | `DEGRADED` until the current contract and guarded transport remediation pass |
| Stale branding residue | Inline footer JavaScript still writes `Link Intime India Pvt Ltd` |

Do not route by the legacy Link Intime name or an old hostname. The canonical provider id remains `mufg_intime` with current MUFG branding and aliases handled as metadata.

## Public issue discovery

The page makes a public, identifier-free request:

```text
POST IPO.aspx/GetDetails
Content-Type: application/json
body: {}
```

The response is a JSON wrapper containing XML tables with `company_id` and `companyname`. Gate 2 received HTTP 200 and four current issues:

| Provider issue ID | Issue |
|---|---|
| `11926` | Symbiotec Pharmalab Limited - IPO |
| `11925` | Augmont Enterprises Limited - IPO |
| `11924` | Gaja Alternative Asset Management Limited - IPO |
| `11923` | Lalithaa Jewellery Mart Limited - IPO |

The 2026-08-30 discovery response was HTTP 200,
`application/json; charset=utf-8`, JSON wrapper key `d`, 545-byte inner XML,
and one `Set-Cookie` response. The isolated in-memory jar contained only the
secure `ASP.NET_SessionId` cookie for `in.mpms.mufg.com`; no cookie value was
printed or persisted. Issue `11926` remained present as exactly
`Symbiotec Pharmalab Limited - IPO`. No investor identifier was sent.

## Current lookup modes

The rendered page offers:

- PAN;
- Application Number;
- DP/Client ID;
- Account Number plus IFSC.

A selected company uses its `company_id` as the registrar-specific issue identifier.

The client obtains a request token through `POST IPO.aspx/generateToken`. On
2026-08-30 the response was HTTP 200,
`application/json; charset=utf-8`, with exactly one non-empty string at JSON
key `d`. The browser applies AES-128-CBC with PKCS#7 padding to that string
using the public page's fixed UTF-8 key and IV, then stores the Base64 result
in `hidToken`. No token or cookie value was printed or persisted.

The status request is:

```text
POST IPO.aspx/SearchOnPan
```

```text
Content-Type: application/json; charset=utf-8
body keys: clientid, PAN, IFSC, CHKVAL, token
```

`PAN` is the public endpoint's generic lookup-value field for every advertised
mode. `CHKVAL` selects PAN (`1`), application number (`2`), DP/client id (`3`),
or account number + IFSC (`4`). `token` is the browser-encrypted token. No
investor request was submitted.

## Result structure visible in the public client

The response is parsed as XML tables inside a JSON wrapper. Recognized result fields include:

- `PEMNDG` / application detail;
- `NAME1`;
- `SHARES`;
- `offer_price`;
- `ALLOT`;
- `AMTADJ`;
- `RFNDAMT`;
- message rows in `Table1`.

Gate 2 did not obtain a real result payload, so exact final-status phrases remain `UNKNOWN`. An adapter must use a sanitized fixture corpus before normalizing `ALLOTTED`, `NOT_ALLOTTED`, `PENDING`, or `NOT_FOUND`.

## CAPTCHA and browser state

The current HTML still contains:

- `CImage.aspx` CAPTCHA image;
- `txtCaptch` input;
- `CaptchaImage.aspx/CheckCaptcha` JavaScript;
- refresh behavior.

The 2026-08-30 live structure is nested:

```html
<div class="input-section paddingcnd" style="display:none;">
  <div class="six columns six-m">
    <input id="txtCaptch" ...>
  </div>
  <div class="three columns four-m">
    <img id="img_cap" src="CImage.aspx">
  </div>
  <div class="three columns two-m">...</div>
</div>
```

In both the raw and rendered current page:

- the CAPTCHA container is `display:none`;
- the client-side CAPTCHA-required checks are commented out;
- computed display for `.input-section.paddingcnd` is `none` and its CAPTCHA
  descendants have zero rendered width/height;
- the CAPTCHA input/image themselves remain `display:inline-block`/`block`, so
  the hidden evidence is specifically the ancestor wrapper;
- `paddingcnd` appears once and no JavaScript selector or rule toggles it;
- the CAPTCHA-required and CAPTCHA-equality checks inside `CALLPANSERCH` are
  commented out;
- `CheckingCaptcha()` and `refreshCaptcha()` remain present but do not make the
  wrapper visible.

A direct image request returned HTTP 200 and set a cookie, but no image was solved or submitted. Therefore the current accurate classification is:

```text
CAPTCHA PRESENT BUT DORMANT/CONDITIONAL
HUMAN VERIFICATION REQUIREMENT: UNKNOWN/CONDITIONAL
```

Sanket must retain the ability to return `NEEDS_HUMAN_VERIFICATION` if the
same authoritative wrapper becomes visible. It must not assume the dormant
state is permanent.

### Exact parser/fixture drift

The old fixture places `style="display:none"` on the direct `<div>` wrapping
`CImage`. `captcha_marker_hidden()` therefore finds the style on the nearest
container. The live page instead places the style on the outer CAPTCHA-section
grandparent while the nearest marker containers have empty styles. The parser
does not walk bounded ancestors and returns `Unknown` despite positive live
hidden evidence. This is a containment-shape change, not evidence that
`Unknown` should generally mean `Dormant`.

The token fixture also models an HTML `<input id="hidToken" value="...">`,
but the live `generateToken` endpoint returns `{"d":"..."}` and JavaScript
performs the encryption. The synthetic request fixture uses non-live field
names (`companyId`, `searchText`, `searchMode`, `requestToken`); the current
public client uses `clientid`, `PAN`, `IFSC`, `CHKVAL`, and `token`.

## JavaScript and session behavior

| Question | Observation |
|---|---|
| JavaScript required for official status flow | Yes |
| Public issue discovery possible by HTTP | Yes |
| PAN lookup advertised | Yes |
| Other lookup keys | Application Number, DP/Client ID, Account Number + IFSC |
| Cookie/token required for result lookup | Yes, based on current token request behavior |
| CAPTCHA currently enforced | Not observed; markup is hidden and validation dormant |
| Deterministic unattended result lookup proven | No real lookup; guarded transport remediation required |
| Rate-limit behavior | `UNKNOWN`; not induced and no explicit current mapping was found |

## Transport Remediation & Root Cause

**Observed 2026-08-30:**

1. **Request order / session state machine**:
   The verified identifier-free sequence is public page → `GetDetails` →
   `generateToken`. `GetDetails` establishes `ASP.NET_SessionId`; the same
   in-memory cookie jar is then used for token generation.
2. **Connection reuse and body framing**:
   The Akamai/IIS edge stalled a POST sent over ureq's recycled HTTP/1.1
   connection. Sending `Connection: close` and an explicit `Content-Length`
   made the exact page → discovery → token sequence complete deterministically
   while retaining cookies in the isolated in-memory jar.

## Adapter verdict

**PUBLIC PRECHECK PASS; REAL LOOKUP NOT AUTHORIZED.**

The MUFG transport remediation is complete and verified. The live precheck passes in ~2.2 seconds without requiring real investor lookup authorization.
