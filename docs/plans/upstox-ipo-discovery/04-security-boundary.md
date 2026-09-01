# Security boundary — Upstox public IPO metadata

**Gate:** 2 research/design only
**Observation:** 2026-08-31
**Production implementation:** not started

## Data-flow lock

```text
Upstox Analytics Token (OS keyring only)
        ↓  Authorization header created in Rust only
Rust Upstox public IPO service
        ↓  normalized, validated public DTO
Tauri typed commands
        ↓
React catalogue/details/planner
```

MemberVault, PAN, UPI, friend identity, balances, screenshots, encrypted identity, and private Sanket financial history are not inputs to IPO metadata requests. Upstox must never receive member/account data merely to list public IPOs. The Analytics Token is a provider credential and must not be placed into `SensitiveIdentityService.with_pan(...)` or any PAN encryption/event path.

## Credential design (not implemented)

The current repository has `OsKeyringKeyProvider` for identity key material and a development-only in-memory provider. Its purpose and value type are identity encryption keys, not opaque provider tokens. Do not reuse `SensitiveIdentityService` or overload the identity provider.

Gate 3 should design a separate purpose-specific `ProviderCredentialStore`/`PublicDataCredentialStore` backed by OS keyring entries, for example a distinct service namespace and provider key. It must:

- store only the Upstox Analytics Token as an opaque secret;
- use the real OS backend in production and fail closed if unavailable;
- never write the token to source, Git, settings JSON, SQLite, MemberVault, events, logs, screenshots, prompts, frontend state, or localStorage;
- provide only `connected`, `provider`, `last_validated_at`, and a safe error state to React;
- support replacement/deletion without exposing the old token;
- avoid logging `Authorization` headers and request bodies.

The Settings/Data Sources UI may display `UPSTOX IPO DATA`, connected status, a masked token indicator, and last validation time. It must never render the full token.

## HTTP boundary

Only Rust may call `https://api.upstox.com`. The native service should reuse the repository's bounded HTTP policy concepts: exact host allowlist, HTTPS, timeout, response-size cap, redirect host validation, JSON content-type checks, request pacing, and sanitized errors. The future API client should add the Bearer header internally and return typed safe errors; it must not return raw headers/body/query strings.

React may call only narrow typed Tauri commands such as:

- `list_available_ipos(status, issue_type, page)`
- `get_ipo_details(source_ipo_id)`
- `refresh_ipo_catalog()`
- `get_upstox_connection_status()`
- future explicit credential configure/delete commands

No command may accept PAN, UPI, member identity, account id, or financial history as a requirement for public metadata. The Upstox source IPO id remains separate from registrar `provider_issue_id`.

## Safe persistence boundary

The public catalogue cache stores normalized metadata only with source, fetched time, expiry, and safe DTO fields. It excludes credentials, raw responses, auth state beyond safe status, and member/account data.

A financial record created after catalogue selection may snapshot only necessary public metadata, such as `metadata_source = UPSTOX_IPO_API`, source IPO id, fetched time, price basis, lot/minimum quantities, planning price, allotment/listing dates, and registrar name. This snapshot explains the planning context; it does not imply Upstox submitted the investment. Owner-entered financial provenance remains separate (`OWNER_CURRENT_ENTRY` or existing domain source).

Provider discovery remains identifier-free and independent:

```text
Upstox public identity + registrar metadata
        ↓
Sanket RegistrarDiscoveryService
        ↓
registrar-specific provider_issue_id (if independently resolved)
```

The Upstox id must never be copied into `provider_issue_id` merely because names or ISINs match. Unknown registrar mappings fail closed for future allotment automation but do not block investment planning.

## UI and logging rules

Safe UI states include `AUTHENTICATION_REQUIRED`, `TOKEN_INVALID`, `RATE_LIMITED`, `UPSTOX_UNAVAILABLE`, `NETWORK_ERROR`, `INVALID_RESPONSE`, `NO_OPEN_IPOS`, and visible cached/offline provenance. Do not display raw HTTP error text if it may include URL/query/provider details.

Safe logs, if needed, may include endpoint class (`IPO_LIST`/`IPO_DETAILS`), HTTP status, bounded duration, normalized record count, and refresh timestamp. Never log token, Authorization header, API secret, OAuth code, raw response, PAN, UPI, or member identifiers.

## Explicit prohibited operations

- PAN accessed: **NO**
- UPI accessed: **NO**
- MemberVault private data sent to Upstox: **NO**
- Broker IPO order placed: **NO**
- UPI mandate created: **NO**
- IPO order withdrawn: **NO**
- Registrar investor lookup: **NO**
- Real MUFG lookup: **NOT EXECUTED**
- Symbiotec pilot continued: **NO**
- Allotment authorization altered: **NO**

## Gate 3 security acceptance criteria

Before implementation can be approved, tests must prove token absence from serialized DTOs/DOM/logging, keyring namespace separation from identity keys, no frontend Upstox URL calls, safe error sanitization, no provider-id conflation, and no public-data request path requiring MemberVault/PAN/account data.
