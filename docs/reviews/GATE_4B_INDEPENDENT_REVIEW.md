# Gate 4B Independent Review

- **Reviewer:** gpt-oss-120b
- **Provider:** generalcompute (free tier; no paid usage)
- **Review date:** 2026-08-29
- **Scope:** Gate 4B KFintech live-adapter replacement, git range `5f3710c..b281379` on `feature/multi-registrar` — bounded fixture-driven parser, guarded affirmative/pending result constructors, sanitized fixtures with pinned SHA-256 provenance, 13 focused tests
- **Mode:** independent, read-only, fail-closed review
- **Review infrastructure note:** the verdict JSON echoed `minimax-m2.7` because the prompt header (written for an earlier rejected route) pinned that literal model name; the request was actually served by `gpt-oss-120b` via generalcompute, as configured in the runner and verified from the request/response metadata. This affects no review content.

## Verdict

**PASS WITH NON-BLOCKING FINDINGS**

**Recommendation:** APPROVE (`approve: true`, zero blocking findings)

## Findings

### Non-blocking

1. **`ProviderHealth::Degraded` is returned when `network_enabled` is true, even though the provider is otherwise functional** (`crates/allotment/src/kfintech.rs:210-218`). This is a semantics/style issue and does not affect security invariants.

   *Disposition: accepted, deferred.* `Degraded` is deliberately honest here: the live transport (`check_allotment` request construction) is a later-slice stub, so the provider genuinely cannot fully serve requests while `network_enabled` is true. Reporting `Available` would overstate capability. Transport wiring is tracked as explicit next-slice work in `docs/plans/multi-registrar-allotment/04-slices.md` (Gate 4B item 10). No security invariant is weakened: `check_allotment` fails closed to `Unknown`/`NeedsHuman` and the parser never fabricates `NOT_ALLOTTED`.

## Reviewer route record

Independent review must use a model distinct from implementers (GLM 5.3 — active chat model, DeepSeek V4 Pro — 4B implementer) and is bound by owner policy: **Gemini permanently disabled for all Sanket IPO roles**, zero paid usage. Route probing on 2026-08-29:

| Route | Result |
|---|---|
| TokenRouter `z-ai/glm-5.3-free` | Healthy but excluded — active chat model, non-independent |
| TokenRouter Qwen 3.8 Max | HTTP 403 — no access on this token |
| inference.net Kimi K3 | HTTP 402 — paid wall, excluded |
| inference.net Gemini 3.6 Flash | Barred by owner policy |
| generalcompute MiniMax M2.7 | HTTP 200 healthy; degenerate token-soup output on the evidence pack — verdict rejected as invalid |
| generalcompute gpt-oss-120b | HTTP 200 healthy, coherent (17×23=391 check) — **selected** |
| Groq `openai/gpt-oss-120b` | HTTP 200 healthy, coherent — but free-tier 8,000 TPM cap rejects the 10,455-token evidence pack (HTTP 413), so unusable for this review |
| omni localhost proxy | HTTP 000 — down |
| inferex deepseek-v4-flash | Rate-limited |

gpt-oss-120b via generalcompute served the full 23 KB evidence pack and returned a structured verdict in JSON on the required schema with an explicit verdict, findings by severity, and approve flag.

## What the reviewer confirmed

Per the prompt contract (fail-closed parser semantics, guarded constructors, PAN-free construction, fixture provenance verification, no weaker public result path), the verdict reports zero blocking findings and one non-blocking finding.

## Verification evidence supplied

- `cargo test -p sanket-allotment` — 30 tests PASS
- `cargo test --workspace` — PASS
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS
- `cargo fmt --all -- --check` — PASS
- `npm run check` (biome, typecheck, vitest 4, python 4) — PASS on host Node 26
- `npm run secrets` — 135 files PASS
- Implementation commit: `b281379` (already independently verified and committed before review)

## Disposition

Gate 4B is APPROVED. Non-blocking finding deferred to the transport-wiring slice. Real PAN remains blocked pending separately authorized controlled pilot.
