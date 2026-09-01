# Gate 2 — Live Upstox contract research

**Observation time (UTC):** 2026-08-31T12:55:18Z
**Scope:** research and contract validation only; no production adapter or credential persistence.

## Sources

- [Analytics Token](https://upstox.com/developer/api-documentation/analytics-token)
- [Analytics Token announcement](https://upstox.com/developer/api-documentation/announcements/analytics-token)
- [Static IP announcement](https://upstox.com/developer/api-documentation/announcements/analytics-token-static-ip)
- [Get IPOs](https://upstox.com/developer/api-documentation/get-ipos)
- [Get IPO Details](https://upstox.com/developer/api-documentation/get-ipo-details)
- [Request Structure](https://upstox.com/developer/api-documentation/request-structure)
- [Response Structure](https://upstox.com/developer/api-documentation/response-structure)
- [Error Codes](https://upstox.com/developer/api-documentation/error-codes)
- [Rate Limits](https://upstox.com/developer/api-documentation/rate-limiting)
- [API pricing](https://upstox.com/trading-api)

## Authentication findings

Upstox documents the Analytics Token as a long-lived, read-only token generated from the Developer Apps page. It is valid for one year, one active token is permitted per account, and the page exposes a Revoke action. Only GET APIs are supported. The IPO category is explicitly listed as supported and is marked “No static IP needed”. The same documentation places User, Payments, Orders, GTT Orders, Portfolio, Mutual Fund, and Trade Profit/Loss under “Static IP required”; this feature does not use those categories.

The request contract uses the HTTP Authorization Bearer scheme with the stored Analytics Token, plus `Accept: application/json`; the credential value is never present in source or documentation. IPO requests do not require an API key/client id in the request; the token is generated in the Developer Apps console. Standard OAuth is outside this Gate 2 contract: the implementation design selects Analytics Token for IPO metadata and must not rely on an undocumented OAuth lifetime or flow detail.

The official docs state that Analytics Token is free and the Upstox API pricing page states trading and data APIs are free. This is a current-source observation, not a perpetual pricing guarantee; the implementation should surface a connection error rather than silently introduce a paid route if the terms change.

No public API for Analytics Token revocation or introspection is documented. Revocation is performed through the Developer Apps console. Sanket should treat 401/invalid-token responses as disconnected and require the owner to replace/revoke the credential through the explicit Settings flow.

## Live validation

No Upstox credential was present in the execution environment (`UPSTOX_ANALYTICS_TOKEN` and `UPSTOX_ACCESS_TOKEN` absent). No credential was created, copied, stored, or requested during this research phase.

Unauthenticated, read-only probes were made against both public metadata paths:

- `GET https://api.upstox.com/v2/ipos?status=open&records=1` → HTTP 401, JSON error response, `UDAPI100050`.
- `GET https://api.upstox.com/v2/ipos?status=upcoming&records=1` → HTTP 401, JSON error response, `UDAPI100050`.

The body was inspected only for response shape and error code; no token or private data was involved. Therefore:

- **OPEN IPO COUNT:** not available without owner-supplied Analytics Token; no claim about current open inventory.
- **UPCOMING IPO COUNT:** not available without owner-supplied Analytics Token; no claim about current upcoming inventory.
- **Authenticated live field coverage:** not executed.

The documented examples are used as sanitized contract fixtures only and are not represented as current live inventory.

## Lifecycle and issue type

`status` supports `upcoming`, `open`, `closed`, and `listed`; default is `open`. `open` means bidding is active; `upcoming` means announced but bidding has not opened and price/dates may not be final. `issue_type` supports `regular` (documented as mainboard) and `sme`; default returns both.

The list endpoint supports pagination with `page_number` default 1 and `records` default 20, maximum 30. A successful empty `data` array is a confirmed zero-result response and must not be conflated with authentication, network, or server errors.

## Field/nullability observations

The list example contains `id`, `symbol`, `name`, `status`, `isin`, `issue_type`, `issue_size`, `industry`, `minimum_price`, `maximum_price`, `bidding_start_date`, `bidding_end_date`, and `total_subscription`. The documented list schema does **not** include `lot_size`, `minimum_quantity`, `cut_off_price`, `listing_date`, `registrar_info`, or `listing_exchange`; details must be fetched before an IPO is eligible for auto-fill.

The detail example additionally contains `daily_start_time`, `daily_end_time`, `face_value`, nullable `tick_size`, `lot_size`, `minimum_quantity`, `cut_off_price`, nullable `listing_price`, `listing_exchange`, nullable `rhp_url`/`drhp_url`, `timeline`, `registrar_info`, and `total_subscription`.

Documented unknown behavior:

- `minimum_price` and `maximum_price`: number in INR; documented as `0` when not announced.
- `tick_size`: number, documented `null` when not applicable.
- `listing_price`: number, documented `null` until listing.
- Detail fields other than the explicitly nullable fields are not promised nullable by the docs. The parser must reject malformed shape and treat absent/`null`/non-positive planning inputs as unavailable rather than inventing values.
- Dates in examples and field descriptions are `YYYY-MM-DD` strings. They are calendar dates, not timestamps; no timezone conversion is permitted.
- `total_subscription`: decimal string such as `"10.0"`; it may be unavailable in future responses and is informational only.

## Mainboard/SME evidence

The documented detail example is an SME IPO with `lot_size = 3000` and `minimum_quantity = 6000`, proving that `minimum_quantity > lot_size` is a supported contract state and that “minimum application” is not necessarily one lot. Derive `minimum_lots` only when `minimum_quantity % lot_size == 0` and both values are positive. Do not round or silently coerce invalid quantities.

The documented schema identifies regular as mainboard and SME as SME. No authenticated live mainboard sample was available in this environment; implementation fixtures must cover both issue types without claiming live inventory.

## Money representation

- `minimum_price`, `maximum_price`, `cut_off_price`, `face_value`, and `listing_price` are documented JSON numbers in INR.
- `issue_size` is a JSON number in INR crores, not share price and not paise. It must not be reused as investment amount.
- `total_subscription` is a decimal string multiple.

The implementation boundary must parse decimal/number input into validated integer paise using decimal arithmetic or exact string conversion. Floating-point values must never become authoritative domain money. A missing/zero price produces `TBA` planning state; a positive cut-off price produces `CUT_OFF`; otherwise a positive maximum price produces `UPPER_BAND_ESTIMATE`.

## Registrar and provider identity

The detail schema contains `registrar_info.name`, `registrar_info.email`, `registrar_info.contact_name`, `registrar_info.contact_number`, `registrar_info.website`, and `registrar_info.registrar` (short identifier). The docs do not expose a registrar-specific issue id, KFintech issue id, MUFG company id, or Bigshare issue id.

**UPSTOX PROVIDER ISSUE MAPPING: NOT PROVIDED.** Upstox `data.id` is a slug such as `autofurnish-limited-ipo` and is explicitly the path identifier for Upstox detail retrieval. It must remain separate from Sanket `provider_issue_id`; mapping stays with the independent public `RegistrarDiscoveryService` and remains optional for recording an investment.

Design-only normalized aliases:

| Upstox value family | Sanket registrar id | Automation |
| --- | --- | --- |
| `KFin Technologies Limited`, `KFintech`, case/spacing variants | `kfintech` | supported only after registry confirmation |
| `MUFG Intime India Private Limited`, historical `Link Intime India Private Limited`, case/spacing variants | `mufg_intime` | supported only after registry confirmation |
| `Bigshare Services Pvt Ltd`, case/spacing variants | `bigshare` | supported only after registry confirmation |
| any unknown/ambiguous value | no automatic id | fail closed; show official value and mapping pending |

These aliases are design input, not an implementation or proof that a particular current Upstox record uses them.

## Rate-limit finding

Upstox documents limits per API/per user. The page gives “Other Standard APIs” as 50 requests/second, 500/minute, and 2,000/30 minutes, but it does not name the IPO endpoints specifically. The separate Apply IPO limits (1/second, 10/minute, 300/30 minutes) are irrelevant and must not be used for metadata GETs. `Retry-After` behavior is not specified on the reviewed pages.

Recommendation: local catalogue TTL 10 minutes, detail TTL 10 minutes, one in-flight refresh per catalogue, no render-triggered requests, exponential backoff only for a bounded manual/background retry, and a conservative local floor of one Upstox metadata request per second. Treat 429 as `RATE_LIMITED` and do not turn it into `NO_OPEN_IPOS`.

## Error finding

Official generic contract is `{status: "error", errors: [{error_code, message, property_path, invalid_value}]}`. Snake-case fields are current; camelCase aliases are deprecated. Current documented HTTP classes include 400, 401, 403, 404, 405, 406, 410, 429, 500, and 503. The unauthenticated probe confirmed 401 with `UDAPI100050` and JSON error shape.

Recommended Sanket mapping:

| Upstox/local condition | Sanket state |
| --- | --- |
| missing configured token / 401 / `UDAPI100050` / invalid token | `AUTHENTICATION_REQUIRED` or `TOKEN_INVALID` |
| 429 / `UDAPI10005` | `RATE_LIMITED` |
| 5xx / 503 / transport outage | `UPSTOX_UNAVAILABLE` |
| DNS/TLS/connect/read timeout | `NETWORK_ERROR` |
| 2xx with invalid JSON/schema/status | `INVALID_RESPONSE` |
| successful open request with `data: []` | `NO_OPEN_IPOS` |
| IPO-details endpoint `404 / UDAPI100500` | selected IPO unavailable; preserve catalogue and offer retry/manual fallback. Do not globally classify every `UDAPI100500` response as “not found”; the generic error documentation also uses that code for an unexpected error. |

Never expose raw response bodies, Authorization headers, query strings, or provider internals through the UI.

## Gate 2 outcome

The public contract is sufficiently understood for an implementation design. The only unavailable evidence is authenticated live inventory/field coverage, because no token was present; this is a validation limitation, not a contract ambiguity. Gate 3 should require an owner-supplied token only for a later explicit live validation, never for fixtures or unit tests.
