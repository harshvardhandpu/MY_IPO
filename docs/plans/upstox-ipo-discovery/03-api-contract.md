# Upstox IPO API contract — Gate 2 design input

**Status:** research-only; no production client implemented.
**Observed:** 2026-08-31

## Transport contract

| Item | Contract |
| --- | --- |
| Base URL | `https://api.upstox.com` |
| List | `GET /v2/ipos` |
| Details | `GET /v2/ipos/{id}` where `{id}` is the list item's Upstox slug id |
| Required headers | `Accept: application/json`; Authorization header using the Bearer scheme with the stored Analytics Token (credential value omitted) |
| API key/client id on metadata GET | Not documented as additionally required |
| Static IP | Not required for IPO category with Analytics Token |
| Query | `status`, `issue_type`, `page_number`, `records` |
| Status values | `open`, `upcoming`, `closed`, `listed` |
| Issue types | `regular` (mainboard), `sme` |
| Pagination | `page_number` default 1; `records` default 20, max 30 |
| Response envelope | success: `status = "success"`, `data`; list also has `meta_data.page` |
| Error envelope | `status = "error"`, `errors[]` with `error_code`, `message`, nullable `property_path`, nullable `invalid_value` |

The client must use the exact-host HTTPS allowlist `api.upstox.com`, bounded response bytes, bounded timeout, no redirects to other hosts, no request/response logging, and no raw error propagation.

## List response fields

The following is the documented list object. “Documented” means the current page describes the field; it does not mean every future record must contain a non-empty value.

| Field | Wire type | Nullability/unknown | Meaning | Sanket use |
| --- | --- | --- | --- | --- |
| `id` | string | required in documented object | Upstox IPO slug/path id | `source_ipo_id`, never provider issue id |
| `symbol` | string | documented | exchange ticker | display |
| `name` | string | documented | full IPO name | display/identity |
| `status` | string | enum | lifecycle stage | OPEN/UPCOMING gate |
| `isin` | string | documented | ISIN | safe public identity |
| `issue_type` | string | enum `regular`/`sme` | market segment | mainboard/SME display |
| `issue_size` | number | documented | INR crores | display only; not application amount |
| `industry` | string | documented | sector | optional display |
| `minimum_price` | number | `0` if not announced | lower band in INR | price band |
| `maximum_price` | number | `0` if not announced | upper band in INR | price band/planning fallback |
| `bidding_start_date` | string | `YYYY-MM-DD`; unknown handling not promised | bid-open calendar date | date-only |
| `bidding_end_date` | string | `YYYY-MM-DD`; unknown handling not promised | bid-close calendar date | date-only |
| `total_subscription` | decimal string | availability not guaranteed by contract text | overall subscription multiple | informational display |

The list does not document `lot_size`, `minimum_quantity`, `cut_off_price`, `timeline`, `listing_date`, `registrar_info`, or exchange. Details are required for complete auto-fill.

## Details response fields

| Field | Wire type | Nullability/unknown | Meaning | Sanket use |
| --- | --- | --- | --- | --- |
| `id`, `symbol`, `name`, `status`, `isin`, `issue_type`, `issue_size`, `industry` | string/number | as above | identity/classification | safe DTO |
| `minimum_price` / `maximum_price` | number | `0` if not announced | INR price band | exact boundary parse |
| `bidding_start_date` / `bidding_end_date` | string | `YYYY-MM-DD` | bid window dates | date-only |
| `daily_start_time` / `daily_end_time` | string | `HH:MM:SS` (IST) | daily bid hours | optional display, not date conversion |
| `face_value` | number | documented | INR face value | optional display |
| `tick_size` | number | documented nullable | INR price movement | optional display |
| `lot_size` | integer | documented | shares per lot | lot math |
| `minimum_quantity` | integer | documented | minimum shares to apply | minimum application math |
| `cut_off_price` | number | not documented nullable; handle absent/null/non-positive as unavailable | retail cut-off INR | price basis |
| `listing_price` | number | nullable until listing | realized listing price | not used for planning |
| `listing_exchange` | string | documented | exchange(s), e.g. `BSE`, `NSE,BSE` | display |
| `rhp_url` / `drhp_url` | string | nullable | public prospectus URL | optional link; not fetched in Gate 2 |
| `timeline` | object | details object | event calendar | date-only |
| `timeline.pre_apply_start_date` | string | documented `YYYY-MM-DD` | pre-application date | optional |
| `timeline.application_start_date` / `application_end_date` | string | documented `YYYY-MM-DD` | bid window dates | reconcile with top-level; never timezone-shift |
| `timeline.allotment_start_date` | string | documented `YYYY-MM-DD` | processing start | optional |
| `timeline.allotment_date` | string | documented `YYYY-MM-DD` | final allotment date | auto-fill/TBA |
| `timeline.refund_initiation_date` | string | documented `YYYY-MM-DD` | refund date | optional |
| `timeline.listing_date` | string | documented `YYYY-MM-DD` | exchange listing date | auto-fill/TBA |
| `timeline.mandate_end_date` | string | documented `YYYY-MM-DD` | mandate end date | not order integration |
| `registrar_info.name` | string | documented | full registrar name | display/alias design |
| `registrar_info.email` | string | documented | registrar contact | do not send private data; optional public display |
| `registrar_info.contact_name` / `contact_number` | string | documented | public contact fields | optional display; sanitize |
| `registrar_info.website` | string | documented | public website | display/link |
| `registrar_info.registrar` | string | documented | short registrar identifier | alias input |
| `total_subscription` | decimal string | may be unavailable | overall multiple | informational only |

The API docs do not define a separate `exchange` field; the documented detail field is `listing_exchange`. The API docs also do not define a registrar provider issue id.

## Normalization and calculations

- Price basis: positive `cut_off_price` → `CUT_OFF`; else positive `maximum_price` → `UPPER_BAND_ESTIMATE`; else `TBA`.
- Parse API INR number/decimal at the boundary into integer paise using exact decimal conversion with bounds. Reject non-finite, negative, excessive precision, and overflow values.
- `cost_per_lot = lot_size × planning_price`.
- `minimum_application_amount = minimum_quantity × planning_price`.
- Derive `minimum_lots` only when both quantities are positive and `minimum_quantity % lot_size == 0`.
- Never display ₹0 as an announced price. TBA quantities/prices prevent authoritative money calculation.
- The selected IPO's `source_ipo_id` is not `provider_issue_id`.

## Date contract

All documented IPO event fields are date-only `YYYY-MM-DD` values. Parse and retain the calendar date as a date-only type/string. Do not parse into UTC timestamps or apply timezone conversion. Missing, null, malformed, or invalid dates become TBA/invalid-response according to context; never infer.

## Cache and freshness contract

Public-only cache entries contain normalized DTOs plus `source`, `fetched_at`, and `expires_at`; no token or raw body. Target TTL is 10 minutes for catalogue and details. Catalogue opening may render fresh cache and perform one bounded background refresh. Manual refresh is explicit. Before catalogue-selected financial submission, attempt refresh while online and re-check `status == open`; if unavailable, disclose cached use before continuing the existing CHECK/SUBMIT workflow. A manual-entry flow remains available offline.

## Proposed typed safe DTO (design only)

```text
IpoCatalogItem {
  source: UPSTOX_IPO_API,
  source_ipo_id: string,
  isin: optional string,
  symbol: string,
  name: string,
  issue_type: REGULAR | SME,
  status: OPEN | UPCOMING | CLOSED | LISTED,
  issue_size_crore: optional exact decimal,
  minimum_price: optional Money,
  maximum_price: optional Money,
  cut_off_price: optional Money,
  planning_price: optional Money,
  price_basis: CUT_OFF | UPPER_BAND_ESTIMATE | TBA,
  lot_size: optional positive integer,
  minimum_quantity: optional positive integer,
  minimum_lots: optional positive integer,
  cost_per_lot: optional Money,
  minimum_application_amount: optional Money,
  bidding_start_date: optional DateOnly,
  bidding_end_date: optional DateOnly,
  allotment_date: optional DateOnly,
  listing_date: optional DateOnly,
  registrar_name: optional string,
  registrar_short_name: optional string,
  registrar_website: optional string,
  listing_exchange: optional string,
  total_subscription: optional exact decimal,
  fetched_at: DateTime,
}
```

This is not a production type yet and intentionally excludes credentials, account/member data, PAN, UPI, raw API response, registrar provider id, and financial event fields.

## Error mapping input

`401`/`UDAPI100050` → token missing/invalid; `403` → forbidden/unsupported credential state; `404`/`UDAPI100500` → detail not found; `429`/`UDAPI10005` → rate limited; `500`/`503` → Upstox unavailable; network/TLS/timeout → network error; 2xx malformed envelope/fields → invalid response; successful empty open array → no open IPOs. These are distinct states.

## Sources

See the source URLs and observation details in `02-live-contract-research.md`.
