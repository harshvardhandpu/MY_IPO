# Status: Automatic IPO discovery and invest auto-fill

- Gate 1 — Product: APPROVED 2026-08-31
- Gate 2 — Contract/research: PASS WITH NON-BLOCKING FINDINGS 2026-08-31
- Gate 3 — Program Design: APPROVED 2026-08-31
- Gate 4 — Phase 2 implementation and verification: PASS 2026-09-01
- Gate 4 — Phase 3 implementation and host verification: PASS 2026-09-01
- Gate 4H — Authenticated read-only live validation: PASS 2026-09-01

## Gate 4 slices
- [x] 4A — secure credential/keyring integration and safe connection status
- [x] 4B — wire contract, parser, normalization, and fixtures
- [x] 4C — bounded native HTTPS transport and public metadata cache
- [x] 4D — typed Tauri boundary
- [x] 4E — Available IPO catalogue and details
- [x] 4F — lots, accounts, and existing CHECK/SUBMIT preparation
- [x] 4G — submission revalidation and safe metadata snapshot
- [x] 4H — authenticated identifier-free metadata validation (read-only live validation)
- [x] 4I — independent whole-feature review

## Phase 2 verification

- Host Node: `v26.8.1` (meets repository requirement `>=22.22.2`)
- Vitest: `9/9` passed
- Rust: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` passed
- Frontend: `npm run check` and `npm run build` passed
- Security/integrity: secrets scan and `git diff --check` passed
- Independent review: PASS

## Phase 2 boundary and deferred work

- Phase 2 is read-only Upstox public IPO discovery: OPEN/UPCOMING catalogue, details, safe normalized metadata, and OPEN-only auto-fill into the existing Invest composer.
- Manual entry and the existing CHECK/SUBMIT flow remain available; no broker order path was added.
- No real Analytics Token was requested or used during Phase 2 verification. Authenticated live inventory validation was completed in Phase 3; see below.
- PAN access, UPI operations, registrar investor lookup, real MUFG lookup, and Symbiotec pilot continuation remain out of scope.

## Notes for a fresh session
- Aether redesign was separately committed as `5afd75a` before this feature.
- Existing production pilot state is locked: accidental Symbiotec entry remains voided; active amount ₹0; no historical record; no PAN/MUFG lookup.
- Upstox is public metadata only. No member, friend, balance, PAN, UPI, or private Sanket data may be sent to Upstox.
- No Upstox order/application API will be integrated in this feature.
- Owner approved Gate 3 entry on 2026-08-31. Use Analytics Token only; do not design an Algo Trading App/OAuth flow or request a token in chat.
- Gate 4A passed before the owner entered the Analytics Token through native Settings.
- Current official docs: `/v2/ipos`, `/v2/ipos/{id}`, Analytics Token (read-only, one-year, free per current docs).
- Upstox IPO `id` is a separate namespace from registrar `provider_issue_id`.

## Phase 3 final verification

- Host verification passed before cleanup: Rust format check, workspace clippy with `-D warnings`, workspace tests, schema v8 tests, legacy event compatibility, Upstox Phase 3 tests, frontend production build, Rust release build, DEB build, RPM build, and `git diff --check`.
- Verified release binary is preserved at `~/.local/bin/sanket-ipo`. `target/`, `node_modules/`, and `apps/desktop/dist/` were intentionally cleaned after verification and must not be recreated for this handoff.
- AppImage packaging is a non-blocking follow-up: a square icon is not configured.
- Live authenticated validation passed through the native read-only status/catalogue/details paths. The token value was not accessed or displayed; PAN, broker orders, UPI mandates, MUFG lookup, and Symbiotec pilot activity remain out of scope.

## Phase 3 live validation

- Authenticated status: **PASS**
- OPEN IPO catalogue: **PASS**
- UPCOMING IPO catalogue: **PASS**
- IPO details and real field rendering: **PASS** — name, status, price band/cutoff/planning price, lot size, minimum quantity/lots, cost per lot, minimum application amount, bidding dates, allotment/listing dates, registrar, and subscription state.
- Catalogue UI: **PASS**
- Price/lot math: **PASS**
- Registrar parsing: **PASS**
- Token leak: **NO**
- PAN accessed: **NO**
- Financial record created: **NO**
- Broker order placed: **NO**
- Upstox IPO id remained separate from registrar `provider_issue_id`.
- **UPSTOX FEATURE: READY FOR USE**

## Locked Gate 1 product decisions
- Upstox-supplied IPO metadata is read-only in normal auto-fill mode. Explicit `MANUAL ENTRY` is the only override path and uses `metadata_source = OWNER_MANUAL_METADATA`.
- `cost_per_lot = lot_size × planning_price`; `minimum_application_amount = minimum_quantity × planning_price`. `minimum_lots` is derived only for valid multiples; one-lot wording is forbidden when minimum quantity exceeds lot size.
- Price basis is explicit: `CUT_OFF`, otherwise `UPPER_BAND_ESTIMATE`, otherwise `TBA`; missing prices never display as ₹0.
- Cached catalogue data is visibly timestamped. Online submission attempts a current refresh and verifies `OPEN`; offline submission must clearly disclose cached metadata before allowing the existing workflow.
- IPO dates are Indian calendar date-only values. Missing dates display `TBA`; no timezone shifting or inferred dates.
- `upstox_ipo_id` and `registrar_provider_issue_id` are separate namespaces. Upstox id never populates provider issue id without independent provider evidence.
- A submitted/selected investment may snapshot only necessary safe public metadata with source, fetched time, price basis, lot/minimum quantities, planning price, dates, and registrar name. Raw responses and credentials never persist.
- Multi-account presentation always separates `PER ACCOUNT` from `TOTAL CAPITAL`; each account remains an independent application.
- Flow is `INVEST → AVAILABLE IPOs → OPEN/UPCOMING → details → lots → accounts → per-account/total capital → existing CHECK → existing SUBMIT`. Upcoming has no Invest action until open; manual entry remains available.
- Scope remains discover/select/auto-fill/plan/record only. No broker execution, UPI mandate, PAN, registrar investor lookup, or Symbiotec pilot continuation.
