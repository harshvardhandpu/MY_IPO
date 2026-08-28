# Product: Phase 3C Multi-Registrar Allotment

## Problem

Members cannot safely rely on one registrar integration because Indian IPOs use different registrars, public status services drift, and some require human verification. A broken or challenged service must never look like a failed allotment or block the rest of the application.

## Success metric

In acceptance scenarios for KFintech, Bigshare, and MUFG Intime, 100% of account checks end in either a positively recognized final result or an explicit pending, unavailable, unknown, rate-limited, retry, cancelled, manual, or human-verification state, with zero PAN disclosures.

## Locked fail-closed result invariant

No `UNKNOWN`, parser failure, malformed response, HTTP failure, CAPTCHA/human-verification state, rate-limit response, provider-unavailable state, unrecognized response, or unsupported-provider state may ever be normalized as `NOT_ALLOTTED`.

`NOT_ALLOTTED` is allowed only when the provider response is positively and confidently recognized as a valid not-allotted result.

- Parser mismatch → `UNKNOWN`
- HTTP 500 → `PROVIDER_TEMPORARILY_UNAVAILABLE` or `RETRY_SCHEDULED`
- HTTP 429 → `RATE_LIMITED`
- CAPTCHA or equivalent challenge → `VERIFICATION_REQUIRED`
- Unrecognized response → `UNKNOWN`

`UNKNOWN` must never visually resemble `NOT_ALLOTTED`: the former means the result could not be safely determined; the latter is a final registrar result.

## User-visible result provenance

Every account result shows:

- status;
- source/provider;
- method (`AUTOMATED` or `MANUAL`);
- checked or entered timestamp;
- confidence/provenance state;
- actor for manual results.

Registrar-confirmed, manual, pending, unknown, and verification-required results remain visually distinguishable. Manual results are permanently labeled `MANUAL` and show who entered them and when.

## Product-level job states

The workflow represents these states without collapsing them into a generic failure:

- `READY_TO_CHECK`
- `CHECKING`
- `PARTIALLY_COMPLETE`
- `VERIFICATION_REQUIRED`
- `RATE_LIMITED`
- `RETRY_SCHEDULED`
- `PROVIDER_TEMPORARILY_UNAVAILABLE`
- `UNKNOWN_RESPONSE`
- `COMPLETED`
- `CANCELLED`
- `MANUAL_RESULT`

## Restart and recovery guarantee

If Sanket IPO closes, crashes, or restarts during an allotment run:

- completed account results remain preserved;
- finalized account checks are not unnecessarily rerun;
- pending and retryable accounts can resume;
- job progress can be reconstructed;
- duplicate execution is prevented by the durable worker/lease behavior.

## Real-PAN gate

Gate 1 approval does **not** authorize real-PAN testing. Real PAN use remains blocked until the controlled pilot gate is satisfied, including:

- production OS key provider active;
- no insecure or development-key fallback in real-sensitive mode;
- provider live-health validation passed;
- PAN security invariant passed;
- allotment security invariant passed;
- owner explicitly starts the controlled pilot;
- PAN-holder authorization/consent exists;
- the real-PAN pilot checklist is satisfied.

No real PAN may be used during Gate 2.

## Announcement — the blog post before the feature

Sanket IPO can now identify the registrar for a submitted IPO and show whether its official allotment service is ready. KFintech, Bigshare, and MUFG Intime checks share one report card, while registrar outages and verification challenges remain isolated and honest. Every result shows whether it came from a confirmed provider response or a member's manual entry. Interrupted checks resume without losing completed work, and real-PAN checks remain locked behind a separate owner-approved pilot checklist.

## Screens

- `mockups/check-allotment.html` — provider health, explicit job states, result provenance, verification-required actions, cross-provider report rows, manual fallback, and profit basis.
