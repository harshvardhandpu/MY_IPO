# Product: Automatic IPO discovery and invest auto-fill

## Problem
When the owner opens Invest, they should see the IPOs that are actually open or upcoming without copying facts from another website. Selecting an IPO should carry its official public details into Sanket's planning flow so the owner chooses lots and accounts instead of retyping prices, dates, registrars, and quantities. The owner must always be able to tell what is investable now, what is only upcoming, when public data was last refreshed, and when a detail is not yet announced.

## Success metric
For an IPO with complete official metadata, the owner can go from opening Invest to a populated, lot-based planning form in **three or fewer deliberate clicks**, with **zero manual IPO metadata fields** required and with the selected IPO's per-account and multi-account capital totals visible before submission.

## Announcement — the feature before the feature
Invest now opens with a live catalogue of public IPO information, organized into Open and Upcoming opportunities. Choose an IPO to see its price band, lot rules, application minimum, schedule, registrar, and subscription snapshot in one calm, readable view. Sanket then fills the planning form for you: choose the number of lots and accounts, review the exact capital required, and submit only when the plan is clear. Public catalogue data is clearly sourced and timestamped, while private account and identity data stays inside Sanket.

## Screens
- `mockups/available-ipo-catalogue.html` — Open and Upcoming catalogue with search, segment filter, refresh/offline state, cards, and details panel.
- Existing Invest composer — selected IPO metadata and lot-based planning summary.
- Existing Settings/Data Sources — Upstox IPO Data connection status and masked token configuration.

## Locked product decisions

### Official metadata is read-only
Metadata supplied by Upstox is read-only in normal auto-fill mode, including name, Upstox id, prices, lot rules, dates, registrar, and issue details. The owner must intentionally switch to `MANUAL ENTRY` to override it. Manual overrides use `metadata_source = OWNER_MANUAL_METADATA`; Upstox provenance is never retained for owner-edited values.

### Honest price and lot math
`cost_per_lot = lot_size × planning_price` and `minimum_application_amount = minimum_quantity × planning_price` are separate values. `minimum_lots = minimum_quantity / lot_size` is shown only for valid integer multiples. “1 lot minimum” is shown only when the two quantities are equal. Planning price is explicitly `CUT_OFF`, `UPPER_BAND_ESTIMATE`, or `TBA`; a missing price is never shown as ₹0.

### Freshness and date semantics
Cached catalogue data remains available for local-first use but is labeled `Cached data` with its last-updated time. Before catalogue-selected financial submission, Sanket attempts a current online refresh and verifies the IPO is still `OPEN`. If refresh is unavailable, the owner sees that cached metadata is being used before continuing; manual entry remains available. Bidding, allotment, and listing values are Indian date-only calendar dates. Missing dates display `TBA` without timezone conversion or inference.

### Identity, provenance, and account presentation
The Upstox IPO id and registrar provider issue id are different namespaces. The Upstox id never fills `provider_issue_id` merely because the IPO names match. When a catalogue-selected investment is submitted, Sanket snapshots only necessary safe public metadata with `metadata_source = UPSTOX_IPO_API`, source id, fetched time, price basis, quantities, planning price, dates, and registrar name; credentials and raw responses are excluded. The UI always shows `PER ACCOUNT` separately from `TOTAL CAPITAL`, with each account remaining an independent application.

### Approved flow and boundaries
`INVEST → AVAILABLE IPOs → OPEN/UPCOMING → details → select lots → select accounts → calculate per-account and total capital → existing CHECK → existing SUBMIT`. Upcoming IPOs are details-only until open. Manual entry is the fallback. This feature does not place broker orders, create or withdraw UPI mandates, access PAN, perform registrar investor-result lookups, alter allotment authorization, or continue the Symbiotec pilot.
