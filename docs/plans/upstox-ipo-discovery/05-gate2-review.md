# Gate 2 review — Upstox IPO public metadata contract

**Observation date:** 2026-08-31
**Implementation status:** NOT STARTED

## Verdict

**PASS WITH NON-BLOCKING FINDINGS**

The current official Upstox documentation is sufficient to proceed to Gate 3 implementation design. The public IPO list/detail paths, Analytics Token restrictions, response envelope, date/money shapes, registrar structure, and the absence of provider issue identifiers are explicit enough to design a fail-closed adapter.

## Findings

### Passed

- IPO list is `GET /v2/ipos`; detail is `GET /v2/ipos/{id}`.
- Analytics Token is explicitly supported for IPO GET APIs, read-only, one-year, free, one active token per account, and does not require static IP for IPO.
- API calls require `Accept: application/json` and the Authorization Bearer scheme; the credential value is omitted. No extra API key is documented for metadata GETs.
- Open/upcoming/closed/listed lifecycle and regular/SME issue-type filters are explicit.
- List and detail schemas are correctly separated; details are required for lot rules, cut-off price, timeline, registrar, and listing exchange.
- `minimum_quantity` can exceed `lot_size`; the documented SME example proves the distinction.
- Prices are INR JSON numbers; issue size is INR crores; subscription is a decimal string. Exact boundary conversion is required.
- Dates are documented `YYYY-MM-DD` calendar dates; timezone conversion is prohibited.
- Registrar data is nested in `registrar_info`; no registrar provider issue id appears in the documented contract.
- Upstox id remains distinct from registrar provider issue id.
- Generic errors distinguish HTTP 401/403/404/429/5xx and JSON error envelopes; live unauthenticated probe confirmed 401/`UDAPI100050`.
- No PAN, UPI, MemberVault private data, financial mutation, broker order, or registrar investor lookup occurred.

### Non-blocking findings / Gate 3 prerequisites

1. **Authenticated live inventory unavailable:** no Analytics Token was present during Gate 2, so current OPEN/UPCOMING counts and live field coverage were not run. Gate 3 may proceed with sanitized documented fixtures; a later owner-authorized live validation should report counts/coverage without storing the token.
2. **IPO-specific rate limit not named:** Upstox publishes a generic “Other Standard APIs” limit of 50/sec, 500/minute, and 2,000/30 minutes, but does not explicitly classify IPO. Use a conservative local one-request-per-second floor and ten-minute cache until observed otherwise.
3. **Retry-After not documented:** treat 429 as rate limited, use bounded backoff, and do not require a Retry-After header.
4. **Nullability is incomplete:** only some null/zero semantics are explicit. The adapter must distinguish absent/null/zero, validate required identity fields, and convert unavailable planning inputs to TBA rather than guessing.
5. **Revocation endpoint not documented:** Settings should direct the owner to replace/delete the stored credential; 401 marks the provider disconnected. Do not invent a revocation API.
6. **Pricing can change:** current official sources say Analytics Token and trading/data APIs are free. Gate 3 must not introduce paid infrastructure or an alternate paid route.

## Required Gate 3 design locks

- Separate `ProviderCredentialStore` keyring purpose from identity key provider.
- Native-only Upstox HTTP with bounded transport and sanitized error states.
- Public DTO excludes credential, raw body, member/account/PAN/UPI data, registrar provider id, and broker execution fields.
- Cache public normalized data only; visible timestamps and stale-before-submit protection are mandatory.
- Manual entry is explicit and carries `OWNER_MANUAL_METADATA`; auto-filled official fields are read-only.
- `CUT_OFF`, `UPPER_BAND_ESTIMATE`, and `TBA` are explicit price-basis states.
- Lot cost and minimum application amount are separate integer-money calculations.
- Upcoming cards never expose Invest action.
- Registrar alias recognition is fail-closed and provider issue mapping remains independent.
- No Upstox apply/order/cancel/UPI APIs in this phase.

## Independent review corrections applied

- Static-IP wording now enumerates the documented category split instead of using a broad “account-specific GET” shorthand.
- The standard OAuth lifetime aside is explicitly out of scope because it was not supported by the requested Gate 2 source set.
- `UDAPI100500` is scoped to the IPO-details 404 mapping; generic responses using the same code are not globally treated as “not found”.

## Gate 2 boundary report

- **GATE 2 LIVE CONTRACT RESEARCH:** PASS WITH NON-BLOCKING FINDINGS
- **TOKEN STORAGE DESIGN:** READY FOR GATE 3 DESIGN; not implemented
- **FRONTEND TOKEN EXPOSURE:** NO
- **PAN ACCESSED:** NO
- **BROKER ORDER PLACED:** NO
- **REGISTRAR INVESTOR LOOKUP:** NO
- **REAL MUFG LOOKUP:** NOT EXECUTED
- **IMPLEMENTATION:** NOT STARTED
