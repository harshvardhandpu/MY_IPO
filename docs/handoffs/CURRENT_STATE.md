# Current State

- **Branch:** `feature/sensitive-identity`
- **Milestone base:** annotated tag `phase-2b-green`
- **Phase 2C:** **CLOSED** — independent review PASS WITH NON-BLOCKING FINDINGS (`docs/reviews/PHASE_2C_INDEPENDENT_REVIEW.md`)
- **Phase:** Phase 3A allotment vertical — fixture-first KFintech path implemented
- **Date:** 2026-08-28

## HARD RELEASE BLOCKER

**Before any real-member trial or production release with real PAN:**

| Requirement | Status |
|---|---|
| Linux Secret Service / native keyring `KeyProvider` | **BLOCKER — not started** |
| Windows Credential Manager `KeyProvider` | **BLOCKER — not started** |
| Device-ID-derived dev key | Allowed for **synthetic local development only** |
| Real PAN with dev key scheme | **FORBIDDEN** |

## Phase 3 implemented (this milestone)

### Research

- `docs/research/registrars/KFINTECH.md` — official status URL `https://ipostatus.kfintech.com`; no public stable API found; fixture-first
- `docs/research/registrars/BIGSHARE.md` — stub notes; multi-server UI
- `docs/research/registrars/MUFG_INTIME.md` — captcha observed on public issues page → human verification path

### Domain / provider (`crates/allotment`)

- `NormalizedAllotmentStatus` with fail-closed unknown (never auto-`NOT_ALLOTTED`)
- `AllotmentProvider` trait + `FixtureKfintechProvider` (deterministic synthetic routing)
- Job/attempt state machines (`CREATED`→`RUNNING`→`COMPLETE` / partial)
- Manual result provenance types

### Events + SQLite schema v3

- `ALLOTMENT_JOB_CREATED`, `ALLOTMENT_JOB_STATUS_CHANGED`, `ALLOTMENT_ATTEMPT_RECORDED`
- Tables `allotment_jobs`, `allotment_attempts` (account IDs + masked display only; **no PAN columns**)

### Application / UI

- Commands: `list_allotment_candidates`, `start_allotment_check`, `get_allotment_report`
- PAN access only via `SensitiveIdentityService::with_pan(..., AllotmentCheck, ...)`
- Dashboard **Check Allotment** enabled → Allotment page → report card (masked PAN)

### Security

- `apps/desktop/src-tauri/tests/allotment_security.rs` — after fixture check, plaintext PAN absent from report JSON and vault tree

## Still deferred / next exact tasks

1. Live KFintech HTTP/browser spike behind the same trait (no Playwright in React).
2. Bigshare / MUFG Intime adapters (MUFG defaults to `NEEDS_HUMAN_VERIFICATION` when captcha present).
3. Background worker process for long jobs + restart recovery UX polish (events already durable).
4. Per-provider rate-limit config + retry scheduler.
5. Manual result entry UI + open-official-URL action.
6. Estimated profit only with explicit price basis (not fabricated).
7. OS keyring providers (release blocker).
8. Independent Phase 3 review when live/provider path freezes.

## Out of scope (unchanged)

Strategy Lab, full News, production ranking algorithm, mobile, Supabase, SaaS, broker execution.

## Verification (latest)

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (allotment + prior suites green; allotment security test green)
- `npm run check` (secret scan 110 files; Biome; tsc; 4 Vitest; 4 Python scanner)
- `npm run build`

## Commands

```bash
npm install
npm run check
npm run build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run tauri:dev
```
