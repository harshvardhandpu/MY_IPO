# Gate 3 — Provider Implementation Design

- **Gate:** Provider implementation design
- **Date:** 2026-08-28
- **Branch:** `feature/multi-registrar`
- **Gate 2 baseline:** `03d33f4` — `docs(registrar): lock gate 2 live validation`
- **Real-PAN status:** **BLOCKED**
- **Live investor-result submission:** prohibited during this gate

## Decisions at a glance

1. Keep one provider-independent domain contract and three provider-owned transports.
2. Replace the obsolete KFintech adapter instead of preserving its Phase 3B assumptions.
3. Model capabilities with enums and lookup-key sets, not scattered booleans.
4. Split execution into discovery, resolution, prepare, execute/verify, normalize, and safe persistence.
5. Treat human verification as a first-class resumable state; never as an arbitrary error string.
6. Persist only safe challenge metadata. Cookies, request tokens, CAPTCHA content/answers, response bodies, and PAN stay ephemeral.
7. Use a concrete shared HTTP helper; do not add a generic transport trait.
8. Make provider-specific structured parsers the only path to final provider outcomes.
9. Prefer deterministic HTTP for MUFG only if a bounded no-PAN spike proves its token/session contract; otherwise use the same isolated verification surface as Bigshare.
10. Keep provider retries and health/failover policies separate.
11. Treat implementation and authorization as separate gates: `LIVE_ADAPTER_IMPLEMENTED != REAL_INVESTOR_LOOKUP_AUTHORIZED`.

## Current fit

The design extends existing boundaries instead of replacing them:

- `crates/allotment/src/provider.rs` already owns `AllotmentProvider`, `ProviderHealth`, safe results, and provider errors.
- `crates/allotment/src/job.rs` already keeps account ids—not PAN—on jobs/attempts.
- `crates/allotment/src/runtime.rs` already provides durable leases and a basic rate policy.
- `crates/identity-security/src/lib.rs` already exposes PAN only inside `with_pan(..., AllotmentCheck, ...)`.
- `apps/desktop/src-tauri/src/service.rs` already persists jobs before execution and calls the provider inside `with_pan`.
- `apps/desktop/src-tauri/src/worker.rs` already reconciles resumable jobs after restart.
- `crates/domain/src/lib.rs` and `crates/local-index/src/lib.rs` already persist safe attempt, health, and issue-mapping events/projections.

Current gaps that Gate 4 must remove:

- KFintech discovery and status logic are obsolete.
- provider selection is hard-coded in `service.rs`.
- the trait has no capability/discovery/prepare/continuation contract.
- status aggregation uses strings in the service.
- manual negative results visually share `NOT_ALLOTTED` with confirmed provider results.
- `from_provider_text` could be misused on arbitrary full-page text.
- no durable human-verification record exists.

Gate 4A replaces string aggregation with the locked product job states:

```rust
pub enum AllotmentJobStatus {
    ReadyToCheck,
    Checking,
    PartiallyComplete,
    VerificationRequired,
    RateLimited,
    RetryScheduled,
    ProviderTemporarilyUnavailable,
    UnknownResponse,
    Completed,
    Cancelled,
    ManualResult,
}
```

# A. Shared provider architecture

```text
Desktop command / worker
        |
        v
ProviderRegistry -- resolves stable provider id and registrar aliases
        |
        v
AllotmentProvider (domain contract)
        |
        +-- KfintechProvider
        |      +-- RegistrarHttpClient
        |
        +-- BigshareProvider
        |      +-- RegistrarHttpClient (discovery/health only)
        |      +-- RegistrarVerificationSurface (human continuation)
        |
        +-- MufgIntimeProvider
               +-- RegistrarHttpClient + ephemeral cookie/token session
               +-- RegistrarVerificationSurface only when challenge/JS requires it
```

### Shared runtime responsibilities

- resolve stable provider id;
- persist job before external work;
- discover and resolve a public provider issue;
- aggregate provider health;
- enforce lease, cancellation, rate, retry, and restart rules;
- enter `with_pan` only after preparation says an unattended request is ready;
- persist normalized result, provenance, challenge metadata, and safe messages;
- reconstruct progress without rerunning final attempts.

### Provider-specific responsibilities

- official URLs and endpoint identifiers;
- request headers, payloads, cookies, tokens, and lookup-key encoding;
- issue-list parser and issue-id extraction;
- challenge detection and endpoint affinity;
- structural fingerprint and required fields;
- response parser and status mapping;
- safe provider-specific retry classification.

The domain/application layer never sees `client_id`, `reqparam`, CSS selectors, Bigshare server URLs, MUFG token algorithms, CAPTCHA markup, or provider response phrases.

# B. Capability model

Capabilities are static adapter facts. Health is a separate time-varying report.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LookupKeyKind {
    Pan,
    ApplicationNumber,
    ApplicationNumberAndPan,
    DematAccount,
    BankAccountAndIfsc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueDiscoveryMode {
    None,
    PublicHttp,
    PublicJavascript,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionRequirement {
    None,
    ChallengeToken,
    Cookie,
    CookieAndRequestToken,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JavascriptRequirement {
    None,
    OfficialUiOnly,
    RequiredForLookup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HumanVerificationRequirement {
    None,
    Conditional,
    Required,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderTransportKind {
    Http,
    Browser,
    Hybrid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackgroundExecution {
    Unattended,
    PrepareOnly,
    ForegroundOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub issue_discovery: IssueDiscoveryMode,
    pub lookup_keys: Vec<LookupKeyKind>,
    pub session: SessionRequirement,
    pub javascript: JavascriptRequirement,
    pub human_verification: HumanVerificationRequirement,
    pub transport: ProviderTransportKind,
    pub background: BackgroundExecution,
}

impl ProviderCapabilities {
    pub fn supports(&self, key: LookupKeyKind) -> bool;
    pub fn supports_background_lookup(&self) -> bool;
    pub fn supports_unattended_lookup(&self) -> bool;
}
```

Accepted provider declarations:

| Provider | Discovery | Keys | Session | JavaScript | Human verification | Transport | Background |
|---|---|---|---|---|---|---|---|
| KFintech | `PublicJavascript` | PAN, Application+PAN, Demat | `None` | `OfficialUiOnly` | `None` | `Http` | `Unattended` |
| Bigshare | `PublicHttp` | PAN, Application, Demat | `ChallengeToken` | `RequiredForLookup` | `Required` | `Browser` | `PrepareOnly` |
| MUFG Intime | `PublicHttp` | PAN, Application, Demat, Bank+IFSC | `CookieAndRequestToken` | `RequiredForLookup` until HTTP spike proves otherwise | `Conditional` | `Hybrid` | `PrepareOnly` initially |

`ProviderHealth` remains the accepted five-state enum. Add a safe report:

```rust
pub struct ProviderHealthReport {
    pub provider_id: String,
    pub health: ProviderHealth,
    pub checked_at: String,
    pub contract_fingerprint: Option<String>,
    pub endpoints: Vec<ProviderEndpointHealth>,
    pub safe_message: Option<String>,
}

pub struct ProviderEndpointHealth {
    pub endpoint_id: String,
    pub health: ProviderHealth,
    pub checked_at: String,
}
```

# C. Provider execution lifecycle

## Shared types and signatures

```rust
pub struct DiscoveredRegistrarIssue {
    pub provider_id: String,
    pub provider_issue_id: String,
    pub display_name: String,
    pub official_status_url: String,
    pub observed_at: String,
    pub contract_fingerprint: String,
}

pub struct ResolvedRegistrarIssue {
    pub application_id: String,
    pub provider_id: String,
    pub provider_issue_id: String,
    pub display_name: String,
    pub official_status_url: String,
    pub verified_at: String,
    pub contract_fingerprint: String,
}

pub struct PrepareLookupContext {
    pub job_id: String,
    pub attempt_id: String,
    pub account_id: String,
    pub issue: ResolvedRegistrarIssue,
}

pub struct ProviderSessionId(String);
pub struct ProviderContinuationReference(String);

// Safe metadata only. No Serialize/Debug implementation on PreparedLookup.
pub struct PreparedLookup {
    pub provider_id: String,
    pub preparation_id: String,
    pub session_id: Option<ProviderSessionId>,
    pub expires_at: Option<String>,
}

// Deliberately not Clone, Debug, Serialize, or Deserialize.
pub enum LookupSecret<'a> {
    Pan(&'a sanket_identity_security::Pan),
}

pub enum LookupPreparation {
    Ready(PreparedLookup),
    VerificationRequired(HumanVerificationChallenge),
}

pub enum ProviderExecutionOutcome {
    Completed(ProviderAllotmentResult),
    VerificationRequired(HumanVerificationChallenge),
}

// Owns zeroizing response bytes; no Clone/Debug/Serialize implementation.
pub struct EphemeralProviderPayload(Zeroizing<Vec<u8>>);

pub enum SafeProviderReason {
    Timeout,
    HttpStatus(u16),
    UnexpectedContentType,
    ResponseTooLarge,
    ContractChanged,
    NoHealthyEndpoint,
}

pub enum ProviderError {
    Unavailable(SafeProviderReason),
    RateLimited { retry_at: Option<String> },
    Retryable(SafeProviderReason),
    ContractMismatch(SafeProviderReason),
    SessionExpired,
    TokenInvalid,
    Unknown(SafeProviderReason),
}

pub trait AllotmentProvider: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn capabilities(&self) -> ProviderCapabilities;
    fn health(&self) -> Result<ProviderHealthReport, ProviderError>;
    fn discover_issues(&self) -> Result<Vec<DiscoveredRegistrarIssue>, ProviderError>;
    fn resolve_issue(
        &self,
        application_id: &str,
        ipo_name: &str,
        discovered: &[DiscoveredRegistrarIssue],
    ) -> Result<ResolvedRegistrarIssue, ProviderError>;
    fn prepare_lookup(
        &self,
        context: &PrepareLookupContext,
    ) -> Result<LookupPreparation, ProviderError>;
    fn execute_lookup(
        &self,
        context: &PrepareLookupContext,
        prepared: PreparedLookup,
        secret: LookupSecret<'_>,
    ) -> Result<ProviderExecutionOutcome, ProviderError>;
    fn resume_verified_lookup(
        &self,
        context: &PrepareLookupContext,
        challenge: &HumanVerificationChallenge,
        payload: EphemeralProviderPayload,
    ) -> Result<ProviderAllotmentResult, ProviderError>;
}
```

`EphemeralProviderPayload` owns zeroizing response bytes and has no `Debug`, `Clone`, or serialization implementation. It moves in-process from the isolated verification surface to the provider parser, then zeroizes on drop.

## Lifecycle

```text
DISCOVER
  provider.health()
  provider.discover_issues()

RESOLVE
  exact normalized name / explicit alias / existing verified mapping
  ambiguous or missing => manual issue selection, no guessing

PREPARE (no PAN)
  validate health + fingerprint + issue mapping
  create ephemeral session/token where allowed
  Ready | VerificationRequired

EXECUTE
  Ready only => enter with_pan closure
  provider builds and sends request in the closure

VERIFY
  validate HTTP status, content type, response cap, required fields,
  provider issue, and structural fingerprint

NORMALIZE
  provider-specific parser creates safe result through guarded constructors

PERSIST
  drop/zero request and response buffers
  leave with_pan closure
  persist status, method, provenance, checked_at, fingerprint, safe message
```

No provider request is allowed before its job and issue mapping are durable. No sensitive access occurs when preparation already knows a human challenge is required.

## Strict negative-result proof

Every provider defines one explicit structurally confirmed negative-result condition. `NOT_ALLOTTED` may be constructed only when all five facts hold:

1. provider confirmed;
2. issue confirmed;
3. expected response structure confirmed;
4. recognized provider-specific negative marker confirmed;
5. no parser ambiguity exists.

Any missing fact yields `UNKNOWN` or the appropriate operational state. HTTP failure, empty data, `NOT_FOUND`, verification state, parser drift, and generic negative-looking text are never negative allotment proof.

## Shared HTTP security boundary

`RegistrarHttpClient` enforces, centrally:

- a registrar-domain allowlist;
- TLS certificate verification;
- bounded response size and bounded redirect count;
- redirects only to approved provider domains;
- connection, request, and read timeouts;
- cooperative cancellation;
- provider-scoped cookie isolation;
- no sensitive request/response logging;
- no response-body persistence;
- content-type sanity checks;
- one explicit Sanket IPO user agent.

A request containing PAN is never followed to an arbitrary third-party domain. Redirect policy validates the destination before replaying any sensitive request.

# D. KFintech design

## Replacement, not incremental patching

Remove `kfintech_live.rs` after its fixture tests are superseded. Add `kfintech.rs` with the current contract only.

## Discovery and issue resolution

1. GET the official React root with a bounded body.
2. Resolve the versioned application bundle URL from the root.
3. Require the bundle host to remain `ipostatus.kfintech.com`.
4. Parse explicit `{clientId,name}` records into `DiscoveredRegistrarIssue`.
5. Require unique, non-empty ids/names and a non-empty list.
6. Resolve by exact normalized IPO name, then an explicit local alias table.
7. Persist the accepted provider issue mapping and fingerprint.
8. Ambiguous/no match remains unresolved; never choose the closest string automatically.

`clientId` becomes `provider_issue_id`. It is derived from the current public bundle and refreshed by discovery; it is never a hard-coded IPO id.

## Request construction

Provider-local type:

```rust
enum KfinLookupType {
    Pan,
    ApplicationNumberAndPan,
    DematAccount,
}

struct KfinRequest<'a> {
    endpoint: &'static str,
    lookup_type: KfinLookupType,
    client_id: &'a str,
    reqparam: Zeroizing<String>,
}
```

- endpoint host: the accepted API Gateway host from Gate 2;
- query: `type={pan|appno|dpclid}`;
- headers: `client_id` and `reqparam`;
- PAN-only background implementation initially uses `type=pan`;
- Application+PAN and Demat stay declared capabilities but are not exposed until Sanket stores those lookup keys safely;
- no cookies/session currently;
- redirects to unapproved hosts are rejected;
- request headers and bodies are always redacted from logs/errors.

The API endpoint is a provider contract constant, not a secret. A changed hostname fails health and requires an adapter update; the runtime does not trust an arbitrary host parsed from JavaScript.

## Response normalization

A success parser requires all of:

- HTTP 200;
- expected JSON content type and bounded body;
- recognized wrapper and array;
- exactly one applicable structured record or a provider-defined unambiguous no-record response;
- parseable expected fields;
- issue context consistent with the prepared lookup;
- current structural fingerprint accepted.

`NOT_ALLOTTED` requires a recognized success record with numeric `All_Shares == 0`. Positive shares produce `ALLOTTED`. Missing/duplicate/unparseable fields produce `UNKNOWN`, never a guess.

Provider error mapping:

| Observation | Domain result |
|---|---|
| 404 recognized no-record contract | `NOT_FOUND` |
| 429 | `RATE_LIMITED` |
| 500/502/503/504 or timeout | `RETRYABLE_ERROR` / `PROVIDER_UNAVAILABLE` |
| unexpected redirect/content type/schema | health `DEGRADED` or `BROKEN`, result `UNKNOWN` |
| empty/ambiguous records | `UNKNOWN` |

## Rate policy

- low-frequency discovery/health cache;
- one account request at a time;
- honor `Retry-After`;
- at most three account attempts (initial + two retries);
- exponential backoff capped at 60 seconds;
- parser mismatch is never retried automatically.

# E. Bigshare design

## Background-safe work

Background worker may:

- probe the three public endpoints at low frequency;
- discover issue ids;
- resolve/persist an issue mapping;
- select one healthy endpoint before a challenge starts;
- create a safe challenge record.

It may not submit a lookup, solve a CAPTCHA, or send challenge content to AI.

## Human-verification lifecycle

```text
START
  |
  v
public discovery + endpoint health
  |
  v
prepare lookup on one endpoint
  |
  v
VERIFICATION_REQUIRED (persist safe metadata)
  |
  v
user opens isolated registrar surface
  |
  v
user enters lookup value and CAPTCHA on official page
  |
  v
official page submits to the same endpoint
  |
  v
browser worker captures ephemeral result response
  |
  v
Bigshare parser normalizes structured response
  |
  v
destroy profile/context and persist safe result
```

The first implementation deliberately does not inject stored PAN into browser JavaScript/IPC. The user enters the lookup value directly on the official isolated page for the selected masked account. This is the smallest boundary that satisfies legitimate verification without creating a new secret bridge.

## Endpoint health/failover

```rust
struct BigshareEndpoint {
    id: &'static str, // server-1, server-2, server-3
    official_url: &'static str,
}
```

- health probes are conservative and cached;
- choose the first healthy endpoint in deterministic order before challenge creation;
- one failed endpoint does not mark the provider broken;
- `AVAILABLE` requires at least one structurally valid endpoint;
- challenge creation binds `endpoint_id` to the challenge;
- never switch endpoint during an active challenge;
- if the bound endpoint fails/expires, expire the challenge and require a new user action;
- no automatic CAPTCHA retry.

## Current contract checks

Require:

- current company select structure and valid issue ids;
- `Captcha.ashx` token + image shape;
- `Data.aspx/FetchIpodetails` contract;
- explicit `OK`, `NOTFOUND`, `CAPTCHA`, `RATELIMIT`, `WARMING` states;
- result fields `APPLICATION_NO`, `DPID`, `Name`, `APPLIED`, `ALLOTED` for `OK`.

`NOTFOUND` becomes `NOT_FOUND`, not `NOT_ALLOTTED`. Numeric zero allotment becomes `NOT_ALLOTTED` only in a validated `OK` record.

# F. MUFG Intime design

## Discovery

Use identifier-free `POST IPO.aspx/GetDetails`. Parse the JSON-wrapped XML with a non-expanding, bounded parser. Require unique `company_id` and non-empty `companyname`. Persist the accepted mapping/fingerprint.

## Session/token preparation

Provider-private, never serialized:

```rust
struct MufgEphemeralSession {
    id: ProviderSessionId,
    cookie_jar: EphemeralCookieJar,
    request_token: Zeroizing<String>,
    created_at: String,
    expires_at: Option<String>,
}
```

Preparation:

1. create a fresh provider-scoped HTTP agent;
2. obtain the session cookie and request token;
3. derive the request token exactly as the current public client does;
4. inspect the current page/contract for active CAPTCHA requirements;
5. return `Ready` only when the deterministic HTTP contract is proven;
6. return `VerificationRequired` when CAPTCHA is active or required JavaScript cannot be reproduced safely.

## Bounded HTTP proof

Gate 4D starts with a no-PAN spike using request-shape fixtures and identifier-free token/session calls. It must prove:

- cookie continuity;
- token derivation without executing arbitrary page JavaScript;
- bounded request construction;
- deterministic response parsing from sanitized fixtures;
- no browser-only state beyond a conditional challenge.

If all pass, MUFG remains `Hybrid` but uses HTTP for unattended PAN lookup. If any fail, the result path uses the isolated verification surface and asks the user to enter the lookup value directly. No browser automation is selected merely for convenience.

## Error policy

- invalid/expired token: refresh one fresh session once, then `PROVIDER_UNAVAILABLE` or `UNKNOWN`;
- active CAPTCHA: `NEEDS_HUMAN_VERIFICATION`, no automatic retry;
- missing expected XML tables/fields: `UNKNOWN` and health `BROKEN`;
- 429: `RATE_LIMITED`, honor server delay;
- 5xx/timeout: one delayed retry, then `PROVIDER_UNAVAILABLE`;
- no-record/result phrases remain `UNKNOWN` until sanitized fixtures prove the exact structured contract.

# G. Human-verification model

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HumanVerificationType {
    Captcha,
    Otp,
    OtherProviderVerification,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HumanVerificationStatus {
    Required,
    Presented,
    Completed,
    Expired,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanVerificationChallenge {
    pub challenge_id: String,
    pub provider_id: String,
    pub job_id: String,
    pub attempt_id: String,
    pub account_id: String,
    pub challenge_type: HumanVerificationType,
    pub status: HumanVerificationStatus,
    pub endpoint_id: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub continuation_reference: ProviderContinuationReference,
}
```

`ProviderSessionId` and `ProviderContinuationReference` are safe serializable random-id newtypes. They contain no provider cookie, token, lookup value, or challenge content.

The persisted challenge never contains:

- CAPTCHA image or answer;
- OTP or destination;
- cookie/session/request token;
- response body;
- PAN or lookup value;
- browser profile path;
- arbitrary provider error text.

`continuation_reference` is a random local opaque id, not a registrar token. It resolves only to an in-memory session.

## Ephemeral session restart rule

Provider cookies, sessions, request tokens, CAPTCHA content/answers, and response bodies are never serialized or reused after restart. The durable `AllotmentCheckJob` and finalized account results remain preserved. Any old continuation expires, and the unfinished attempt transitions safely to `PREPARING_PROVIDER_SESSION` or `VERIFICATION_REQUIRED_REFRESH` according to provider capability. A later user/worker action creates a fresh legitimate provider session; it never reconstructs stale sensitive runtime state.

## Verification surface

`apps/desktop/src-tauri/src/verification.rs` owns `RegistrarVerificationSurface`:

```rust
pub struct VerificationSurfacePolicy {
    pub challenge_id: String,
    pub allowed_hosts: Vec<String>,
    pub timeout_secs: u64,
}

pub enum VerificationSurfaceOutcome {
    Completed(EphemeralProviderPayload),
    Cancelled,
    Expired,
    ContractMismatch,
}

pub struct RegistrarVerificationSurface;

impl RegistrarVerificationSurface {
    pub fn open(
        &self,
        challenge: &HumanVerificationChallenge,
        policy: VerificationSurfacePolicy,
    ) -> Result<VerificationSurfaceOutcome, VerificationSurfaceError>;
}
```

The implementation uses a fresh temporary webview/browser context, registrar allowlist, visible user interaction, disabled downloads/devtools/traces/popups, cancellation/timeout, and recursive cleanup. Normal browser profiles are never used. CAPTCHA/OTP content never enters logs, screenshots, events, AI prompts, or external services.

# H. Provider session/token model

Only ids and safe lifecycle metadata cross the domain boundary. Provider session objects are adapter-private and process-local.

| State | Persisted? | Contents |
|---|---:|---|
| `ProviderSessionId` | Only inside safe challenge metadata if needed | random local id |
| Cookie jar | No | ephemeral provider cookies |
| Request token | No | zeroizing provider token |
| CAPTCHA token/image/answer | No | browser/provider memory only |
| Continuation reference | Yes | random local reference only |
| Endpoint id | Yes | safe enum-like server id |
| Structural fingerprint | Yes | hash of required public structure |
| PAN | Never | available only inside `with_pan` or official isolated page entry |

Add one safe projection table rather than a generic persisted session store:

```text
provider_challenges(
  challenge_id PRIMARY KEY,
  job_id,
  attempt_id,
  account_id,
  provider_id,
  challenge_type,
  status,
  endpoint_id,
  continuation_reference,
  created_at,
  expires_at
)
```

This table cannot resume a browser session after restart. It exists to reconstruct UI/job state and expire the old challenge honestly.

# I. Retry and rate policy

| Provider/state | Automatic behavior |
|---|---|
| KFintech 429 | honor `Retry-After`; capped retry |
| KFintech 5xx/timeout | exponential backoff; max three total attempts |
| KFintech parser mismatch | no retry loop; `UNKNOWN`, health `BROKEN` |
| Bigshare endpoint 503 before challenge | try one other healthy endpoint after cached health check |
| Bigshare endpoint failure during challenge | expire challenge; no endpoint switch |
| Bigshare CAPTCHA/invalid CAPTCHA | user action only; no automatic retry |
| MUFG invalid token | refresh one fresh session once |
| MUFG active CAPTCHA | human verification; no automatic retry |
| MUFG 429 | rate limited; honor server delay |
| MUFG 5xx/timeout | one delayed retry |
| Any recognized final result | never retry |

Replace the single global `ProviderRatePolicy::default()` decision with provider-declared `ProviderRetryPolicy`. Keep one shared limiter implementation keyed by provider and, for Bigshare, endpoint id.

# J. Live drift detection

Each adapter defines required structural expectations and computes a safe fingerprint from field/tag/endpoint names—not raw response contents.

```rust
pub struct ProviderContractFingerprint {
    pub provider_id: String,
    pub version_label: String,
    pub sha256: String,
}

pub enum ContractValidation {
    Accepted(ProviderContractFingerprint),
    Degraded { safe_reason: String },
    Broken { safe_reason: String },
}
```

Provider checks:

- required issue-list structure and non-empty validated ids;
- expected endpoint host/path;
- expected headers or token handshake;
- challenge presence/absence consistent with capabilities;
- expected response wrapper, status field, and required result fields;
- content-type and maximum-body constraints.

Rules:

- optional new fields do not break a parser;
- missing required fields, a moved endpoint, unexpected challenge activation, or lost parser confidence marks health `DEGRADED`/`BROKEN`;
- no changed fingerprint alone creates a final result;
- no raw HTML/JSON/XML is persisted for diagnostics;
- safe fingerprint, checked time, endpoint id, and a fixed safe reason are sufficient durable evidence.

# K. Fixture and test plan

Fixtures are sanitized, synthetic, reviewed, and contain no real identity. Use one named-case corpus per provider plus the minimum issue/challenge page fixture.

Every fixture has adjacent provenance metadata recording:

- provider;
- source URL;
- `retrieved_at`;
- fixture type;
- `sanitized=true`;
- content SHA-256;
- structural fingerprint/version where practical.

The SHA-256 covers the sanitized fixture bytes stored in the repository. Live structural drift from the tested fingerprint degrades/breaks provider health rather than guessing a result.

```text
crates/allotment/tests/fixtures/kfintech/issues.js
crates/allotment/tests/fixtures/kfintech/cases.json
crates/allotment/tests/fixtures/bigshare/issues.html
crates/allotment/tests/fixtures/bigshare/cases.json
crates/allotment/tests/fixtures/mufg/issues.json
crates/allotment/tests/fixtures/mufg/cases.json
```

Every `cases.json` contains named cases for:

- `ISSUE_LIST`
- `ALLOTTED`
- `NOT_ALLOTTED`
- `PENDING`
- `UNKNOWN`
- `MALFORMED`
- `RATE_LIMITED`
- `PROVIDER_UNAVAILABLE`
- `VERIFICATION_REQUIRED`

Bigshare additionally includes challenge expiry and endpoint failure. MUFG additionally includes `SESSION_EXPIRED` and `TOKEN_INVALID`. KFintech includes duplicate/ambiguous record and changed API wrapper.

Fixture values use explicit placeholders such as `[SYNTHETIC_LOOKUP]`; provider parsers must ignore reflected identity fields. Request-shape tests create a synthetic `Pan` at runtime and assert only method, URL, query, header names, and redacted values. Test failures must never print the secret.

## Required tests

### Shared

- `not_allotted_requires_confirmed_provider_provenance`
- `manual_negative_remains_manual_result`
- `unknown_parser_output_never_becomes_not_allotted`
- `prepared_lookup_and_challenge_cannot_serialize_pan`
- `restart_expires_ephemeral_challenge_but_preserves_final_attempts`
- `restart_requires_fresh_provider_session_without_serialized_secrets`
- `provider_registry_resolves_only_known_ids_and_aliases`
- `contract_mismatch_marks_provider_broken`
- `not_allotted_requires_all_five_negative_proof_facts`

### KFintech

- `discovers_current_bundle_issue_records`
- `rejects_empty_duplicate_or_ambiguous_issue_ids`
- `builds_pan_request_with_redacted_debug_output`
- `zero_all_shares_in_confirmed_record_is_not_allotted`
- `zero_text_in_malformed_response_is_unknown`
- `maps_404_429_and_5xx_without_guessing`

### Bigshare

- `preparation_returns_verification_required_without_pan_access`
- `selects_healthy_endpoint_before_challenge`
- `does_not_fail_over_during_active_challenge`
- `expired_challenge_requires_new_user_action`
- `normalizes_only_explicit_ok_result`
- `notfound_is_not_not_allotted`

### MUFG

- `discovers_json_wrapped_xml_issue_records`
- `keeps_cookie_and_request_token_ephemeral`
- `active_captcha_returns_verification_required_before_pan_access`
- `invalid_token_refreshes_once`
- `unknown_xml_message_fails_closed`

# L. Result provenance and PAN lifecycle security proof

## Provenance

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultProvenance {
    ConfirmedProviderResponse,
    ManualUserEntry,
    ProviderPending,
    ProviderUnknown,
    ProviderError,
    Fixture,
}

pub struct ProviderAllotmentResult {
    status: NormalizedAllotmentStatus,
    allotted_lots: Option<u32>,
    allotted_shares: Option<u64>,
    provider_reference: Option<String>,
    checked_at: String,
    provenance: ResultProvenance,
    contract_fingerprint: Option<String>,
    safe_message: Option<String>,
}
```

Fields become private. Provider modules construct results through guarded crate-level constructors. `NOT_ALLOTTED` is accepted only from `ConfirmedProviderResponse` with an accepted structural fingerprint, or from the explicit fixture constructor in synthetic mode.

Manual writes persist `status=MANUAL_RESULT` plus a separate `reported_outcome` (`ALLOTTED`, `NOT_ALLOTTED`, or `UNKNOWN`). They never impersonate registrar-confirmed `NOT_ALLOTTED`.

## Sensitive lifetime

```text
encrypted identity envelope
  -> authorize AllotmentCheck purpose
  -> decrypt inside with_pan closure
  -> Pan reference passed as non-serializable LookupSecret
  -> provider creates zeroizing request value
  -> bounded request and ephemeral zeroizing response
  -> structured normalization
  -> request/response/Pan dropped before closure returns
  -> purpose audit + safe normalized result persisted
```

For Bigshare and MUFG browser challenge paths, the first implementation does not decrypt the vault PAN. The user enters the lookup value directly in the isolated official surface. The app associates that surface with a safe account id and masked display.

Compile-time/structural protections:

- `LookupSecret`, `PreparedLookup`, ephemeral sessions, and payloads do not implement serialization;
- ephemeral types do not implement `Debug` or `Clone`;
- request/response buffers use zeroization where sensitive reflections are possible;
- persisted events/tables accept only dedicated safe types;
- free-form safe strings continue through embedded-PAN rejection;
- browser traces, screenshots, devtools, downloads, and AI integrations are disabled;
- domain errors use fixed safe categories, never provider bodies.

PAN is forbidden from `ProviderSession`, challenge, job, attempt, result, event, SQLite, report card, log, error, filename, note, provenance, or AI payload.

## Implemented is not authorized

`LIVE_ADAPTER_IMPLEMENTED != REAL_INVESTOR_LOOKUP_AUTHORIZED`.

Gate 4 may implement and verify live provider machinery with public discovery, sanitized fixtures, and synthetic identities. It may not automatically perform a real-PAN investor lookup. Real PAN remains blocked until the separately authorized controlled-pilot gate, including production key provider, live-health and security invariants, explicit owner initiation, PAN-holder consent, and completed pilot checklist.

# M. Files and proposed implementation sequence

## Planned files

| File | Gate 4 purpose |
|---|---|
| `crates/allotment/src/provider.rs` | capability, preparation, challenge, provenance, and guarded result contracts |
| `crates/allotment/src/status.rs` | remove arbitrary-page normalization as a final-result path; retain typed status helpers |
| `crates/allotment/src/runtime.rs` | provider retry policies, endpoint-aware limiter, state aggregation |
| `crates/allotment/src/http.rs` | concrete bounded/redacting HTTPS helper |
| `crates/allotment/src/registry.rs` | deterministic provider id/alias resolution |
| `crates/allotment/src/kfintech.rs` | current KFintech discovery, request, health, and structured parser |
| `crates/allotment/src/kfintech_live.rs` | delete after replacement tests pass |
| `crates/allotment/src/bigshare.rs` | discovery, endpoint health, challenge contract, structured parser |
| `crates/allotment/src/mufg_intime.rs` | discovery, ephemeral session/token path, conditional challenge, parser |
| `crates/allotment/src/job.rs` | product job states and manual reported outcome |
| `crates/allotment/src/lib.rs` | explicit public exports only |
| `crates/allotment/Cargo.toml` | one maintained blocking HTTPS client and existing workspace zeroize dependency |
| `crates/allotment/tests/*_provider.rs` | shared and provider contract tests |
| `crates/allotment/tests/fixtures/**` | sanitized named-case corpora |
| `crates/domain/src/lib.rs` | safe challenge/provenance events |
| `crates/local-index/src/lib.rs` | schema v5 challenge/provenance projections and restart reconciliation |
| `apps/desktop/src-tauri/src/service.rs` | registry orchestration and exact prepare/execute/persist flow |
| `apps/desktop/src-tauri/src/worker.rs` | pause/resume/expiry behavior; no challenge auto-retry |
| `apps/desktop/src-tauri/src/verification.rs` | isolated registrar verification surface |
| `apps/desktop/src-tauri/src/lib.rs` | verification commands/state wiring |
| `apps/desktop/src/App.tsx` | capability, provenance, challenge, partial/retry UI |
| `apps/desktop/src/App.flows.test.tsx` | user-visible state and provenance tests |

No new generic session service, browser framework, transport factory, or provider plugin system is planned.

## Gate 4A — shared capability/session/runtime changes

1. Add types, guarded constructors, registry, retry policy, safe challenge event/projection, and schema v5.
2. Keep fixture provider green.
3. Prove restart, cancellation, provenance, and PAN-free persistence.

## Gate 4B — KFintech adapter replacement

1. Add sanitized fixtures and parser tests.
2. Implement discovery/health and request-shape tests.
3. Replace old adapter; remove `kfintech_live.rs` only after tests pass.
4. Keep real network investor lookup disabled.

## Gate 4C — Bigshare human-verification adapter

1. Add endpoint/discovery/challenge fixtures.
2. Implement prepare-only background path and endpoint affinity.
3. Implement isolated user-visible continuation with direct user entry.
4. Normalize only the ephemeral structured response.

## Gate 4D — MUFG session/token adapter

1. Implement discovery fixtures.
2. Run bounded no-PAN HTTP token/session proof.
3. Select HTTP result path only if deterministic proof passes; otherwise reuse verification surface.
4. Add conditional-CAPTCHA and token-expiry tests.

## Gate 4E — cross-provider normalization and UI

1. Thread capability, job state, method, checked-at, and provenance fields.
2. Render confirmed/manual/pending/unknown/verification states distinctly.
3. Preserve cancellation, partial completion, restart, and manual fallback.

## Gate 4F — independent review

1. Full Rust/frontend/security gates.
2. Secret/PAN persistence scan.
3. Independent free-model review with explicit verdict.
4. No controlled pilot until all findings close.

# N. Gate 4 entrance criteria

Gate 4 may begin only after owner approval of this design and ADR. The approved slice plan must preserve:

- provider-owned transports;
- guarded `NOT_ALLOTTED` construction;
- first-class challenge state;
- no persisted provider session secrets;
- no browser profile reuse;
- no CAPTCHA/OTP bypass or AI challenge processing;
- Bigshare endpoint affinity during a challenge;
- MUFG HTTP-first proof with safe browser fallback;
- fixtures before parser implementation;
- real PAN blocked through Gate 4 and independent review.

## Call stacks

### Unattended HTTP path

```text
allotment worker
 -> acquire durable lease
 -> registry.resolve(provider_id)
 -> provider.health + discover + resolve
 -> provider.prepare_lookup(no PAN)
 -> with_pan(AllotmentCheck)
 -> provider.execute_lookup(LookupSecret::Pan)
 -> provider parser + guarded result constructor
 -> drop/zero sensitive values
 -> persist audit + safe result/provenance
 -> release lease
```

### Human-verification path

```text
allotment worker
 -> provider.prepare_lookup(no PAN)
 -> persist HumanVerificationChallenge
 -> pause attempt/job
 -> user opens isolated verification surface
 -> user completes official challenge and lookup
 -> ephemeral response moves to provider parser
 -> persist safe normalized result/provenance
 -> destroy browser context
 -> resume remaining accounts
```

### Restart

```text
worker starts
 -> reclaim expired job leases
 -> keep final attempts unchanged
 -> expire continuations whose ephemeral session vanished
 -> PREPARING_PROVIDER_SESSION for unattended provider work
 -> VERIFICATION_REQUIRED_REFRESH for human-verification work
 -> create a fresh legitimate session only on resumed user/worker action
```

## Least confident decisions

1. MUFG token derivation may still require browser JavaScript. The bounded Gate 4D spike resolves this without changing the domain contract.
2. Cross-platform Tauri/webview profile isolation must be proven on Linux, Windows, and macOS. Unsupported platforms fail closed to manual official-page fallback.
3. KFintech's API Gateway host may change. The adapter deliberately breaks health rather than trusting a newly discovered arbitrary host.

None requires an owner architecture choice before implementation; each has a fail-closed Gate 4 acceptance test.

## Gate 3 exit verdict

**PASS — IMPLEMENTATION DESIGN READY**

Real PAN remains blocked. Gate 4 implementation must not begin before Gate 3 approval and an approved vertical-slice plan.
