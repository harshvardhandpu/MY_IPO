# MUFG Intime India — current IPO allotment service

**Observed:** 2026-08-28 17:48–18:09 UTC
**Validation level:** Level 1 public service only
**Live PAN submitted:** no
**Result lookup submitted:** no

## Current branding and official service

| Item | Current observation |
|---|---|
| Current brand | MUFG Intime India Private Limited |
| Investor hub | <https://in.mpms.mufg.com> |
| Public-issues status page | <https://in.mpms.mufg.com/Initial_Offer/public-issues.html> |
| Availability | HTTP 200; page rendered in headless Chromium |
| Provider health | `DEGRADED` for automated lookup confidence; public discovery is available |
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

This discovery call required no PAN and set no cookie in the observed request.

## Current lookup modes

The rendered page offers:

- PAN;
- Application Number;
- DP/Client ID;
- Account Number plus IFSC.

A selected company uses its `company_id` as the registrar-specific issue identifier.

The client obtains a request token through `POST IPO.aspx/generateToken`, stores an encrypted form in `hidToken`, and submits the status request to:

```text
POST IPO.aspx/SearchOnPan
```

The payload includes `clientid`, lookup value, optional IFSC, lookup-mode number, and the encrypted token. The token request returned HTTP 200 and set a cookie. No token value was retained in research documentation, and no investor lookup was performed.

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

However, in both the raw and rendered current page:

- the CAPTCHA container is `display:none`;
- the client-side CAPTCHA-required checks are commented out;
- no script was observed making that container visible.

A direct image request returned HTTP 200 and set a cookie, but no image was solved or submitted. Therefore the current accurate classification is:

```text
CAPTCHA PRESENT BUT DORMANT/CONDITIONAL
HUMAN VERIFICATION REQUIREMENT: UNKNOWN/CONDITIONAL
```

Sanket must retain the ability to return `NEEDS_HUMAN_VERIFICATION` if the challenge becomes active. It must not assume the dormant state is permanent.

## JavaScript and session behavior

| Question | Observation |
|---|---|
| JavaScript required for official status flow | Yes |
| Public issue discovery possible by HTTP | Yes |
| PAN lookup advertised | Yes |
| Other lookup keys | Application Number, DP/Client ID, Account Number + IFSC |
| Cookie/token required for result lookup | Yes, based on current token request behavior |
| CAPTCHA currently enforced | Not observed; markup is hidden and validation dormant |
| Deterministic unattended result lookup proven | No |
| Rate-limit behavior | `UNKNOWN`; not induced and no explicit current mapping was found |

## Adapter verdict

**NOT IMPLEMENTED.**

The existing provider abstraction can safely represent unavailable, retryable, human-verification, and unknown states. A MUFG adapter needs identifier-free discovery, browser/session capabilities, token lifecycle isolation, structured XML-result fixtures, and fail-closed parsing. Until those exist, manual fallback remains the only supported Sanket path.
