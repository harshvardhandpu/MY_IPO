# Gate 2 — Live Registrar Behavior + Capability Validation

**Observed:** 2026-08-28 17:48–18:09 UTC
**Branch:** `feature/multi-registrar`
**Gate 1 commit:** `b87bdfb`
**Validation level:** Level 1 public service only
**Real PAN used:** no
**CAPTCHA/OTP bypass attempted:** no

## Method and access boundary

Gate 2 used one-off, human-paced requests to official public pages, public issue-list resources, and identifier-free challenge/token handshakes. Headless Chromium rendered each official page. No corpus, repeated crawler, investor result request, real member identity, CAPTCHA answer, or OTP was used.

`robots.txt` was unavailable on KFintech, blocked by the generic web extractor for Bigshare's DNS classification, and returned 404 on MUFG. Because this was one-off metadata/contract validation rather than ingestion, validation stayed limited to official pages and did not mirror third-party content.

## KFintech

### Observed

- Official page: <https://ipostatus.kfintech.com>, HTTP 200.
- React UI rendered and exposed PAN, Application Number + PAN, and NSDL/CDSL Demat lookup.
- 64 current `{clientId,name}` issue records were present in the public bundle.
- Current UI calls an API Gateway GET with `type={pan|appno|dpclid}`, `client_id`, and `reqparam` headers.
- No CAPTCHA or OTP appeared in the rendered current flow.
- Recognized result records use `All_Shares`; the public client treats numeric zero as not allotted.
- Client code distinguishes 404, 429, 500, 502, 504, and network failure.

### Capability

Public issue discovery and deterministic HTTP request mechanics are visible. The official UI requires JavaScript, but a provider adapter can use the explicit HTTP contract after its contract and fixture parsers are approved.

### Current implementation status

**MAJOR UPDATE REQUIRED.**

`LiveKfintechProvider` reaches only the root page, cannot parse the current issue list, declares any non-challenge 200 page available, and always returns `UNKNOWN` for live result checks. The safe provider/error abstraction is reusable.

### Changes required

1. Parse and validate explicit `{clientId,name}` issue records rather than arbitrary HTML lines.
2. Health-check root, bundle, non-empty issue list, and expected endpoint/header contract.
3. Add structured sanitized API fixtures before live result parsing.
4. Map 404/429/5xx/network/parser states explicitly.
5. Treat `All_Shares == 0` as `NOT_ALLOTTED` only inside a validated successful record.

## Bigshare

### Observed

- Official landing and three status-server URLs are current.
- Final check: Servers 1, 2, and 3 returned HTTP 200.
- Server 2 returned a transient HTTP 503 earlier in the same window, then recovered.
- The final public page exposed four issue IDs: `9046`, `9045`, `590`, and `9044`.
- Lookup modes: Application/CAF, Beneficiary ID, and PAN.
- Current new-search flow requires a server-verified image CAPTCHA.
- `Captcha.ashx` returned an image data URL plus signed challenge token; no answer was submitted.
- Result request uses `Data.aspx/FetchIpodetails` with explicit `OK`, `NOTFOUND`, `CAPTCHA`, `RATELIMIT`, and `WARMING` states.
- HTTP 429/503 and `Retry-After` are handled explicitly.
- The page changed from a legacy client-side challenge to the current server-verified challenge during observation, proving active adapter drift.

### Capability

Public issue discovery is available. Automated investor lookup is **blocked by legitimate human verification**. The supported continuation is isolated official-page/browser verification or manual entry; not CAPTCHA bypass.

### Current implementation status

**NOT IMPLEMENTED / BLOCKED BY HUMAN VERIFICATION for unattended lookup.**

### Changes required

1. Add Bigshare public discovery and capabilities.
2. Validate the current token/image challenge contract in health checks.
3. Return `NEEDS_HUMAN_VERIFICATION` before any result lookup.
4. Map `NOTFOUND`, `RATELIMIT`, `WARMING`, HTTP 429, and HTTP 503 without guessing.
5. Add sanitized structured response fixtures before any result parser is enabled.

## MUFG Intime

### Observed

- Current brand/page: MUFG Intime India at <https://in.mpms.mufg.com/Initial_Offer/public-issues.html>, HTTP 200.
- Public `POST IPO.aspx/GetDetails` returned four current company IDs: `11926`, `11925`, `11924`, `11923`.
- Lookup modes: PAN, Application Number, DP/Client ID, and Account Number + IFSC.
- Status flow obtains a token from `IPO.aspx/generateToken`, uses a cookie, and submits through `IPO.aspx/SearchOnPan`.
- CAPTCHA image/input/check code remains in the page, but the container is hidden and its client validation is commented out in the current raw and rendered page.
- Current CAPTCHA requirement is therefore conditional/unknown, not safely "always required" or "absent".
- Inline footer code still contains stale Link Intime branding.
- No explicit rate-limit contract was verified.

### Capability

Identifier-free HTTP issue discovery works. Current result lookup is JavaScript/session/token dependent. Deterministic unattended result parsing is not proven and must remain disabled.

### Current implementation status

**NOT IMPLEMENTED.**

### Changes required

1. Add MUFG public discovery for JSON-wrapped XML company records.
2. Declare browser/session/token capabilities.
3. Detect whether the dormant CAPTCHA becomes active and return `NEEDS_HUMAN_VERIFICATION`.
4. Build sanitized XML success/failure fixtures before normalization.
5. Keep legacy Link Intime only as a registrar alias, never as the routing key.

## Provider Capability Matrix

| Provider | Public Discovery | PAN Lookup | Other Lookup Keys | CAPTCHA | Deterministic HTTP | Browser Required | Human Verification | Current Health | Adapter Status |
|---|---|---|---|---|---|---|---|---|---|
| KFintech | Yes — 64 bundle records | Yes | Application + PAN; NSDL/CDSL Demat | None observed | Yes, current API contract visible; result call not exercised | Official UI: yes; adapter: potentially no | No current challenge observed | `AVAILABLE` | `MAJOR UPDATE REQUIRED` |
| Bigshare | Yes — four current server select options | Yes | Application/CAF; NSDL/CDSL Beneficiary ID | Yes — server-verified image/token | Discovery: yes; result: human-challenge gated | Yes for legitimate continuation | Required | `HUMAN_VERIFICATION_REQUIRED` | `NOT IMPLEMENTED / BLOCKED BY HUMAN VERIFICATION` |
| MUFG Intime | Yes — public WebMethod, 4 records | Yes | Application; DP/Client ID; Account + IFSC | Present but hidden/dormant | Discovery: yes; result: session/token path unproven | Yes for current official result flow | Conditional / `UNKNOWN` | `DEGRADED` | `NOT IMPLEMENTED` |

Unknown cells remain `UNKNOWN`; Gate 2 does not infer behavior from other registrars.

## Existing Abstraction Assessment

The current abstraction safely represents normalized results and operational failures:

- `ProviderHealth`: `AVAILABLE`, `DEGRADED`, `HUMAN_VERIFICATION_REQUIRED`, `BROKEN`, `UNKNOWN`;
- `ProviderError`: unavailable, rate limited, human verification, retryable, unknown;
- normalized `UNKNOWN` remains distinct from `NOT_ALLOTTED`.

It does **not** yet declare a capability matrix or provider-independent public discovery contract, and no provider registry exists. Those are additive Gate 3 design changes; website details remain isolated in adapters.

The generic `from_provider_text` helper must never parse arbitrary full HTML. Final `NOT_ALLOTTED` requires a provider-specific, structured, positively recognized success contract.

## Provider Health Model

Gate 2 confirms the product health states are sufficient:

- `AVAILABLE` — public service and expected contract are healthy;
- `DEGRADED` — reachable, but a required contract or automated path is uncertain;
- `HUMAN_VERIFICATION_REQUIRED` — legitimate human challenge blocks unattended lookup;
- `BROKEN` — expected contract is absent or parser confidence fails;
- `UNKNOWN` — health could not be established.

HTTP 200 alone is never enough. Empty issue lists, changed bundles, changed challenge mechanics, malformed structured results, or parser mismatches degrade/break the adapter and cannot yield a final allotment result.

## Cross-Provider Risks

1. **Live drift:** Bigshare changed challenge implementation during the observation window.
2. **Server inconsistency/transience:** one Bigshare server returned 503 before recovering.
3. **Embedded discovery:** KFintech issue records are coupled to a versioned frontend bundle.
4. **Legacy residue:** MUFG current pages retain some Link Intime strings.
5. **Challenge activation:** MUFG's dormant CAPTCHA may be re-enabled without notice.
6. **Unsafe phrase parsing:** generic substring parsing could mistake explanatory text for a final result if used on whole pages.
7. **Rate policy differences:** KFintech and Bigshare expose different retry behaviors; MUFG remains unknown.

Each failure remains provider-local. Historical data, investment entry, profit calculations, and manual allotment must remain available.

## Security Assessment

- No real PAN, synthetic PAN, UPI, member identity, OTP, or result lookup was sent.
- No CAPTCHA was solved, transcribed, or bypassed.
- Only public issue metadata and ephemeral challenge/token metadata were inspected.
- Ephemeral token values are absent from committed documentation.
- Provider result parsers must emit safe structured metadata only.
- PAN remains restricted to the audited `with_pan(account_id, AllotmentCheck, ...)` closure in a future approved pilot.
- `UNKNOWN`, malformed data, parser mismatch, HTTP failure, challenge, throttling, and provider unavailability can never normalize to `NOT_ALLOTTED`.

## Real-PAN Status

**STILL BLOCKED.**

Gate 2 does not authorize real-PAN testing. The controlled pilot checklist, production OS key provider, security suites, provider health, explicit owner initiation, and PAN-holder consent are still required.

## Gate 2 Verdict

**PASS WITH CHANGES REQUIRED — current provider behaviors are understood well enough to design the isolated adapter updates.**

KFintech needs a major isolated adapter refresh. Bigshare needs a human-verification-first adapter. MUFG needs discovery plus session/token-aware capability and fixture design. The domain status/error model is safe; capability discovery and deterministic registry routing still require Gate 3 design.
