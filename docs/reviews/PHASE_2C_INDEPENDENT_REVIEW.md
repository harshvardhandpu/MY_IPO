# Phase 2C Independent Review

**Date:** 2026-08-28  
**Reviewed HEAD:** `65d905b`  
**Branch:** `feature/sensitive-identity`  
**Scope:** Phase 2C vertical slice only (onboarding → invest → CHECK/SUBMIT → projection)

## Reviewer

| Field | Value |
|---|---|
| Preferred reviewer | Qwen 3.8 Max Free via TokenRouter, EXTRA HIGH |
| Preferred status | **Unavailable for Hermes tool sessions** — repeated `HTTP 503 cache-only admission rejected` |
| DeepSeek V4 Pro / Flash (NVIDIA NIM) | Unavailable / EOL / unhealthy during gate |
| Independent fallback used | **Gemini 3.6 Flash** (Google free API) |
| Independence | Did not implement Phase 2C |

## Verdict

**PASS WITH NON-BLOCKING FINDINGS**

## Blocking Findings

None.

## High Severity

None.

## Medium Severity

None.

## Low / Non-blocking Findings

1. **Development key derived from device ID** (`apps/desktop/src-tauri/src/service.rs`)  
   Acceptable for synthetic local development only. Device UUID is not a secret.  
   **Release blocker before real-member trial / production:** native Linux Secret Service and Windows Credential Manager `KeyProvider`s.

2. **Onboarding `email` / `broker` accepted then dropped**  
   Intentional Phase 2C scope; domain profiles do not yet persist those fields.

## Security Boundary Assessment (summary)

- PAN/UPI enter only via narrow Tauri commands, validate, encrypt immediately into MemberVault.
- Profiles, events, SQLite, AI DTOs, logs: masked PAN / no plaintext private identity.
- `InvestmentDecisionRequest` / payload fail-closed via `assert_safe()`.
- Security regression tests cover vault/SQLite/event/AI/log non-leak paths.

## Accounting / Event / UI (summary)

- Integer paise + basis points; friend default 10% when eligible; archive-not-delete.
- CHECK ≠ SUBMIT; recommendation applies draft only; no auto-submit.
- Events integrity-sealed; SQLite schema v2 is rebuildable projection.

## Final Recommendation

**APPROVE PHASE 2C AND PROCEED TO REGISTRAR / ALLOTMENT AUTOMATION**

## Evidence

- Master source + `docs/handoffs/CURRENT_STATE.md`
- Commits `b834d89` … `65d905b`
- Domain, identity-security, intelligence-vault, ranking, local-index, member-vault, desktop service/UI
- Security / AI-boundary tests

Phase 2C independent review gate is **CLOSED**.
