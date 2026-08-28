# Current State

- **Branch:** `feature/sensitive-identity`
- **Milestone base:** annotated tag `phase-2b-green`
- **Phase 2C:** **CLOSED** — independent review PASS WITH NON-BLOCKING FINDINGS (`docs/reviews/PHASE_2C_INDEPENDENT_REVIEW.md`)
- **Phase:** Phase 3 — registrar / allotment automation (in progress)
- **Verified HEAD at Phase 2C close:** `65d905b`
- **Date:** 2026-08-28

## Phase 2C closed summary

Onboarding → encrypted identity → Invest → CHECK (sanitized DTO + dev ranking) → SUBMIT → events → SQLite v2 projection → dashboard.

Independent review (Gemini 3.6 Flash free fallback; Qwen TokenRouter tool sessions 503; DeepSeek NIM unavailable):

- **Verdict:** PASS WITH NON-BLOCKING FINDINGS
- **Blocking / High / Medium:** none
- **Non-blocking:**
  1. Device-ID-derived **development key only** — not production key storage
  2. `email` / `broker` onboarding fields deferred (not persisted yet)

## HARD RELEASE BLOCKER (do not forget)

**Before any real-member trial or production release with real PAN:**

| Requirement | Status |
|---|---|
| Linux Secret Service / native keyring `KeyProvider` | **BLOCKER — not started** |
| Windows Credential Manager `KeyProvider` | **BLOCKER — not started** |
| Device-ID-derived dev key | Allowed for **synthetic local development only** |
| Real PAN with dev key scheme | **FORBIDDEN** |

## Implemented workflow (Phase 2C)

### Onboarding and private identity

- First-run React onboarding captures owner name, email, primary account, broker, UPI, PAN, and explicit local-storage consent.
- Rust validates PAN/UPI at a narrow Tauri command boundary and immediately encrypts `IdentitySecret` with XChaCha20-Poly1305.
- Member/friend profile JSON and UI responses contain masked PAN only (`ABCDE****F`).
- Friend accounts support 10% default profit-share eligibility and archive-not-delete behavior.
- Captured `email` and `broker` are currently acknowledged by the command but not persisted because the domain profile does not yet contain those fields.
- Purpose-scoped access already exists: `SensitiveIdentityService::with_pan(..., SensitivePurpose::AllotmentCheck, ...)`. There is **no** general `get_pan() -> String`.

### Investment workflow

- `InvestmentSession`, `IpoApplication`, and `InvestmentAllocation` use UUIDv7 IDs.
- Money is integer paise; percentages are integer basis points. No financial float is stored in Rust.
- CHECK constructs the approved multi-IPO DTO only (no PAN/UPI/names in AI path).
- `DevRankingAlgorithm` is deterministic scaffolding labeled **DEVELOPMENT ALGORITHM — NOT INVESTMENT ADVICE**.
- SUBMIT seals immutable events, appends them to MemberVault, and applies SQLite projections.

### Projection-driven dashboard

SQLite schema version 2 projects members, friends, IPOs, sessions, applications, allocations, recommendations. Rebuild-from-events is supported. SQLite is derived state only.

## Command surface

- `get_app_status`, `onboard_member`, `add_friend`, `archive_friend`, `list_members`, `list_friends`, `check_recommendation`, `submit_investment`, `get_dashboard`
- Check Allotment UI remains the Phase 3 entry point (was disabled; now to be wired).

## Security invariants proven (must stay green)

1. Full PAN/UPI encrypted before persistence.
2. Plaintext identity never in profiles, events, SQLite, AI payloads, errors, or logs.
3. PAN type is non-serializable; Display/Debug redacted.
4. AI boundary fail-closed.
5. SQLite has `masked_pan` only.
6. Archive-not-delete for friends.
7. Purpose-scoped `with_pan` for allotment only (Phase 3 must keep this).
8. Repo secret scanner fails closed outside fixtures.

## Verification at Phase 2C close

- 116 Rust tests; npm check/build; secret scan 99 files; Linux Tauri smoke passed.

## Phase 2C commits

```text
b834d89 feat(domain): investment sessions, applications, allocations and events
4e0ce96 feat(ranking): versioned algorithm interface, deterministic dev algo, multi-IPO request
704ae2e feat(index): project investment events into SQLite (schema v2)
fdc6795 feat(desktop): secure onboarding, recommendation, submit, and dashboard commands
7f9f7e6 feat(ui): add private onboarding and investment workflows
66f322a test(security): prove private identity never crosses storage or AI boundaries
65d905b docs(handoff): record verified Phase 2C investment workflow
```

## Phase 3 objective (current)

Registrar/allotment vertical:

Submitted IPO → registrar discovery → Check Allotment → durable `AllotmentCheckJob` → purpose-scoped PAN → provider → normalized status → report card → dashboard (no LLM on PAN path).

### Next exact tasks

1. Research KFintech / Bigshare / MUFG Intime official status flows → `docs/research/registrars/`.
2. Domain: provider contract, job/attempt/result, normalized statuses (unknown ≠ NOT_ALLOTTED).
3. Durable events + SQLite projection for jobs.
4. Fixture-first KFintech adapter; browser worker only if required.
5. Background sequential execution + UI + report card + manual fallback.
6. PAN regression after synthetic allotment check.
7. Independent review freeze when green.

### Out of scope this phase

Strategy Lab, full News, production ranking algorithm, mobile, Supabase, SaaS, broker execution.

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
