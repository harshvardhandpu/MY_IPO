# Gate 3 — Program Design: Upstox Analytics Token IPO discovery

**Status:** design draft; awaiting Gate 3 approval
**Observed:** 2026-08-31
**Production implementation:** NOT STARTED
**Authentication lock:** Upstox Analytics Token only; no Algo Trading App, OAuth redirect, redirect URL, client secret, or daily OAuth flow.

## Design decisions

1. Upstox is a read-only public metadata source. It cannot place IPO orders, create UPI mandates, access PAN, perform registrar investor lookups, or receive MemberVault/member/account data.
2. The token is entered only through the native Settings flow after Gate 4A passes. It is stored only in a purpose-specific OS keyring entry. It never crosses into React, JSON settings, SQLite, events, logs, screenshots, prompts, or source.
3. No new crate is required. The feature is isolated in `sanket-desktop`; the existing shared native HTTP policy in `sanket-allotment` is extended rather than reimplemented.
4. Existing CHECK/SUBMIT behavior remains the investment write path. Catalogue-selected SUBMIT performs an Upstox revalidation first; manual entry remains available when Upstox is unavailable.
5. Existing domain events and projections change additively only. A safe metadata snapshot is optional and absent for ordinary/manual entries; raw wire responses and credentials are never persisted.

## 1. Files and ownership

### Files created in implementation slices

- `apps/desktop/src-tauri/src/provider_credentials.rs` — opaque provider credential trait, OS-keyring implementation, zeroizing token wrapper, safe credential errors, and Gate 4A tests. It must not import or call `SensitiveIdentityService`, `with_pan`, `IdentityCipher`, or identity key APIs.
- `apps/desktop/src-tauri/src/upstox.rs` — Upstox Analytics Token client, wire DTOs, normalization, public normalized model, cache orchestration, submission revalidation, registrar alias normalization, safe errors, and unit tests. No Tauri/React types or private Sanket data in wire/client APIs.
- `apps/desktop/src/App.upstox.test.tsx` — catalogue, details, settings status, manual fallback, cached freshness, and submission-gate UI tests.
- `apps/desktop/src-tauri/tests/upstox_security.rs` — native boundary tests for serialized DTOs, safe error output, command input/output, and prohibited data.

### Existing files changed additively

- `apps/desktop/src-tauri/Cargo.toml` — declare direct `keyring.workspace` and `zeroize.workspace` dependencies if the provider credential module uses them directly; do not add an OAuth or network dependency.
- `apps/desktop/src-tauri/src/lib.rs` — register `provider_credentials`/`upstox` modules, add `UpstoxService` to shared `AppState`, initialize it under app-data, and register only narrow typed commands.
- `apps/desktop/src-tauri/src/service.rs` — extend `SubmitIpoInput`/`SubmitRequest` with an optional safe metadata snapshot and cache-acknowledgement/revalidation result; preserve existing manual payload compatibility and CHECK/SUBMIT persistence semantics.
- `crates/allotment/src/lib.rs` — make the existing HTTP policy module public to the desktop crate without changing registrar behavior.
- `crates/allotment/src/http.rs` — add an additive bounded JSON GET operation that accepts internal headers, returns a bounded response for native parsing, preserves exact-host HTTPS/redirect/timeout/content-type policy, and never logs bodies or authorization headers. Existing `get`/`post_json` callers remain compatible.
- `crates/domain/src/lib.rs` — add an optional safe `IpoMetadataSnapshot` field to the existing `IpoApplicationCreated` payload with serde defaults; do not add credentials, raw body, account data, PAN, UPI, or provider issue IDs.
- `crates/local-index/src/lib.rs` — project the optional snapshot into an additive nullable `metadata_json` application column or equivalent rebuildable projection; old events remain readable and old rows remain valid.
- `apps/desktop/src/App.tsx` — add Settings → Data Sources → Upstox IPO Data, catalogue/details state machine, read-only metadata display, lots/accounts planning UI, manual fallback, cache freshness, and revalidation acknowledgement. No direct `fetch` to Upstox and no token state after command completion.
- `apps/desktop/src/styles.css` — only the required catalogue/settings/details styles; preserve the separately locked Aether redesign.

### Explicitly unchanged

- `crates/identity-security/src/key_provider.rs` and `SensitiveIdentityService`: identity encryption keys/PAN only; no provider-token reuse.
- `crates/member-vault`, `crates/audit`, allotment provider lookup code, and registrar automation: no Upstox token or public IPO request path enters these modules.
- Existing application event meaning, account authorization, append-only semantics, void behavior, and existing CHECK/SUBMIT command names.
- No new database table is needed for the token. Public cache persistence is separate from token storage.

## 2. Module and crate ownership

```text
React App.tsx
  └─ typed Tauri invoke only
      ├─ provider status/configuration commands
      ├─ IPO catalogue/detail/refresh commands
      └─ existing CHECK/SUBMIT commands
          └─ sanket-desktop::service::Application

sanket-desktop::provider_credentials
  └─ keyring::Entry("sanket-ipo-provider", "upstox:analytics-token:v1")

sanket-desktop::upstox
  ├─ ProviderCredentialStore
  ├─ sanket-allotment::http::SharedHttpClient
  ├─ wire DTO parser/normalizer
  ├─ public cache under app-data/public-cache/upstox-ipo.json
  └─ safe public DTOs and submission revalidation

sanket-allotment::http
  └─ native ureq + rustls bounded transport policy

sanket-domain/local-index
  └─ additive safe metadata snapshot projection only
```

`OsKeyringKeyProvider` remains owned by `sanket-identity-security` for identity key material. `ProviderCredentialStore` is deliberately a separate type and keyring service namespace even though both use the OS keyring backend.

## 3. ProviderCredentialStore and OS-keyring semantics

### Public native signatures (design only)

```rust
pub const UPSTOX_ANALYTICS_TOKEN_KEY: &str = "upstox:analytics-token:v1";

pub struct SecretValue(Zeroizing<String>);

pub trait ProviderCredentialStore: Send + Sync {
    fn get(&self, key: ProviderCredentialKey)
        -> Result<Option<SecretValue>, CredentialError>;
    fn put(&self, key: ProviderCredentialKey, value: SecretValue)
        -> Result<(), CredentialError>;
    fn delete(&self, key: ProviderCredentialKey)
        -> Result<(), CredentialError>;
    fn exists(&self, key: ProviderCredentialKey)
        -> Result<bool, CredentialError>;
}

pub struct OsProviderCredentialStore;

impl OsProviderCredentialStore {
    pub fn new() -> Result<Self, CredentialError>;
}

pub enum ProviderCredentialKey {
    UpstoxAnalyticsToken,
}

pub enum CredentialError {
    NoDurableOsBackend,
    KeyringUnavailable,
    KeyringLocked,
    MissingCredential,
    InvalidCredential,
    StorageFailure,
}

pub struct ProviderConnectionStatusDto {
    pub provider: &'static str,       // "UPSTOX_IPO_DATA"
    pub state: ConnectionState,       // NOT_CONNECTED | CONNECTED | FAILED
    pub last_validated_at: Option<String>,
    pub safe_message: Option<String>,
}
```

`SecretValue` has no serialized form and a redacted `Debug` representation. `get` is called only inside native Upstox operations; it is never placed in `AppState` DTOs, React state, logs, or errors. The native Connect/Replace command clears the received token variable before returning. The React password input is cleared in `finally` regardless of success/failure.

The keyring service namespace is separate from the identity service namespace. On Linux the configured `keyring` native-sync-persistent backend must resolve to Secret Service. A mock/non-durable backend, locked store, missing D-Bus service, or unsupported platform fails closed; it must never be reported as connected or production-safe. Disconnect deletes the provider entry and returns only safe status.

### Native credential commands

```rust
#[tauri::command]
fn get_upstox_connection_status(
    state: tauri::State<'_, AppState>,
) -> Result<ProviderConnectionStatusDto, String>;

#[tauri::command]
fn connect_upstox_analytics_token(
    state: tauri::State<'_, AppState>,
    request: ConnectUpstoxTokenRequest,
) -> Result<ProviderConnectionStatusDto, String>;

#[tauri::command]
fn replace_upstox_analytics_token(
    state: tauri::State<'_, AppState>,
    request: ConnectUpstoxTokenRequest,
) -> Result<ProviderConnectionStatusDto, String>;

#[tauri::command]
fn disconnect_upstox(
    state: tauri::State<'_, AppState>,
) -> Result<ProviderConnectionStatusDto, String>;

#[tauri::command]
fn test_upstox_connection(
    state: tauri::State<'_, AppState>,
) -> Result<ProviderConnectionStatusDto, String>;

pub struct ConnectUpstoxTokenRequest {
    pub token: String, // command input only; never echoed or serialized back
}
```

Connect/Replace stores the token and performs a bounded read-only validation. The result is only `CONNECTED` or `FAILED` (plus safe status metadata). A network failure after storage is a failed validation, not permission to expose the token or silently use unauthenticated data. The UI offers Test, Replace, and Disconnect without displaying the token.

## 4. Upstox client and bounded transport

### Native client signatures

```rust
pub struct UpstoxMetadataClient {
    credentials: Arc<dyn ProviderCredentialStore>,
    http: &'static SharedHttpClient,
}

impl UpstoxMetadataClient {
    pub fn list(
        &self,
        query: IpoListQuery,
    ) -> Result<UpstoxListWireResponse, UpstoxError>;
    pub fn details(
        &self,
        source_ipo_id: &str,
    ) -> Result<UpstoxDetailsWireResponse, UpstoxError>;
    pub fn test_connection(&self) -> Result<(), UpstoxError>;
}

pub struct IpoListQuery {
    pub status: UpstoxStatus,       // OPEN or UPCOMING for this feature
    pub issue_type: Option<IssueType>,
    pub page_number: u32,
    pub records: u8,                // bounded to 30
}
```

The client constructs only `https://api.upstox.com/v2/ipos` and `/v2/ipos/{validated-id}`. The host, scheme, path, query, redirect destination, response bytes, timeout, and content type are validated by the shared native HTTP policy. It adds the Bearer authorization header internally and `Accept: application/json`; no API key/client secret or OAuth route exists in this design.

Required transport ceilings:

- exact host allowlist: `api.upstox.com`;
- HTTPS only and no redirect to another host;
- 15-second overall/connect timeout, subject to the shared policy;
- bounded response body (design ceiling: 256 KiB per response);
- JSON content-type sanity check;
- no request/response body, query, header, token, or raw provider error logging;
- local one-request-per-second floor for Upstox metadata, with one in-flight refresh;
- 429 bounded backoff; no reliance on undocumented `Retry-After`.

`UpstoxError` maps provider/status/transport failures to safe states and retains no raw URL, body, query, header, or token.

```rust
pub enum UpstoxError {
    CredentialRequired,
    TokenInvalid,
    Forbidden,
    RateLimited,
    NotFound,
    ProviderUnavailable,
    Network,
    InvalidResponse,
    PolicyRefused,
}
```

The IPO-details `404 / UDAPI100500` mapping is endpoint-scoped. A generic occurrence of the same code is not globally interpreted as “not found”.

## 5. Wire DTO → normalized IPO model → Tauri DTO

### Wire DTO rules

Wire structs are private to `upstox.rs`, use `serde` field names matching the current documented snake-case response, and preserve numeric JSON values as `serde_json::Number` or exact strings—not `f64`. They contain no account/member/PAN/UPI fields by construction.

```rust
struct UpstoxListWireResponse { status: String, data: Vec<UpstoxListWireItem>, meta_data: Option<WirePage> }
struct UpstoxDetailsWireResponse { status: String, data: UpstoxDetailsWireItem }
struct UpstoxListWireItem { id: String, symbol: String, name: String, status: String, isin: Option<String>, issue_type: String, issue_size: Option<serde_json::Number>, industry: Option<String>, minimum_price: Option<serde_json::Number>, maximum_price: Option<serde_json::Number>, bidding_start_date: Option<String>, bidding_end_date: Option<String>, total_subscription: Option<String> }
struct UpstoxDetailsWireItem { /* documented list fields plus detail-only fields */ }
```

### Normalization signatures

```rust
pub fn normalize_list_item(item: UpstoxListWireItem, fetched_at: DateTimeUtc)
    -> Result<IpoCatalogItem, NormalizeError>;

pub fn normalize_details(item: UpstoxDetailsWireItem, fetched_at: DateTimeUtc)
    -> Result<IpoDetails, NormalizeError>;

pub fn parse_inr_to_paise(value: Option<&serde_json::Number>)
    -> Result<Option<i64>, NormalizeError>;

pub fn derive_lot_math(
    lot_size: Option<u64>,
    minimum_quantity: Option<u64>,
    planning_price_paise: Option<i64>,
) -> LotMath;

pub fn normalize_registrar(value: Option<RegistrarWire>) -> RegistrarInfo;

pub fn parse_date_only(value: Option<&str>)
    -> Result<Option<DateOnly>, NormalizeError>;
```

`parse_inr_to_paise` converts an exact decimal representation to integer paise, rejects negative/non-finite/excess-precision/overflow values, and never uses floating-point arithmetic. `0`/absent announced prices become unavailable planning inputs, not ₹0 display values. `issue_size` remains exact crores display data and never becomes application amount.

`LotMath` keeps these independent:

```rust
pub struct LotMath {
    pub lot_size: Option<u64>,
    pub minimum_quantity: Option<u64>,
    pub minimum_lots: Option<u64>,
    pub cost_per_lot_paise: Option<i64>,
    pub minimum_application_amount_paise: Option<i64>,
}
```

`minimum_lots` exists only for positive quantities where `minimum_quantity % lot_size == 0`. “1 lot minimum” is rendered only when `minimum_quantity == lot_size`.

Price basis is deterministic:

```rust
pub enum PriceBasis { CutOff, UpperBandEstimate, Tba }

pub fn choose_planning_price(
    cut_off_price_paise: Option<i64>,
    maximum_price_paise: Option<i64>,
) -> (Option<i64>, PriceBasis);
```

Positive cut-off → `CUT_OFF`; otherwise positive maximum band → `UPPER_BAND_ESTIMATE`; otherwise `(None, TBA)`. Date fields remain `DateOnly` `YYYY-MM-DD` values without timezone conversion. Missing/malformed dates become TBA or invalid response according to field policy; no dates are inferred.

### Registrar normalization

```rust
pub enum RegistrarAlias { Kfintech, MufgIntime, Bigshare }

pub struct RegistrarInfo {
    pub official_name: Option<String>,
    pub short_name: Option<String>,
    pub alias: Option<RegistrarAlias>,
    pub website: Option<String>,
    pub mapping_state: RegistrarMappingState,
}

pub enum RegistrarMappingState { ConfirmedAlias, Unknown, Ambiguous }
```

Alias matching is case/spacing-normalized and fail-closed. It can produce a display alias only; it never creates `registrar_provider_issue_id`. `source_ipo_id` remains the Upstox path identity and is never copied into the registrar provider namespace.

### Safe public Tauri DTO

```rust
#[derive(Serialize)]
pub struct IpoCatalogItemDto {
    pub source: &'static str,                 // UPSTOX_IPO_API
    pub source_ipo_id: String,
    pub isin: Option<String>,
    pub symbol: String,
    pub name: String,
    pub issue_type: IssueType,
    pub status: UpstoxStatus,
    pub minimum_price_paise: Option<i64>,
    pub maximum_price_paise: Option<i64>,
    pub planning_price_paise: Option<i64>,
    pub price_basis: PriceBasis,
    pub lot_size: Option<u64>,
    pub minimum_quantity: Option<u64>,
    pub minimum_lots: Option<u64>,
    pub cost_per_lot_paise: Option<i64>,
    pub minimum_application_amount_paise: Option<i64>,
    pub bidding_start_date: Option<String>,
    pub bidding_end_date: Option<String>,
    pub allotment_date: Option<String>,
    pub listing_date: Option<String>,
    pub registrar_name: Option<String>,
    pub registrar_short_name: Option<String>,
    pub registrar_mapping_state: RegistrarMappingState,
    pub total_subscription: Option<String>,
    pub fetched_at: String,
}
```

The public DTO excludes token, raw body, response headers, provider issue ID, broker fields, account IDs, member IDs, balances, PAN, UPI, and private financial history.

## 6. Cache, stale-while-refresh, and freshness

### Cache contract

```rust
pub struct PublicIpoCacheEntry {
    pub source: &'static str,
    pub fetched_at: String,
    pub expires_at: String,
    pub items: Vec<IpoCatalogItem>,
}

pub struct UpstoxCacheStore { path: PathBuf }

impl UpstoxCacheStore {
    pub fn load(&self) -> Result<Option<PublicIpoCacheEntry>, CacheError>;
    pub fn write(&self, entry: &PublicIpoCacheEntry) -> Result<(), CacheError>;
    pub fn clear(&self) -> Result<(), CacheError>;
}
```

The file is under the platform app-data directory at `public-cache/upstox-ipo.json`, written atomically with restrictive permissions. It contains normalized public data only—never a credential, Authorization header, raw response, private identity, or account data. Target TTL is 600 seconds for catalogue and details.

### Service and Tauri commands

```rust
pub struct UpstoxService { /* credential store, client, cache, single-flight state */ }

impl UpstoxService {
    pub fn list_catalogue(&self, query: IpoListQuery) -> Result<IpoCatalogueDto, UpstoxServiceError>;
    pub fn refresh_catalogue(&self, query: IpoListQuery) -> Result<IpoCatalogueDto, UpstoxServiceError>;
    pub fn get_details(&self, source_ipo_id: &str) -> Result<IpoDetailsDto, UpstoxServiceError>;
    pub fn revalidate_for_submission(
        &self,
        snapshot: &IpoMetadataSnapshot,
        cached_acknowledged: bool,
    ) -> Result<SubmissionMetadataDecision, UpstoxServiceError>;
}

#[tauri::command]
fn list_available_ipos(
    state: tauri::State<'_, AppState>,
    query: IpoListQuery,
) -> Result<IpoCatalogueDto, String>;

#[tauri::command]
fn refresh_ipo_catalog(
    state: tauri::State<'_, AppState>,
    query: IpoListQuery,
) -> Result<IpoCatalogueDto, String>;

#[tauri::command]
fn get_ipo_details(
    state: tauri::State<'_, AppState>,
    source_ipo_id: String,
) -> Result<IpoDetailsDto, String>;
```

Fresh cache is rendered immediately. Stale cache is rendered with `Cached data` and `Last updated: <timestamp>` while one explicit/deduplicated refresh runs. A failed refresh retains stale data but changes the UI to offline/cached state. A successful empty OPEN result is `NO_OPEN_IPOS`, not an error. Upcoming items never expose Invest.

## 7. Metadata snapshot and existing investment flow

### Safe snapshot

```rust
pub struct IpoMetadataSnapshot {
    pub metadata_source: MetadataSource,       // UPSTOX_IPO_API or OWNER_MANUAL_METADATA
    pub source_ipo_id: Option<String>,
    pub fetched_at: Option<String>,
    pub revalidated_at: Option<String>,
    pub status_at_validation: Option<UpstoxStatus>,
    pub price_basis: PriceBasis,
    pub planning_price_paise: Option<i64>,
    pub lot_size: Option<u64>,
    pub minimum_quantity: Option<u64>,
    pub minimum_lots: Option<u64>,
    pub cost_per_lot_paise: Option<i64>,
    pub minimum_application_amount_paise: Option<i64>,
    pub bidding_start_date: Option<String>,
    pub bidding_end_date: Option<String>,
    pub allotment_date: Option<String>,
    pub listing_date: Option<String>,
    pub registrar_name: Option<String>,
}

pub enum MetadataSource { UpstoxIpoApi, OwnerManualMetadata, OwnerCurrentEntry }
```

The owner’s override mode must replace the metadata source with `OWNER_MANUAL_METADATA`; it cannot edit an object still labeled `UPSTOX_IPO_API`. No `registrar_provider_issue_id` is in the snapshot.

The additive event design attaches `Option<IpoMetadataSnapshot>` to the existing `IpoApplicationCreated` payload with serde defaults. `local-index` projects it into an additive nullable `metadata_json` field or equivalent rebuildable projection. Existing application rows/events remain valid, and manual entries have no Upstox snapshot.

### Revalidation decision

```rust
pub enum SubmissionMetadataDecision {
    OpenConfirmed { refreshed: IpoMetadataSnapshot },
    CachedAcknowledgementRequired { cached: IpoMetadataSnapshot },
    NoLongerOpen,
    MetadataChangedReviewRequired { current: IpoMetadataSnapshot },
    CredentialRequired,
    ProviderUnavailable,
}
```

Before catalogue-selected SUBMIT, native code attempts a details refresh and verifies `status == OPEN`. If refresh is unavailable, the UI explicitly discloses cached use and requires an owner acknowledgement before existing CHECK/SUBMIT may continue. If the IPO is no longer OPEN or safe metadata changed materially, submission is blocked until the owner reviews or switches to manual entry. Manual entry remains available offline.

The current event source field used by existing allotment behavior remains separate from `metadata_source`. Upstox metadata does not imply that Upstox submitted the investment.

## 8. Frontend state machine

```typescript
type UpstoxConnectionState =
  | "NOT_CONNECTED" | "TESTING" | "CONNECTED" | "FAILED";

type IpoCatalogueState =
  | { kind: "IDLE" }
  | { kind: "LOADING" }
  | { kind: "FRESH"; items: IpoCatalogItemDto[]; fetchedAt: string }
  | { kind: "CACHED"; items: IpoCatalogItemDto[]; fetchedAt: string; refreshing: boolean }
  | { kind: "NO_OPEN_IPOS" }
  | { kind: "ERROR"; safeState: SafeUpstoxError };

type IpoSelectionMode = "UPSTOX_READ_ONLY" | "MANUAL_ENTRY";
type SubmissionGate =
  | "NOT_REQUIRED" | "REVALIDATING" | "OPEN_CONFIRMED"
  | "CACHE_ACK_REQUIRED" | "BLOCKED_NOT_OPEN" | "REVIEW_CHANGED";
```

Settings flow:

```text
Settings → Data Sources → Upstox IPO Data
NOT_CONNECTED → Connect password field
Connect/Replace → native command → clear input immediately
CONNECTED or FAILED → Test / Replace / Disconnect only
```

Catalogue flow:

```text
INVEST → AVAILABLE IPOs → OPEN | UPCOMING
OPEN → details → read-only official metadata → lots → accounts
UPCOMING → details only; no Invest action
Unavailable/cached → visible freshness state → Manual Entry fallback
```

Auto-filled official fields are disabled/read-only in `UPSTOX_READ_ONLY` mode. Switching to `MANUAL_ENTRY` is explicit and clears Upstox provenance for edited values. Per-account amount and total capital are calculated separately before existing CHECK. The token field is never represented in application state after the command promise settles.

## 9. Call stacks

### Connect/replace/test/disconnect

```text
React Settings
  → typed invoke(connect/replace/test/disconnect)
  → Tauri command
  → UpstoxService / ProviderCredentialStore
  → Linux Secret Service keyring
  → optional bounded GET /v2/ipos?status=open&records=1
  → safe ProviderConnectionStatusDto
  → React clears token input and renders status only
```

### Catalogue

```text
React INVEST
  → list_available_ipos(OPEN or UPCOMING)
  → UpstoxService cache read
  → fresh result OR visible stale result
  → one explicit/deduplicated refresh
  → ProviderCredentialStore::get inside native client
  → SharedHttpClient authenticated GET /v2/ipos
  → wire envelope validation
  → normalization and cache write
  → safe IpoCatalogueDto
```

### Details and auto-fill

```text
React selects OPEN card
  → get_ipo_details(source_ipo_id)
  → source-id/path validation
  → authenticated GET /v2/ipos/{id}
  → details parser
  → exact money/date/lot/registrar normalization
  → safe IpoDetailsDto
  → read-only composer state
  → owner selects lots and accounts
  → deterministic per-account and total capital math
```

### Catalogue-selected CHECK/SUBMIT

```text
React CHECK
  → existing check_recommendation with approved non-secret fields

React SUBMIT
  → submit_investment with optional safe snapshot
  → UpstoxService::revalidate_for_submission when source is UPSTOX_IPO_API
  → block / require cached acknowledgement / confirm OPEN
  → existing Application::submit
  → additive safe snapshot on IpoApplicationCreated
  → existing append-only vault + local-index projection
```

## 10. Test plan

Tests are written first per vertical slice; each implementation slice must demonstrate its RED test before production code exists.

### Gate 4A credential tests

- `provider_store_uses_distinct_service_namespace` — provider keyring service/key differs from identity key service/key.
- `token_value_never_appears_in_status_or_debug` — safe DTO/debug/error output excludes token.
- `connect_clears_native_input_and_returns_only_safe_status` — Connect/Replace output has no credential field.
- `disconnect_deletes_provider_entry_only` — identity key entry is untouched.
- `mock_or_unavailable_keyring_fails_closed` — non-durable backend is not reported connected.
- `react_token_input_is_cleared_after_success_and_failure` — UI state no longer contains the password value.

### Gate 4B parser/normalization tests

- `parses_documented_list_fixture` — list fields normalize without detail-only assumptions.
- `parses_documented_detail_fixture` — lot, minimum quantity, timeline, registrar, and nullable fields normalize.
- `zero_or_missing_price_becomes_tba` — no ₹0 planning display.
- `cutoff_precedes_upper_band_estimate` — positive cut-off selects `CUT_OFF`.
- `sme_minimum_quantity_can_require_multiple_lots` — 3000/6000 yields two lots and distinct amounts.
- `invalid_lot_multiple_does_not_round` — invalid division remains unavailable/error.
- `decimal_inr_converts_to_integer_paise_without_f64` — exact boundary conversion and precision rejection.
- `date_only_never_timezone_shifts` — `YYYY-MM-DD` remains the same calendar value.
- `unknown_registrar_fails_closed` — official name remains visible; no provider issue ID appears.
- `upstox_id_never_becomes_registrar_issue_id` — namespaces remain separate.

### Gate 4C transport/cache tests

- `transport_rejects_non_https_and_wrong_host` — no network call occurs.
- `transport_caps_response_and_timeout` — bounded policy errors are safe.
- `transport_does_not_log_authorization_or_body` — captured diagnostics contain no secret/raw body.
- `cache_write_is_atomic_and_public_only` — cache contains normalized DTO/timestamps only.
- `fresh_cache_is_returned_without_network` — valid TTL avoids a call.
- `stale_cache_is_visible_while_one_refresh_runs` — stale result is labeled and duplicate refreshes coalesce.
- `successful_empty_open_result_is_not_auth_error` — maps to `NO_OPEN_IPOS`.
- `rate_limit_is_not_no_open_ipos` — 429 remains `RATE_LIMITED`.

### Gate 4D–4G boundary/UI/submission tests

- `tauri_dto_excludes_secret_private_fields` — serialized DTO has no token/PAN/UPI/member/account/provider-id fields.
- `frontend_never_calls_upstox_directly` — source/static test finds only typed bridge calls.
- `upcoming_card_has_no_invest_action` — state machine enforces product lock.
- `official_metadata_is_read_only_until_manual_mode` — inputs disabled in automatic mode.
- `manual_mode_changes_provenance` — override cannot retain `UPSTOX_IPO_API` source.
- `per_account_and_total_capital_are_distinct` — multi-account summary is deterministic.
- `submit_revalidates_open_status` — current non-OPEN blocks submission.
- `offline_cached_submit_requires_explicit_acknowledgement` — no silent stale submission.
- `metadata_snapshot_is_safe_and_rebuildable` — event/projection contains only approved fields.
- `existing_manual_check_submit_flow_regresses_zero` — existing payload/semantics remain compatible.

### Gate 4H–4I verification tests

- `authenticated_live_open_and_upcoming_validation_is_identifier_free` — owner-entered token stays native, counts/field coverage only, no retained values.
- `whole_feature_security_review_finds_no_secret_path` — independent reviewer confirms no frontend/token/PAN/order/registrar leakage.

## 11. Rollback design

- **4A:** Disconnect removes only the Upstox provider keyring entry. Identity key material and existing settings remain untouched. If keyring support fails, disable the Settings control and retain manual entry.
- **4B–4C:** Delete/ignore the public cache file; no token or financial event is affected. Revert the Upstox module and shared HTTP additive API while preserving existing registrar callers.
- **4D–4F:** Remove the new typed commands/UI route; existing INVEST/manual CHECK/SUBMIT remains the fallback. No command may silently reinterpret old payloads.
- **4G:** Snapshot fields and nullable projection are additive. Do not delete append-only events. If the snapshot projection is unavailable, rebuild from event data or show missing provenance; never fabricate it.
- **4H:** Revoke/replace the credential through the Upstox Developer Apps page or native Disconnect. Do not request a token in chat and do not retain validation output.
- **4I:** Any security/review failure blocks release and returns to the affected slice; no self-approval or automatic Gate 4 progression.

## 12. Least-confident decisions

1. Whether extending `sanket-allotment::http` is preferable to extracting a future shared HTTP-policy crate. Current choice is extension because the policy already exists and no second consumer is required yet.
2. Whether a nullable `applications.metadata_json` projection or a separate public snapshot table is the smallest additive persistence shape. Current choice is the nullable projection; revisit only if projection queries require independent indexing.
3. Whether Connect should retain a credential after a network validation failure. Current design retains it as configured but returns `FAILED`, allowing Test/Replace/Disconnect; no request may run until a successful validation state is established.
4. The exact Upstox field nullability beyond documented examples. The parser remains fail-closed and TBA-aware until authenticated validation in 4H.
5. The final Tauri/AppState lifetime wiring for a shared cache/service. The service must be initialized once at setup and shared by `Arc`; commands must not reconstruct independent caches.

## Gate 3 acceptance

Gate 3 is complete when the owner approves this document and the companion slice plan, with the explicit constraint that approval does not start Gate 4. Gate 4A must be separately selected and completed before any real Analytics Token is generated or entered.
