# Gate 4 — Vertical implementation slices

**Status:** design only; awaiting separate Gate 4 approval
**Gate 3 dependency:** this plan is the implementation sequencing output, not authorization to execute it.
**Token rule:** do not ask for, generate, paste, or validate a real token until Slice 4A has passed its secure-entry acceptance criteria.
**Production implementation:** NOT STARTED

## Build-order rules

- One vertical slice at a time; no horizontal “backend first” build.
- Every non-trivial behavior follows RED → confirm intended failure → minimal GREEN → focused verification → full regression check.
- Each slice ends in a runnable state and updates `00-status.md` only after verification.
- No Upstox order/application/cancel/UPI operation is added.
- No slice sends PAN, UPI, MemberVault, balances, screenshots, account identity, or private Sanket history to Upstox.
- The existing manual investment flow remains usable at every intermediate slice.

## 4A — Secure credential/keyring integration

**Goal:** establish a native-only Connect/Replace/Disconnect/Test path before any owner token generation.

**Scope:** `provider_credentials.rs`, AppState wiring, narrow Tauri credential commands, Settings/Data Sources UI status panel, security tests. Use deterministic fake credential storage for unit tests and the real OS backend only in the platform integration check.

**Acceptance:**

- `ProviderCredentialStore` is distinct from `OsKeyringKeyProvider`, `SensitiveIdentityService`, and `with_pan`.
- Linux production storage resolves through the configured durable Secret Service backend; mock/unavailable/locked storage fails closed.
- Keyring service/key are purpose-specific and contain only the opaque Upstox Analytics Token.
- Connect/Replace input is password-style, is cleared immediately after the command settles, and returns only `CONNECTED` or `FAILED` safe DTO state.
- Disconnect deletes only the Upstox provider entry.
- Test connection performs only a bounded read-only IPO GET.
- Token is absent from React state after completion, serialized DTOs, debug output, errors, logs, settings JSON, SQLite, events, and source.
- **Only after 4A passes may the owner generate the real token in Upstox Developer Apps → Analytics.** The token is entered through Sanket’s native Settings UI, never chat.

## 4B — Wire contract, parser, and fixtures

**Goal:** normalize documented Upstox list/detail fixtures without network access or credentials.

**Scope:** private wire DTOs, exact numeric parsing, date-only parser, price basis, lot math, registrar aliases, safe normalized model, fixture files under the test module or test fixture directory.

**Acceptance:**

- Documented list and detail examples parse with snake-case names and list/detail separation.
- Numeric INR values never pass through `f64`; exact INR-to-paise conversion rejects negative, excessive-precision, non-finite, and overflow values.
- Zero/absent price becomes `TBA`; positive cut-off takes precedence over upper-band estimate.
- `lot_size`, `minimum_quantity`, `minimum_lots`, cost per lot, and minimum application amount remain distinct.
- SME multi-lot minimum and invalid-multiple cases are covered.
- Date-only fields remain unchanged and are never timezone-converted.
- Registrar aliases are display-only and fail closed; Upstox ID never becomes provider issue ID.
- No fixture is labeled or displayed as current live inventory.

## 4C — Bounded HTTPS transport and public cache

**Goal:** fetch public metadata through the existing bounded native transport and provide local-first freshness behavior.

**Scope:** additive shared HTTP operation in `crates/allotment/src/http.rs`, exact Upstox host allowlist, response/error mapping, `UpstoxCacheStore`, atomic public cache file, pacing/single-flight refresh.

**Acceptance:**

- Only `https://api.upstox.com` is permitted; redirects cannot cross the host allowlist.
- Timeout, response-byte cap, JSON content-type validation, and sanitized errors are enforced.
- Bearer authorization is added only inside Rust and never logged.
- Cache stores normalized public DTOs, `source`, `fetched_at`, and `expires_at`; no token/raw body/private data.
- Catalogue/detail TTL target is 10 minutes.
- Fresh cache returns without network; stale cache is rendered visibly while one refresh runs; duplicate refreshes coalesce.
- 401, 403, endpoint-scoped 404, 429, 5xx, network, malformed response, and successful empty OPEN result remain distinct safe states.
- Existing registrar transport callers and behavior remain compatible.

## 4D — Typed Tauri boundary

**Goal:** expose only safe public catalogue/detail/connection DTOs to React.

**Scope:** `lib.rs` AppState/setup/command registration, `upstox.rs` service, serialized DTO tests, no domain write changes yet.

**Acceptance:**

- Commands are limited to connection status/configuration, catalogue list/refresh, and details.
- No command accepts PAN, UPI, member ID, account ID, balances, or financial history as a requirement for public metadata.
- DTOs contain public normalized values and freshness only; no credential, raw response, response headers, provider issue ID, or private aggregates.
- Command errors are safe user-facing categories, not raw provider text.
- React has no direct Upstox `fetch`/XHR path.
- AppState owns one shared Upstox service/cache; commands do not create independent cache instances.

## 4E — Available IPO catalogue and details

**Goal:** make INVEST open a truthful Available IPOs catalogue with OPEN/UPCOMING separation.

**Scope:** `App.tsx` catalogue/details state, search/filter/refresh/offline states, static styling, frontend tests.

**Acceptance:**

- OPEN and UPCOMING are separate visible sections/states.
- Upcoming cards have no Invest action.
- Details display official metadata as read-only, with source and freshness timestamp.
- Missing dates/prices display TBA; stale data displays “Cached data” and “Last updated”.
- Loading, disconnected, rate-limited, unavailable, invalid-response, no-open, and manual fallback states are accessible and actionable.
- Selecting an OPEN IPO fetches/uses details without exposing the token.
- Manual entry remains available when disconnected or offline.

## 4F — Lots, accounts, and existing CHECK/SUBMIT preparation

**Goal:** connect selected details to the existing investment composer without changing authorization or account semantics.

**Scope:** additive safe composer fields, lot selector, account selector, exact per-account/total math, existing CHECK payload adapter, frontend/native tests.

**Acceptance:**

- Official auto-filled fields are read-only in `UPSTOX_READ_ONLY` mode.
- Explicit `MANUAL_ENTRY` enables overrides and changes provenance to `OWNER_MANUAL_METADATA`.
- `cost_per_lot` and `minimum_application_amount` are separately displayed.
- Per-account amount and total capital are separately displayed; each account remains an independent application.
- No Upstox source ID populates registrar provider issue ID.
- Existing CHECK and SUBMIT command names and baseline manual payloads remain compatible.
- Upcoming and TBA states cannot silently produce an investable zero amount.

## 4G — Submission revalidation and safe metadata snapshot

**Goal:** protect the existing financial write path from stale or changed public metadata and preserve explainable provenance.

**Scope:** additive `SubmitIpoInput`/`SubmitRequest` fields, native pre-submit revalidation, additive `IpoMetadataSnapshot`, existing event payload optional field, nullable/rebuildable local-index projection, submission UI gate.

**Acceptance:**

- Catalogue-selected submission performs a bounded detail refresh and verifies `status == OPEN` before existing `Application::submit`.
- No-longer-OPEN and materially changed metadata block submission for review.
- Network-unavailable/stale path clearly discloses cached use and requires explicit owner acknowledgement before continuing.
- Manual entry bypasses Upstox revalidation and carries manual provenance.
- Snapshot contains only approved public metadata, source, times, price basis, lot/minimum values, planning price, dates, and registrar name.
- Snapshot excludes token, raw wire response, account/member/PAN/UPI data, and registrar provider issue ID.
- Existing append-only event and void semantics remain intact; projection migration is additive and rebuildable.

## 4H — Authenticated metadata validation

**Goal:** validate current OPEN/UPCOMING inventory and field coverage only after 4A is secure.

**Prerequisites:** 4A–4G verified; owner independently generates an Analytics Token from Upstox Developer Apps → Analytics and enters it through the native Sanket Settings UI. The agent must not request or receive the token in chat.

**Acceptance:**

- Validation uses only native Rust calls and public IPO metadata endpoints.
- Record only safe counts, field-presence coverage, status/result category, and observation timestamp.
- Never retain, print, screenshot, paste, or summarize the credential value.
- Never send private Sanket data to Upstox.
- Confirm current OPEN and UPCOMING behavior without claiming unsupported fields or inventing registrar provider issue IDs.
- Revoke/delete the credential through the owner-controlled Settings/Developer Apps path after validation if desired.

## 4I — Independent whole-feature review

**Goal:** fresh-context security, contract, regression, and UX review before release.

**Acceptance:**

- Independent reviewer checks the complete diff and slice evidence; implementer cannot self-approve.
- Secret scan finds no credential material or token-bearing fixtures.
- Rust formatting, clippy, tests, frontend checks/build/tests, and desktop smoke path pass.
- Review confirms no PAN/UPI/MemberVault leakage, no direct frontend network path, no broker/UPI order operation, no registrar investor lookup, and no stale-cache bypass.
- Any security or logic finding blocks release and returns to the owning slice.

## Slice status and stop point

All slices are currently unchecked. This document defines order and acceptance only; it does not authorize 4A. After Gate 3 approval, request separate owner approval for the Gate 4 slice plan. After Gate 4 approval, select 4A explicitly. Do not generate or enter the real Analytics Token before 4A passes.
