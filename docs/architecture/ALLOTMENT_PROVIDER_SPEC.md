# Allotment Provider Specification

Registrar behavior must be re-verified against official live interfaces before each release. Tests use fixtures and do not hammer production sites.

## Interface

```text
provider_id() -> ProviderId
supports(ipo) -> bool
discover_issue(public_ipo) -> IssueMapping
availability(issue) -> Availability
supported_lookup_keys() -> set<LookupKey>
check_allotment(local_lookup_context) -> ProviderResponse
requires_human_verification(response) -> bool
normalize_result(response) -> NormalizedResult
```

The domain/application layer never contains registrar selectors, field names, or response phrases.

## Normalized statuses

Final: `ALLOTTED`, `NOT_ALLOTTED`, `NOT_FOUND`, `MANUAL_RESULT`.  
Operational: `PENDING`, `NEEDS_HUMAN_VERIFICATION`, `RATE_LIMITED`, `PROVIDER_UNAVAILABLE`, `RETRYABLE_ERROR`, `UNKNOWN`.

Unrecognized or malformed output maps to `UNKNOWN`; errors never map to `NOT_ALLOTTED`.

## Sensitive lookup contract

The local worker requests one PAN at a time through `SensitiveIdentityService.with_pan(account_id, ALLOTMENT_CHECK)`. The provider receives only the minimum lookup value. No request/response logger may serialize it. Temporary browser/session state is destroyed after the attempt.

## Safety and reliability

- Official machine-readable flow first, deterministic HTTP/form second, isolated headless browser third.
- CAPTCHA/OTP/anti-bot controls pause as `NEEDS_HUMAN_VERIFICATION`; no bypass.
- Per-provider domain allowlist, rate limit, backoff, timeouts, and circuit breaker.
- Manual official-page/result fallback remains available and audited.
- Persist safe provenance, attempt count, timestamps, and encrypted evidence reference—never PAN.

## First target

KFin Technologies is the first adapter spike. Bigshare and MUFG Intime remain separate future adapters selected from actual registrar metadata.
