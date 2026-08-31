# Session Void Correction — Independent Review

- **Reviewer:** `gpt-oss-120b`
- **Provider:** generalcompute (free tier; no paid usage) — owner-locked reviewer route
- **Review date:** 2026-08-31
- **Scope:** Append-only owner void of submitted investment sessions (`InvestmentSessionVoided`), projection exclusion from active totals/allotment candidates, Tauri command + dashboard confirmation UI
- **Mode:** independent, read-only, fail-closed review
- **Implementer:** separate model route — reviewer is independent
- **Policy:** Gemini barred. No real PAN access, no registrar lookup, no event rewrite/delete.

## Verdict

**PASS — APPROVE** (`approve: true`, zero blocking findings)

## What the reviewer confirmed

1. Void is append-only via `InvestmentSessionVoided`; original events remain.
2. Domain transition only Submitted → Voided.
3. Projection sets `VOIDED`; `list_submitted_applications` (SUBMITTED-only) drops voided apps from allotment eligibility.
4. Dashboard active totals count SUBMITTED sessions only.
5. Service requires owner affirmation, actor match, OWNER role, and PAN-free reason validation.
6. No PAN access path and no provider networking on void.
7. UI requires explicit owner confirmation before void.

## Non-blocking findings (3)

1. `aggregate_revision: epoch_secs().max(3)` is coarse; fine for single-device local ledger.
2. Best-effort job cancel is projection-side (matches existing cancel pattern); no separate cancel event.
3. Void reason free-text is bounded and PAN-scanned; keep UI copy clear.

## Verification evidence

- `cargo fmt --all` PASS
- `cargo clippy --workspace --all-targets -- -D warnings` PASS
- `cargo test --workspace` PASS (includes new domain + projection void tests)
- `npm run typecheck` PASS
- vitest App.test + App.flows 8/8 PASS
- biome check PASS
- `npm run secrets` PASS (176 files)
- `npm run build` PASS

## Disposition

Correction capability is **APPROVED** for owner use. Do **not** auto-void production data or create the historical Symbiotec record — owner performs void via native UI, then historical entry.
