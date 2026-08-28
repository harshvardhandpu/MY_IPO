# ADR: Registrar transport ownership

- **Status:** Accepted — Gate 3 approved
- **Date:** 2026-08-28
- **Decision scope:** Phase 3C registrar adapters

## Context

Gate 2 verified three materially different public workflows:

- KFintech exposes a JavaScript UI backed by a deterministic HTTP API.
- Bigshare requires a server-verified human CAPTCHA in a browser session.
- MUFG Intime uses public HTTP discovery plus a cookie/request-token result flow and a conditionally active browser challenge.

The domain needs one normalized allotment contract, but a generic request/response transport would either expose registrar details to the domain or reduce every provider to the least capable browser path.

The current KFintech adapter shells out to `curl`. That was acceptable only as a Phase 3B reachability probe. It is not the production transport: it has no typed status/headers/body contract, no in-process response cap, and no session ownership.

## Decision

Choose **provider-owned transport with two shared concrete facilities**, not a generic `RegistrarTransport` trait.

1. Each provider adapter owns its endpoints, headers, cookies, selectors, request construction, structural checks, and parser.
2. `RegistrarHttpClient` is a concrete crate-internal helper. It enforces registrar-domain allowlists, TLS certificate verification, response and redirect bounds, approved-domain-only redirects, connection/request/read timeouts, cancellation, provider-scoped cookie isolation, content-type sanity, an explicit user agent, no sensitive logging, and no response-body persistence. It never redirects a PAN-bearing request to an arbitrary third-party domain. It is not a trait and has no registrar semantics.
3. `RegistrarVerificationSurface` is an application-layer facility for visible human verification. It receives an allowlist and opaque challenge reference from an adapter; it does not implement the provider contract.
4. Provider parsers are pure functions over sanitized fixtures or ephemeral response bytes. Tests do not require a mock transport interface.
5. The obsolete shell-out `curl` helper is removed when the KFintech adapter is replaced.
6. Provider cookies, sessions, request tokens, CAPTCHA content/answers, and response bodies stay ephemeral. Restart preserves the durable job, expires the old continuation, transitions to `PREPARING_PROVIDER_SESSION` or `VERIFICATION_REQUIRED_REFRESH`, and creates a fresh legitimate session instead of serializing or reusing stale state.
7. Each provider may emit `NOT_ALLOTTED` only after provider, issue, expected structure, provider-specific negative marker, and parser unambiguity are all confirmed.
8. Sanitized fixtures carry provider/source/retrieval/type/sanitization/hash provenance plus a structural fingerprint/version where practical. Live drift degrades the provider rather than guessing.
9. `LIVE_ADAPTER_IMPLEMENTED != REAL_INVESTOR_LOOKUP_AUTHORIZED`; Gate 4 implementation does not authorize a real-PAN investor lookup.

## Browser boundary

A verification surface is created only after a first-class `HumanVerificationChallenge` exists. It uses a fresh temporary profile/context, a registrar-specific domain allowlist, disabled downloads/devtools/tracing, no normal-browser profile, no AI integration, and deterministic cleanup on completion, cancellation, expiry, or crash.

The first Bigshare implementation asks the user to enter the lookup value and CAPTCHA directly into the official isolated page. Sanket does not inject vault PAN into browser JavaScript or IPC. The selected account remains associated by safe account id and masked display only. A future one-shot sensitive-field injector requires a separate security decision and controlled-pilot approval.

## Why not one transport trait?

A `RegistrarTransport` trait would have to model HTTP requests, cookie/token state, human pauses, visible browser lifetime, challenge expiry, and response interception. That abstraction would be larger than the three adapters and would still leak provider-specific behavior through flags and downcasts.

A concrete HTTP helper is sufficient shared policy. Human verification is a domain/application state transition, not another interchangeable network client.

## Consequences

### Positive

- website details stay inside adapters;
- HTTP providers remain simple and testable;
- Bigshare is not forced into fake unattended automation;
- MUFG can prefer HTTP and switch to the existing browser boundary only when evidence requires it;
- response limits, redirects, timeouts, and redaction are consistent;
- no mock-transport framework is needed for fixtures.

### Costs

- each adapter contains some transport orchestration;
- HTTP and browser continuations have different internal code paths;
- the browser surface needs cross-platform isolation verification before release;
- MUFG requires a bounded token/session spike before its final transport path is selected.

## Rejected alternatives

1. **Generic `RegistrarTransport` trait:** premature and leaky.
2. **Browser for every provider:** slower, harder to test, and unnecessary for KFintech.
3. **Direct shell-out `curl`:** insufficient control for sensitive production requests.
4. **CAPTCHA solver or external vision service:** prohibited.
5. **Reuse the user's normal browser profile:** violates isolation and cleanup requirements.

## Revisit when

Introduce a transport trait only after at least two implemented adapters need the same non-trivial substitutable transport behavior that the concrete helper cannot provide without duplication.
