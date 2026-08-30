# Gate 4F Condition A TLS Amendment — Independent Review

- **Date:** 2026-08-30
- **Scope:** Commit `6b8ff6a` (`fix(allotment): enable rustls HTTPS transport`)
- **Reviewer:** `gpt-oss-120b` via generalcompute (owner-locked, free, independent route)
- **Mode:** READ-ONLY evidence-pack review; no repository edits and no PAN
- **Provenance:** HTTP 200; served model `gpt-oss-120b`; prompt tokens 1759; completion tokens 232; reasoning tokens 62; finish reason `stop`

## Why an amendment was required

The original Condition A correction selected ureq's internal `_tls` feature.
Static compilation and fixture tests passed, but an identifier-free live HTTPS
precheck produced a runtime panic before transport:

`uri scheme is https, provider is Rustls but feature is not enabled: rustls`

Commit `6b8ff6a` selects ureq's public `rustls` feature with `default-features =
false`. The resolved feature tree includes `webpki-roots`, `rustls-webpki`,
`ring`, and the rustls standard/TLS 1.2 features. It also adds a local-only
regression proving an HTTPS URI returns a normal connection error rather than
panicking when no server is listening.

## Verdict

**PASS — BLOCKERS: NONE**

| Check | Result |
|---|---|
| TLS feature | PASS — public `rustls` activates HTTPS with webpki roots |
| Regression test | PASS — reproduces the prior failure boundary without external network |
| Lockfile | PASS — expected rustls, rustls-webpki, webpki-roots, ring, and untrusted dependencies |
| Policy regression | PASS — no allowlist, redirect, size, timeout, or redaction control changed |

## Verification evidence

After the amendment:

- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS
- `cargo test --workspace` — PASS, including `rustls_https_transport_is_enabled`
- `npm run check` — PASS
- `npm run build` — PASS
- `npm run secrets` — PASS (157 files)
- `git diff --check` — PASS

## Disposition

This review supersedes only the original review's unsupported claim that
Condition A already had active rustls transport. Conditions B–E remain covered
by `GATE_4F_CORRECTIONS_INDEPENDENT_REVIEW.md`.
