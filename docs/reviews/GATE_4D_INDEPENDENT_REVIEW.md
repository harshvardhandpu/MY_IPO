# Gate 4D Independent Review

- **Reviewer:** `moonshotai/kimi-k3`
- **Provider:** NVIDIA NIM (free route; zero paid usage)
- **Review date:** 2026-08-29
- **Implementation scope:** Gate 4D MUFG Intime session/token adapter, commit `db5be5d`
- **Correction scope:** blocking CAPTCHA visibility finding plus original non-blocking findings 1, 2, 3, 4, and 6, commit `82971e0`
- **Mode:** independent, read-only, fail-closed review and re-review
- **Route evidence:** runner configured `moonshotai/kimi-k3`; response metadata reported served model `moonshotai/kimi-k3`
- **Implementation routing history:** original adapter recorded GLM 5.3 via TokenRouter; correction used `gpt-5.6-sol` via `openai-codex`; reviewer was independent of both
- **Policy:** **GEMINI = DISABLED FOR SANKET IPO BY OWNER POLICY**

## Initial verdict

**FAIL — BLOCKING FINDINGS**

The reviewer found page-global CAPTCHA visibility detection: an unrelated `display:none` could incorrectly classify a visible active CAPTCHA as dormant. This was a fail-open direction and blocked Gate 4D.

The owner authorized a narrow correction of the blocker and original non-blocking findings 1, 2, 3, 4, and 6. Original finding 5 — lexical RFC3339 comparison in `MufgEphemeralSession::is_expired` — remained an explicitly accepted Gate 4C ceiling and was not changed.

## Correction re-review verdict

**PASS**

- **Blocking findings:** NONE
- **Non-blocking findings:** two safe-direction behavioral notes only:
  1. CAPTCHA visibility recognizes the observed inline MUFG marker/container contract; class/external-CSS/higher-ancestor hiding would classify as `Required`, not bypass the challenge.
  2. Malformed attributes on unrelated input elements can fail token parsing closed to `Unknown`.

## What the reviewer confirmed

1. CAPTCHA visibility is scoped to the `CImage`/`txtCaptch` element and its containing MUFG `<div>`; unrelated hidden elements no longer force `Dormant`.
2. Visible CAPTCHA is `Required`, explicit hidden container is `Dormant`, and malformed boundaries are `Unknown`.
3. CAPTCHA probes are ASCII-case-insensitive; unstructured CAPTCHA-like text is `Unknown`, not `Absent`.
4. A trailing `company_id` without `companyname` fails issue discovery closed.
5. Structurally recognized `NO RECORD` semantics precede generic retry wording in the same provider message row; unknown messages remain `Unknown`.
6. `hidToken` extraction is limited to an actual matching input element; missing, empty, duplicate, and malformed sources fail closed.
7. `MufgLookupRequest` no longer derives `Serialize`/`Deserialize`; outbound JSON remains explicitly built at the transport boundary.
8. No regression was introduced in token redaction, member-field exclusion, or guarded `NOT_ALLOTTED` construction.
9. `MufgEphemeralSession::is_expired` was unchanged from `db5be5d` (SHA-256 `33673c31835b189a2876385e5a689abfe37f006167b8581751e5b91721a7c340`).

## Verification evidence supplied

- Focused MUFG suite — PASS, 40/40
- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS
- `cargo test --workspace` — PASS
- PAN/member-vault/identity-security/provider-contract invariants — PASS
- Fixture SHA-256 provenance for `issues.json`, `cases.json`, and `session.json` — PASS
- `npm run check` — PASS
- `npm run build` — PASS
- `npm run secrets` — PASS (151 files)
- `git diff --check` — PASS

## Disposition

Gate 4D is APPROVED. The implementation is locked at correction commit `82971e0`; the initial blocker is resolved, and there are no remaining blocking findings. No real PAN, investor lookup, CAPTCHA solution, or CAPTCHA bypass was used. Next approved slice: Gate 4E — cross-provider normalization and UI.
